use std::collections::HashMap;

use crate::database::entities::task::TaskStatus;
use crate::{
    database::entities::{Lock, Task},
    errors::SchedulerError,
};

type Constraints<'a> = HashMap<&'a str, &'a Lock>;

trait ConstraintMgmt<'a> {
    fn insert_lock(&mut self, lock: &'a Lock) -> Result<(), SchedulerError>;
    fn can_get_lock(&self, lock: &Lock) -> bool;
}

impl<'a> ConstraintMgmt<'a> for Constraints<'a> {
    fn insert_lock(&mut self, lock: &'a Lock) -> Result<(), SchedulerError> {
        if !self.can_get_lock(lock) {
            return Err(SchedulerError::LockAlreadyOwnedError);
        }
        self.insert(lock.name(), lock);

        Ok(())
    }

    fn can_get_lock(&self, lock: &Lock) -> bool {
        if !self.contains_key(lock.name()) {
            return true; // lock wasn't used yet
        }
        let lock = self.get(lock.name()).expect("Lock must be in list");

        !lock.is_conflicting(lock)
    }
}

pub struct Scheduler<'a> {
    constraints: Constraints<'a>,
    tasks: &'a Vec<Task>,
    poisoned_locks: &'a [Lock],
    machine_features: &'a [String],
}

impl<'a> Scheduler<'a> {
    pub fn new(
        tasks: &'a Vec<Task>,
        poisoned_locks: &'a [Lock],
        machine_features: &'a [String],
    ) -> Result<Self, SchedulerError> {
        let constraints: Constraints = Constraints::new();

        let mut s = Scheduler {
            constraints,
            tasks,
            poisoned_locks,
            machine_features,
        };

        for task in s.tasks {
            if task.status != TaskStatus::Running {
                continue;
            }

            for lock in &task.locks {
                s.constraints.insert_lock(lock)?;
            }
        }

        Ok(s)
    }

    fn is_poisoned(&self, lock: &Lock) -> bool {
        self.poisoned_locks
            .iter()
            .any(|plock| lock.is_conflicting(plock))
    }

    fn is_unconstrained(&self, task: &Task) -> bool {
        task.locks
            .iter()
            .all(|lock| self.constraints.can_get_lock(lock) && !self.is_poisoned(lock))
    }

    fn supports_all_features(&self, task: &Task) -> bool {
        task.features
            .iter()
            .all(|feat| self.machine_features.contains(feat))
    }

    fn should_pick_task(&self, task: &Task) -> bool {
        task.status == TaskStatus::New
            && self.supports_all_features(task)
            && self.is_unconstrained(task)
    }

    pub fn get_next_task(self) -> Option<Task> {
        self.tasks
            .iter()
            .find(|task| self.should_pick_task(task))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::database::entities::task::TaskStatus;
    use crate::database::entities::{Lock, Task};

    use super::Scheduler;

    #[test]
    fn scheduler_creation_no_constraints() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::Running)
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::Running)
                .build(),
        ];

        Scheduler::new(&tasks, &[], &[]).unwrap();
    }

    #[test]
    fn scheduler_creation_multiple_read_constraints() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::Running)
                .with_read_lock("foo 1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::Running)
                .with_read_lock("foo 1")
                .build(),
        ];

        Scheduler::new(&tasks, &[], &[]).unwrap();
    }

    #[test]
    fn scheduler_creation_single_write_constraints() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::Running)
                .with_write_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::Running)
                .with_write_lock("foo2")
                .build(),
        ];

        Scheduler::new(&tasks, &[], &[]).unwrap();
    }

    #[test]
    fn scheduler_creation_multiple_write_constraints() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::Running)
                .with_write_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::Running)
                .with_write_lock("foo1")
                .build(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]);
        assert!(res.is_err());
    }

    #[test]
    fn scheduler_no_new_task() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::Running)
                .with_write_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::New)
                .with_write_lock("foo1")
                .build(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task(), None)
    }

    #[test]
    fn scheduler_no_new_task_with_feature() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::New)
                .requires_feature("huge_cpu")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::New)
                .requires_feature("huge_cpu")
                .build(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 and Test 2 have required features, which our runner does not have.
        assert_eq!(res.get_next_task(), None)
    }

    #[test]
    fn scheduler_new_task_with_specific_feature() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::New)
                .requires_feature("huge_cpu")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::New)
                .requires_feature("huge_cpu")
                .build(),
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
                .with_display_name("Test 1")
                .with_status(TaskStatus::Running)
                .with_write_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::New)
                .with_write_lock("foo2")
                .build(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task().unwrap().display_name, "Test 2")
    }

    #[test]
    fn scheduler_new_task_ro() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::Running)
                .with_read_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::New)
                .with_read_lock("foo1")
                .build(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task().unwrap().display_name, "Test 2")
    }

    #[test]
    fn scheduler_new_task_rw_ro() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::Running)
                .with_write_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::New)
                .with_read_lock("foo1")
                .build(),
        ];

        let res = Scheduler::new(&tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task(), None)
    }

    #[test]
    fn schedule_with_poisoned_lock() {
        let tasks = vec![Task::builder()
            .with_display_name("I need to do something")
            .with_write_lock("Entire Prod Cluster")
            .build()];
        let poisoned_locks = vec![Lock::Write {
            name: "Entire Prod Cluster".to_string(),
            poisoned: Some(Uuid::new_v4()),
        }];

        let res = Scheduler::new(&tasks, &poisoned_locks, &[]).unwrap();

        assert_eq!(res.get_next_task(), None);
    }

    #[test]
    fn schedule_different_tasks_with_poisoned_lock() {
        let tasks = vec![
            Task::builder()
                .with_display_name("I need to do something")
                .with_write_lock("Entire Prod Cluster")
                .build(),
            Task::builder()
                .with_display_name("I need to test something")
                .with_write_lock("Entire Staging Cluster")
                .build(),
        ];
        let poisoned_locks = vec![Lock::Write {
            name: "Entire Prod Cluster".to_string(),
            poisoned: Some(Uuid::new_v4()),
        }];

        let res = Scheduler::new(&tasks, &poisoned_locks, &[]).unwrap();

        assert_eq!(
            res.get_next_task().unwrap().display_name,
            "I need to test something"
        );
    }

    #[test]
    fn schedule_different_tasks_with_poisoned_lock_ro() {
        let tasks = vec![Task::builder()
            .with_display_name("I need to do something")
            .with_read_lock("Entire Prod Cluster")
            .build()];
        let poisoned_locks = vec![Lock::Read {
            name: "Entire Prod Cluster".to_string(),
            poisoned: Some(Uuid::new_v4()),
        }];

        let res = Scheduler::new(&tasks, &poisoned_locks, &[]).unwrap();

        assert_eq!(res.get_next_task(), None);
    }
}
