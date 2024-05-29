use std::collections::HashMap;

use log::debug;

use crate::{
    database::entities::{Lock, Task},
    errors::SchedulerError,
};
use crate::database::entities::task::TaskStatus;

type Constraints = HashMap<String, LockSum>;

trait ConstraintMgmt {
    fn get_map_key(lock: &Lock) -> &String;
    fn get_mut_lock_sum(&mut self, lock: &Lock) -> Option<&mut LockSum>;
    fn get_lock_sum(&self, lock: &Lock) -> Option<&LockSum>;
    fn insert_lock(&mut self, lock: &Lock) -> Result<(), SchedulerError>;
}

impl ConstraintMgmt for Constraints {
    fn get_map_key(lock: &Lock) -> &String {
        match lock {
            Lock::WRITE { name: object, .. } => object,
            Lock::READ { name: object, .. } => object,
        }
    }

    fn get_mut_lock_sum(&mut self, lock: &Lock) -> Option<&mut LockSum> {
        let object = Constraints::get_map_key(lock);
        self.get_mut(object)
    }

    fn get_lock_sum(&self, lock: &Lock) -> Option<&LockSum> {
        let object = Constraints::get_map_key(lock);
        self.get(object)
    }

    fn insert_lock(&mut self, lock: &Lock) -> Result<(), SchedulerError> {
        match self.get_mut_lock_sum(lock) {
            Some(c) => {
                debug!("Found existing LockSum {:?}", lock);
                c.add_lock(lock)?;
            }
            None => {
                debug!("Found no LockSum");
                let object = Constraints::get_map_key(lock);
                self.insert(object.clone(), LockSum::from_lock(lock));
            }
        }

        Ok(())
    }
}

struct LockSum {
    lock: Lock,
    count: i32,
}

impl LockSum {
    pub fn from_lock(lock: &Lock) -> LockSum {
        LockSum {
            lock: lock.clone(),
            count: 1,
        }
    }

    pub fn can_add_lock(&self, lock: &Lock) -> bool {
        match (&self.lock, lock) {
            (Lock::WRITE { .. }, Lock::WRITE { .. }) => false,
            (Lock::WRITE { .. }, Lock::READ { .. }) => false,
            (Lock::READ { .. }, Lock::WRITE { .. }) => false,
            (Lock::READ { name: object, .. }, Lock::READ { name: object2, .. }) => {
                object == object2
            }
        }
    }

    pub fn add_lock(&mut self, lock: &Lock) -> Result<(), SchedulerError> {
        let can_add_lock = self.can_add_lock(lock);
        if !can_add_lock {
            return Err(SchedulerError::GeneralSchedulingError);
        }

        match lock {
            Lock::READ { .. } => {
                self.count += 1;
                Ok(())
            }
            _ => unreachable!(),
        }
    }
}

pub struct Scheduler<'a> {
    constraints: Constraints,
    tasks: Vec<Task>,
    poisoned_locks: &'a [Lock],
    machine_features: &'a [String],
}

impl<'a> Scheduler<'a> {
    pub fn new(
        tasks: Vec<Task>,
        poisoned_locks: &'a [Lock],
        machine_features: &'a [String],
    ) -> Result<Self, SchedulerError> {
        let mut constraints: Constraints = HashMap::new();

        for task in &tasks {
            if task.status != TaskStatus::RUNNING {
                continue;
            }

            for lock in &task.locks {
                constraints.insert_lock(lock)?;
            }
        }

        let s = Scheduler {
            constraints,
            tasks,
            poisoned_locks,
            machine_features,
        };

        Ok(s)
    }

    fn is_unconstrained(&self, task: &Task) -> bool {
        task.locks.iter().all(|lock| {
            self.constraints
                .get_lock_sum(lock)
                .map_or(true, |ls| ls.can_add_lock(lock))
                && !self
                    .poisoned_locks
                    .iter()
                    .any(|plock| lock.is_conflicting(plock))
        })
    }

    fn supports_all_features(&self, task: &Task) -> bool {
        task.features
            .iter()
            .all(|feat| self.machine_features.contains(feat))
    }

    fn should_pick_task(&self, task: &Task) -> bool {
        task.status == TaskStatus::NEW
            && self.supports_all_features(task)
            && self.is_unconstrained(task)
    }

