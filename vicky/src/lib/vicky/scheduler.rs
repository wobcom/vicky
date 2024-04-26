use std::collections::HashMap;

use log::debug;

use crate::{
    database::entities::{Lock, Task, TaskStatus},
    errors::SchedulerError,
};

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
            Lock::WRITE { name: object } => object,
            Lock::READ { name: object } => object,
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
            (Lock::WRITE { name: _ }, Lock::WRITE { name: _ }) => false,
            (Lock::WRITE { name: _ }, Lock::READ { name: _ }) => false,
            (Lock::READ { name: _ }, Lock::WRITE { name: _ }) => false,
            (Lock::READ { name: object }, Lock::READ { name: object2 }) => object == object2,
        }
    }

    pub fn add_lock(&mut self, lock: &Lock) -> Result<(), SchedulerError> {
        let can_add_lock = self.can_add_lock(lock);
        if !can_add_lock {
            return Err(SchedulerError::GeneralSchedulingError);
        }

        match lock {
            Lock::READ { name: _ } => {
                self.count += 1;
                Ok(())
            }
            _ => unreachable!(),
        }
    }
}

pub struct Scheduler {
    constraints: Constraints,
    tasks: Vec<Task>,
    machine_features: Vec<String>,
}

impl Scheduler {
    pub fn new(tasks: Vec<Task>, machine_features: &[String]) -> Result<Self, SchedulerError> {
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
            machine_features: machine_features.to_vec(),
        };

        Ok(s)
    }

    fn is_unconstrained(&self, task: &Task) -> bool {
        task.locks.iter().all(|lock| {
            self.constraints
                .get_lock_sum(lock)
                .map_or(true, |ls| ls.can_add_lock(lock))
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
    use crate::database::entities::{Task, TaskStatus};

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
                .build()
        ];

        Scheduler::new(tasks, &[]).unwrap();
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
                .build()
        ];

        Scheduler::new(tasks, &[]).unwrap();
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
                .build()
        ];

        Scheduler::new(tasks, &[]).unwrap();
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

        let res = Scheduler::new(tasks, &[]);
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

        let res = Scheduler::new(tasks, &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task(), None)
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

        let res = Scheduler::new(tasks, &[]).unwrap();
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

        let res = Scheduler::new(tasks, &[]).unwrap();
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

        let res = Scheduler::new(tasks, &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task(), None)
    }
}
