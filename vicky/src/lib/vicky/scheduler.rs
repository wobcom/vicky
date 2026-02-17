use crate::database::entities::task::TaskStatus;
use crate::vicky::constraints::{ConstraintEvaluation, ConstraintFail, Constraints};
use crate::{
    database::entities::{Lock, Task},
    errors::SchedulerError,
};

pub struct Scheduler<'a> {
    constraints: Constraints<'a>,
    tasks: &'a Vec<Task>,
    machine_features: &'a [String],
}

impl<'a> Scheduler<'a> {
    pub fn new(
        tasks: &'a Vec<Task>,
        poisoned_locks: &'a [Lock],
        machine_features: &'a [String],
    ) -> Result<Self, SchedulerError> {
        let constraints: Constraints = Constraints::from_tasks(tasks, poisoned_locks)?;

        let s = Scheduler {
            constraints,
            tasks,
            machine_features,
        };

        #[cfg(test)]
        s.print_debug_evaluation();

        Ok(s)
    }

    fn is_unconstrained(&'a self, task: &Task) -> Option<ConstraintFail<'a>> {
        task.locks
            .iter()
            .find_map(|lock| self.constraints.try_acquire(lock))
    }

    fn find_unsupported_features(&self, task: &Task) -> Option<String> {
        task.features
            .iter()
            .find(|feat| !self.machine_features.contains(feat))
            .cloned()
    }

    fn evaluate_task_readiness(&'a self, task: &Task) -> ConstraintEvaluation<'a> {
        if task.status != TaskStatus::New {
            return ConstraintEvaluation::NotReady;
        }

        if let Some(feature) = self.find_unsupported_features(task) {
            return ConstraintEvaluation::missing_feature(feature);
        }

        if let Some(constraint) = self.is_unconstrained(task) {
            return ConstraintEvaluation::Constrained(constraint);
        }

        ConstraintEvaluation::Ready
    }

    pub fn get_next_task(self) -> Option<Task> {
        self.tasks
            .iter()
            .find(|task| self.evaluate_task_readiness(task).is_ready())
            .cloned()
    }

    #[allow(unused)]
    pub fn print_debug_evaluation(&self) {
        for task in self.tasks {
            let eval = self.evaluate_task_readiness(task);
            println!("Readiness of {} ({}): {eval:?}", task.id, task.display_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::Scheduler;
    use crate::database::entities::task::{TaskResult, TaskStatus};
    use crate::database::entities::{Lock, Task};

    #[test]
    fn scheduler_creation_no_constraints() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::Running)
                .build_expect(),
            Task::builder()
                .display_name("Test 2")
                .status(TaskStatus::Running)
                .build_expect(),
        ];

        Scheduler::new(&tasks, &[], &[]).unwrap();
    }

    #[test]
    fn scheduler_creation_multiple_read_constraints() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::Running)
                .read_lock("foo 1")
                .build_expect(),
            Task::builder()
                .display_name("Test 2")
                .status(TaskStatus::Running)
                .read_lock("foo 1")
                .build_expect(),
        ];

