use crate::database::entities::{Lock, Task};
use crate::errors::SchedulerError;
use std::collections::HashMap;

#[derive(Clone, Debug)]
#[allow(unused)]
pub enum ConstraintFail<'a> {
    UnsupportedFeature(String),
    ActiveLockCollision(&'a Lock),
    PassiveLockCollision(&'a Lock),
    PoisonedBy(&'a Lock),
}

#[derive(Clone, Debug)]
#[allow(unused)]
pub enum ConstraintEvaluation<'a> {
    Ready,
    NotReady,
    Constrained(ConstraintFail<'a>),
}

#[derive(Clone, Debug, Default)]
pub struct Constraints<'a> {
    /// Takes all active locks and validates running ownership
    active_locks: HashMap<&'a str, &'a Lock>,
    /// Takes all non-running locks that need to be considered for future acquires
    passive_locks: HashMap<&'a str, Vec<&'a Lock>>,
    /// Takes all non-running locks that need to be considered for cleanup order
    waiting_locks: HashMap<&'a str, Vec<&'a Lock>>,
    poisoned_locks: &'a [Lock],
}

impl<'a> Constraints<'a> {
    pub fn insert_task_locks(&mut self, task: &'a Task) -> Result<(), SchedulerError> {
        for lock in &task.locks {
            if task.is_running() {
                self.insert_active_lock(lock)?;
            } else if task.is_waiting_confirmation() {
                self.insert_passive_lock(lock);
            } else if task.is_new() {
                self.insert_waiting_lock(lock);
            }
        }

        Ok(())
    }

    fn insert_active_lock(&mut self, lock: &'a Lock) -> Result<(), SchedulerError> {
        if self.is_actively_locked(lock) {
            return Err(SchedulerError::LockAlreadyOwnedError);
        }

        self.active_locks.insert(lock.name(), lock);

        Ok(())
    }

    fn insert_passive_lock(&mut self, lock: &'a Lock) {
        self.passive_locks
            .entry(lock.name())
            .or_default()
            .push(lock);
    }

    fn insert_waiting_lock(&mut self, lock: &'a Lock) {
        if lock.kind.is_cleanup() {
            return;
        }

        self.waiting_locks
            .entry(lock.name())
            .or_default()
            .push(lock);
    }

    pub fn try_acquire(&'a self, lock: &Lock) -> Option<ConstraintFail<'a>> {
        if let Some(conflict) = self.find_active_conflict(lock) {
            return Some(ConstraintFail::ActiveLockCollision(conflict));
        }

        if let Some(conflict) = self.find_passive_conflict(lock) {
            return Some(ConstraintFail::PassiveLockCollision(conflict));
        }

        if let Some(conflict) = self.find_cleanup_conflict(lock) {
            return Some(ConstraintFail::PassiveLockCollision(conflict));
        }

        if let Some(poison) = self.find_poisoner(lock) {
            return Some(ConstraintFail::PoisonedBy(poison));
        }

        None
    }

    fn is_actively_locked(&self, lock: &Lock) -> bool {
        self.find_active_conflict(lock).is_some()
    }

    #[allow(unused)]
    fn is_poisoned(&self, lock: &Lock) -> bool {
        self.find_poisoner(lock).is_some()
    }

    fn find_active_conflict(&self, lock: &Lock) -> Option<&'a Lock> {
        let existing_lock = self.active_locks.get(lock.name())?;

        if lock.kind.is_cleanup() || existing_lock.kind.is_cleanup() {
            return Some(existing_lock);
        }

        lock.is_conflicting(existing_lock).then_some(existing_lock)
    }

    fn find_passive_conflict(&self, lock: &Lock) -> Option<&'a Lock> {
        let existing_locks = self.passive_locks.get(lock.name())?;

        if lock.kind.is_cleanup() {
            let existing_lock = *existing_locks.first()?;
            return Some(existing_lock);
        }

        existing_locks
            .iter()
            .copied()
            .find(|existing_lock| lock.is_conflicting(existing_lock))
    }

    fn find_poisoner(&self, lock: &Lock) -> Option<&'a Lock> {
        self.poisoned_locks
            .iter()
            .find(|plock| lock.is_conflicting(plock))
    }

    fn find_cleanup_conflict(&self, lock: &Lock) -> Option<&'a Lock> {
        if !lock.kind.is_cleanup() {
            return None;
        }

        let existing_locks = self.waiting_locks.get(lock.name())?;
        existing_locks.first().copied()
    }

    pub fn from_tasks(
        tasks: &'a [Task],
        poisoned_locks: &'a [Lock],
    ) -> Result<Self, SchedulerError> {
        let mut constraints = Self {
            poisoned_locks,
            ..Default::default()
        };

        for task in tasks {
            constraints.insert_task_locks(task)?;
        }

        Ok(constraints)
    }
}

impl ConstraintEvaluation<'_> {
    pub fn missing_feature(feature: String) -> Self {
        ConstraintEvaluation::Constrained(ConstraintFail::UnsupportedFeature(feature))
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, ConstraintEvaluation::Ready)
    }

    #[allow(unused)]
    pub fn is_active_collision(&self) -> bool {
        matches!(
            self,
            ConstraintEvaluation::Constrained(ConstraintFail::ActiveLockCollision(_))
        )
    }

    #[allow(unused)]
    pub fn is_passive_collision(&self) -> bool {
        matches!(
            self,
            ConstraintEvaluation::Constrained(ConstraintFail::PassiveLockCollision(_))
        )
    }
}