    pub fn get_next_task(mut self) -> Option<Task> {
        self.tasks
            .iter()
            .position(|task| self.should_pick_task(task))
            .map(|idx| self.tasks.remove(idx))
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::database::entities::{Lock, Task};
    use crate::database::entities::task::TaskStatus;

    use super::Scheduler;

    #[test]
    fn scheduler_creation_no_constraints() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::RUNNING)
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::RUNNING)
                .build(),
        ];

        Scheduler::new(tasks, &[], &[]).unwrap();
    }

    #[test]
    fn scheduler_creation_multiple_read_constraints() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::RUNNING)
                .with_read_lock("foo 1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::RUNNING)
                .with_read_lock("foo 1")
                .build(),
        ];

        Scheduler::new(tasks, &[], &[]).unwrap();
    }

    #[test]
    fn scheduler_creation_single_write_constraints() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::RUNNING)
                .with_write_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::RUNNING)
                .with_write_lock("foo2")
                .build(),
        ];

        Scheduler::new(tasks, &[], &[]).unwrap();
    }

    #[test]
    fn scheduler_creation_multiple_write_constraints() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::RUNNING)
                .with_write_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::RUNNING)
                .with_write_lock("foo1")
                .build(),
        ];

        let res = Scheduler::new(tasks, &[], &[]);
        assert!(res.is_err());
    }

    #[test]
    fn scheduler_no_new_task() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::RUNNING)
                .with_write_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::NEW)
                .with_write_lock("foo1")
                .build(),
        ];

        let res = Scheduler::new(tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task(), None)
    }

    #[test]
    fn scheduler_no_new_task_with_feature() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::NEW)
                .requires_feature("huge_cpu")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::NEW)
                .requires_feature("huge_cpu")
                .build(),
        ];

        let res = Scheduler::new(tasks, &[], &[]).unwrap();
        // Test 1 and Test 2 have required features, which our runner does not have.
        assert_eq!(res.get_next_task(), None)
    }

    #[test]
    fn scheduler_new_task_with_specific_feature() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::NEW)
                .requires_feature("huge_cpu")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::NEW)
                .requires_feature("huge_cpu")
                .build(),
        ];

        let features = &["huge_cpu".to_string()];
        let res = Scheduler::new(tasks, &[], features).unwrap();
        // Test 1 and Test 2 have required features, which our runner matches.
        assert_eq!(res.get_next_task().unwrap().display_name, "Test 1")
    }

    #[test]
    fn scheduler_new_task() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::RUNNING)
                .with_write_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::NEW)
                .with_write_lock("foo2")
                .build(),
        ];

        let res = Scheduler::new(tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task().unwrap().display_name, "Test 2")
    }

    #[test]
    fn scheduler_new_task_ro() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::RUNNING)
                .with_read_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::NEW)
                .with_read_lock("foo1")
                .build(),
        ];

        let res = Scheduler::new(tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task().unwrap().display_name, "Test 2")
    }

    #[test]
    fn scheduler_new_task_rw_ro() {
        let tasks = vec![
            Task::builder()
                .with_display_name("Test 1")
                .with_status(TaskStatus::RUNNING)
                .with_write_lock("foo1")
                .build(),
            Task::builder()
                .with_display_name("Test 2")
                .with_status(TaskStatus::NEW)
                .with_read_lock("foo1")
                .build(),
        ];

        let res = Scheduler::new(tasks, &[], &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task(), None)
    }

    #[test]
    fn schedule_with_poisoned_lock() {
        let tasks = vec![Task::builder()
            .with_display_name("I need to do something")
            .with_write_lock("Entire Prod Cluster")
            .build()];
        let poisoned_locks = vec![Lock::WRITE {
            name: "Entire Prod Cluster".to_string(),
            poisoned: Some(Uuid::new_v4()),
        }];

        let res = Scheduler::new(tasks, &poisoned_locks, &[]).unwrap();

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
        let poisoned_locks = vec![Lock::WRITE {
            name: "Entire Prod Cluster".to_string(),
            poisoned: Some(Uuid::new_v4()),
        }];

        let res = Scheduler::new(tasks, &poisoned_locks, &[]).unwrap();

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
        let poisoned_locks = vec![Lock::READ {
            name: "Entire Prod Cluster".to_string(),
            poisoned: Some(Uuid::new_v4()),
        }];

        let res = Scheduler::new(tasks, &poisoned_locks, &[]).unwrap();

        assert_eq!(res.get_next_task(), None);
    }
}
