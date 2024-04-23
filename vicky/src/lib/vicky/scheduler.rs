use std::collections::HashMap;

use log::debug;

use crate::{documents::{Lock, Task, TaskStatus}, errors::SchedulerError};

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
            Lock::WRITE { object } => {
                object                    
            },
            Lock::READ { object } => {
                object 
            },
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
            },
            None => {
                debug!("Found no LockSum");
                let object = Constraints::get_map_key(lock);
                self.insert(object.clone(), LockSum::from_lock(lock));
            },
        }

        Ok(())
    }


}


struct LockSum {
    lock: Lock,
    count: i32
}

impl LockSum {
    pub fn from_lock(lock: &Lock) -> LockSum {
        LockSum { lock: lock.clone(), count: 1 }
    }

    pub fn can_add_lock(&self, lock: &Lock) -> bool {
        match (&self.lock, lock) {
            (Lock::WRITE { object: _ }, Lock::WRITE { object: _ }) => false,
            (Lock::WRITE { object: _ }, Lock::READ { object: _ }) => false,
            (Lock::READ { object: _ }, Lock::WRITE { object: _ }) => false,
            (Lock::READ { object }, Lock::READ { object: object2 }) => object == object2,
        }
    }

    pub fn add_lock(&mut self, lock: &Lock) -> Result<(), SchedulerError> {

        let can_add_lock = self.can_add_lock(lock);
        if !can_add_lock {
            return Err(SchedulerError::GeneralSchedulingError);
        }

        match lock {
            Lock::READ { object: _ } => {
                self.count += 1;
                Ok(())
            },
            _ => unreachable!()
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
            machine_features: machine_features.clone().to_vec()
        };

        Ok(s)
    }


    pub fn get_next_task(self) -> Option<Task> {

        for task in self.tasks {
            if task.status != TaskStatus::NEW {
                continue;
            }

            if !task.features.iter().all(|feat| self.machine_features.contains(feat)) {
                continue;
            }

            let mut has_conflicts = false;

            for lock in &task.locks {
                let lock_sum = self.constraints.get_lock_sum(lock);
                match lock_sum {
                    Some(ls) => {
                        if !ls.can_add_lock(lock) {
                            has_conflicts = true;
                        }
                    },
                    None => continue,
                }
            }

            if !has_conflicts {
                return Some(task)   
            }
        }

        None

    }    
}


#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::documents::{TaskStatus, Task, FlakeRef, Lock};

    use super::Scheduler;

    #[test]
    fn scheduler_creation_no_constraints() {

        let tasks = vec![
            Task { id: Uuid::new_v4(), display_name: String::from("Test 1"), status: TaskStatus::RUNNING, locks: vec![], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
            Task { id: Uuid::new_v4(), display_name: String::from("Test 2"), status: TaskStatus::RUNNING, locks: vec![], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
        ];

        Scheduler::new(tasks, &[]).unwrap();

    }

    #[test]
    fn scheduler_creation_multiple_read_constraints() {

        let tasks = vec![
            Task { id: Uuid::new_v4(), display_name: String::from("Test 1"), status: TaskStatus::RUNNING, locks: vec![Lock::READ { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
            Task { id: Uuid::new_v4(), display_name: String::from("Test 2"), status: TaskStatus::RUNNING, locks: vec![Lock::READ { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
        ];

        Scheduler::new(tasks, &[]).unwrap();

    }

    #[test]
    fn scheduler_creation_single_write_constraints() {

        let tasks = vec![
            Task { id: Uuid::new_v4(), display_name: String::from("Test 1"), status: TaskStatus::RUNNING, locks: vec![Lock::WRITE { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
            Task { id: Uuid::new_v4(), display_name: String::from("Test 2"), status: TaskStatus::RUNNING, locks: vec![Lock::WRITE { object: String::from("foo2") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
        ];

        Scheduler::new(tasks, &[]).unwrap();

    }

    #[test]
    fn scheduler_creation_multiple_write_constraints() {

        let tasks = vec![
            Task { id: Uuid::new_v4(), display_name: String::from("Test 1"), status: TaskStatus::RUNNING, locks: vec![Lock::WRITE { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
            Task { id: Uuid::new_v4(), display_name: String::from("Test 2"), status: TaskStatus::RUNNING, locks: vec![Lock::WRITE { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
        ];
        
        let res = Scheduler::new(tasks, &[]);
        assert!(res.is_err());

    }

    #[test]
    fn scheduler_no_new_task() {
        let tasks = vec![
            Task { id: Uuid::new_v4(), display_name: String::from("Test 1"), status: TaskStatus::RUNNING, locks: vec![Lock::WRITE { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
            Task { id: Uuid::new_v4(), display_name: String::from("Test 2"), status: TaskStatus::NEW, locks: vec![Lock::WRITE { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
        ];
        
        let res = Scheduler::new(tasks, &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task(), None)
        
    }

    #[test]
    fn scheduler_new_task() {
        let tasks = vec![
            Task { id: Uuid::new_v4(), display_name: String::from("Test 1"), status: TaskStatus::RUNNING, locks: vec![Lock::WRITE { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
            Task { id: Uuid::new_v4(), display_name: String::from("Test 2"), status: TaskStatus::NEW, locks: vec![Lock::WRITE { object: String::from("foo2") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
        ];
        
        let res = Scheduler::new(tasks, &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert!(res.get_next_task().unwrap().display_name == "Test 2")
        
    }

    #[test]
    fn scheduler_new_task_ro() {
        let tasks = vec![
            Task { id: Uuid::new_v4(), display_name: String::from("Test 1"), status: TaskStatus::RUNNING, locks: vec![Lock::READ { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
            Task { id: Uuid::new_v4(), display_name: String::from("Test 2"), status: TaskStatus::NEW, locks: vec![Lock::READ { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
        ];
        
        let res = Scheduler::new(tasks, &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert!(res.get_next_task().unwrap().display_name == "Test 2")
        
    }

    #[test]
    fn scheduler_new_task_rw_ro() {
        let tasks = vec![
            Task { id: Uuid::new_v4(), display_name: String::from("Test 1"), status: TaskStatus::RUNNING, locks: vec![Lock::WRITE { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
            Task { id: Uuid::new_v4(), display_name: String::from("Test 2"), status: TaskStatus::NEW, locks: vec![Lock::READ { object: String::from("foo1") }], flake_ref: FlakeRef { flake: String::from(""), args: vec![] }, features: vec![] },
        ];
        
        let res = Scheduler::new(tasks, &[]).unwrap();
        // Test 1 is currently running and has the write lock
        assert_eq!(res.get_next_task(), None)
        
    }
}