        Scheduler::new(&tasks, &[], &[]).unwrap();
    }

    #[test]
    fn scheduler_creation_single_write_constraints() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::Running)
                .write_lock("foo1")
                .build_expect(),
            Task::builder()
                .display_name("Test 2")
                .status(TaskStatus::Running)
                .write_lock("foo2")
                .build_expect(),
        ];

        Scheduler::new(&tasks, &[], &[]).unwrap();
    }

    #[test]
    fn scheduler_creation_read_and_cleanup_constraints_is_order_independent() {
        let mut tasks_read_then_clean = vec![
            Task::builder()
                .display_name("Read lock")
                .status(TaskStatus::Running)
                .read_lock("shared")
                .build_expect(),
            Task::builder()
                .display_name("Cleanup lock")
                .status(TaskStatus::New)
                .clean_lock("shared")
                .build_expect(),
        ];

        Scheduler::new(&tasks_read_then_clean, &[], &[])
            .expect("read->cleanup lock order must not fail scheduler creation");

        tasks_read_then_clean.reverse();
        let tasks_clean_then_read = tasks_read_then_clean;

        Scheduler::new(&tasks_clean_then_read, &[], &[])
            .expect("cleanup->read lock order must not fail scheduler creation");
    }

    #[test]
    fn scheduler_creation_multiple_write_constraints() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::Running)
                .write_lock("foo1")
                .build_expect(),
            Task::builder()
                .display_name("Test 2")
                .status(TaskStatus::Running)
                .write_lock("foo1")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]);
        assert!(res.is_err());
    }

    #[test]
    fn scheduler_no_new_task() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::Running)
                .write_lock("foo1")
                .build_expect(),
            Task::builder()
                .display_name("Test 2")
                .status(TaskStatus::New)
                .write_lock("foo1")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task(), None)
    }

    #[test]
    fn scheduler_no_new_task_with_feature() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::New)
                .requires_feature("huge_cpu")
                .build_expect(),
            Task::builder()
                .display_name("Test 2")
                .status(TaskStatus::New)
                .requires_feature("huge_cpu")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 and Test 2 have required features, which our runner does not have.
        assert_eq!(res.get_next_task(), None)
    }

    #[test]
    fn scheduler_new_task_with_specific_feature() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::New)
                .requires_feature("huge_cpu")
                .build_expect(),
            Task::builder()
                .display_name("Test 2")
                .status(TaskStatus::New)
                .requires_feature("huge_cpu")
                .build_expect(),
        ];

        let features = &["huge_cpu".to_string()];
        let res = Scheduler::new(&tasks, &[], features).unwrap();
        // Test 1 and Test 2 have required features, which our runner matches.
        assert_eq!(res.get_next_task().unwrap().display_name, "Test 1")
    }

    #[test]
    fn scheduler_new_task() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::Running)
                .write_lock("foo1")
                .build_expect(),
            Task::builder()
                .display_name("Test 2")
                .status(TaskStatus::New)
                .write_lock("foo2")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task().unwrap().display_name, "Test 2")
    }

    #[test]
    fn scheduler_new_task_ro() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::Running)
                .read_lock("foo1")
                .build_expect(),
            Task::builder()
                .display_name("Test 2")
                .status(TaskStatus::New)
                .read_lock("foo1")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task().unwrap().display_name, "Test 2")
    }

    #[test]
    fn scheduler_new_task_rw_ro() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::Running)
                .write_lock("foo1")
                .build_expect(),
            Task::builder()
                .display_name("Test 2")
                .status(TaskStatus::New)
                .read_lock("foo1")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task(), None)
    }

    #[test]
    fn scheduler_new_task_cleanup_single() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::New)
                .clean_lock("foo1")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task().unwrap().display_name, "Test 1")
    }

    #[test]
    fn scheduler_new_task_cleanup_with_finished() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 5")
                .status(TaskStatus::Finished(TaskResult::Success))
                .write_lock("foo1")
                .build_expect(),
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::New)
                .clean_lock("foo1")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task().unwrap().display_name, "Test 1")
    }

    #[test]
    fn scheduler_cleanup_waits_for_running_task_using_same_lock() {
        let tasks = vec![
            Task::builder()
                .display_name("Im doing something")
                .status(TaskStatus::Running)
                .read_lock("foo1")
                .build_expect(),
            Task::builder()
                .display_name("Cleanup after")
                .status(TaskStatus::New)
                .clean_lock("foo1")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        let eval = res.evaluate_task_readiness(&res.tasks[1]);

        assert!(
            !eval.is_ready(),
            "Expected cleanup to wait while same lock is currently in use. Got {eval:?}"
        );
        assert_eq!(res.get_next_task(), None);
    }

    #[test]
    fn scheduler_new_task_cleanup() {
        let tasks = vec![
            Task::builder()
                .display_name("Test 1")
                .status(TaskStatus::New)
                .clean_lock("foo1")
                .build_expect(),
            Task::builder()
                .display_name("Test 2")
                .status(TaskStatus::New)
                .read_lock("foo1")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        let eval = res.evaluate_task_readiness(&res.tasks[0]);
        assert!(
            eval.is_passive_collision(),
            "Expected evaluation to actively collide. But received {eval:?}"
        );
        let eval = res.evaluate_task_readiness(&res.tasks[1]);
        assert!(
            eval.is_ready(),
            "Expected evaluation to succeed. But received {eval:?}"
        );

        // Run tasks before cleanup
        assert_eq!(res.get_next_task().unwrap().display_name, "Test 2")
    }

    #[test]
    fn scheduler_new_task_cleanup_unrelated_pending_lock() {
        let tasks = vec![
            Task::builder()
                .display_name("Cleanup lock A")
                .status(TaskStatus::New)
                .clean_lock("lock_a")
                .build_expect(),
            Task::builder()
                .display_name("Pending lock B")
                .status(TaskStatus::New)
                .read_lock("lock_b")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();

        assert_eq!(res.get_next_task().unwrap().display_name, "Cleanup lock A")
    }

    #[test]
    fn scheduler_needs_validation_locks_block_conflicts_only() {
        let tasks = vec![
            Task::builder()
                .display_name("Task A")
                .status(TaskStatus::NeedsUserValidation)
                .write_lock("lock_a")
                .read_lock("lock_b")
                .build_expect(),
            Task::builder()
                .display_name("Task A2")
                .status(TaskStatus::New)
                .read_lock("lock_a")
                .build_expect(),
            Task::builder()
                .display_name("Task B")
                .status(TaskStatus::New)
                .read_lock("lock_b")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();

        assert_eq!(res.get_next_task().unwrap().display_name, "Task B");
    }

    #[test]
    fn scheduler_needs_validation_keeps_strongest_lock_when_names_collide() {
        let tasks = vec![
            Task::builder()
                .display_name("Validation writer")
                .status(TaskStatus::NeedsUserValidation)
                .write_lock("shared_lock")
                .build_expect(),
            Task::builder()
                .display_name("Validation reader")
                .status(TaskStatus::NeedsUserValidation)
                .read_lock("shared_lock")
                .build_expect(),
            Task::builder()
                .display_name("New reader")
                .status(TaskStatus::New)
                .read_lock("shared_lock")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        let eval = res.evaluate_task_readiness(&res.tasks[2]);
        assert!(
            eval.is_passive_collision(),
            "Expected passive collision from validation writer, got {eval:?}"
        );
        assert_eq!(res.get_next_task(), None);
    }

    #[test]
    fn scheduler_cleanup_waits_for_non_cleanup_even_with_later_cleanup() {
        let tasks = vec![
            Task::builder()
                .display_name("Cleanup 1")
                .status(TaskStatus::New)
                .clean_lock("shared_lock")
                .build_expect(),
            Task::builder()
                .display_name("Reader")
                .status(TaskStatus::New)
                .read_lock("shared_lock")
                .build_expect(),
            Task::builder()
                .display_name("Cleanup 2")
                .status(TaskStatus::New)
                .clean_lock("shared_lock")
                .build_expect(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        let eval = res.evaluate_task_readiness(&res.tasks[0]);
        assert!(
            eval.is_passive_collision(),
            "Expected cleanup to wait for pending non-clean lock, got {eval:?}"
        );
        assert_eq!(res.get_next_task().unwrap().display_name, "Reader");
    }

    #[test]
    fn schedule_with_poisoned_lock() {
        let tasks = vec![
            Task::builder()
                .display_name("I need to do something")
                .write_lock("Entire Prod Cluster")
                .build_expect(),
        ];
        let mut poisoned_lock = Lock::write("Entire Prod Cluster");
        poisoned_lock.poison(&Uuid::new_v4());
        let poisoned_locks = vec![poisoned_lock];

        let res = Scheduler::new(&tasks, &poisoned_locks, &[]).unwrap();

        assert_eq!(res.get_next_task(), None);
    }

    #[test]
    fn schedule_different_tasks_with_poisoned_lock() {
        let tasks = vec![
            Task::builder()
                .display_name("I need to do something")
                .write_lock("Entire Prod Cluster")
                .build_expect(),
            Task::builder()
                .display_name("I need to test something")
                .write_lock("Entire Staging Cluster")
                .build_expect(),
        ];
        let mut poisoned_lock = Lock::write("Entire Prod Cluster");
        poisoned_lock.poison(&Uuid::new_v4());
        let poisoned_locks = vec![poisoned_lock];

        let res = Scheduler::new(&tasks, &poisoned_locks, &[]).unwrap();

        assert_eq!(
            res.get_next_task().unwrap().display_name,
            "I need to test something"
        );
    }

    #[test]
    fn schedule_different_tasks_with_poisoned_lock_ro() {
        let tasks = vec![
            Task::builder()
                .display_name("I need to do something")
                .read_lock("Entire Prod Cluster")
                .build_expect(),
        ];
        let mut poisoned_lock = Lock::read("Entire Prod Cluster");
        poisoned_lock.poison(&Uuid::new_v4());
        let poisoned_locks = vec![poisoned_lock];

        let res = Scheduler::new(&tasks, &poisoned_locks, &[]).unwrap();

        assert_eq!(res.get_next_task(), None);
    }
}
