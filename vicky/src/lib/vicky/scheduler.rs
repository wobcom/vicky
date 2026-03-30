use crate::database::entities::Database;
use crate::errors::SchedulerError;
use crate::errors::VickyError;
use crate::vicky::events::GlobalEvent;
use crate::{database::entities::Task, vicky::constraints_helper::ConstraintsHelper};
use diesel::PgConnection;
use log::warn;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;

pub struct Scheduler {
    fairy_handles_tx: tokio::sync::mpsc::Sender<FairyHandle>,
    fairy_handles_rx: std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<FairyHandle>>>,
}

pub struct FairyHandle {
    features: HashSet<String>,
    task_tx: tokio::sync::oneshot::Sender<Task>,
}

impl Scheduler {
    pub fn new() -> Arc<Self> {
        let (fairy_handles_tx, fairy_handles_rx) = tokio::sync::mpsc::channel(1);
        Arc::new(Self {
            fairy_handles_tx,
            fairy_handles_rx: Some(fairy_handles_rx).into(),
        })
    }

    pub async fn get_next_task(&self, machine_features: &[String]) -> Task {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.fairy_handles_tx
            .send(FairyHandle {
                features: machine_features.iter().cloned().collect(),
                task_tx: tx,
            })
            .await
            .unwrap();
        rx.await.unwrap()
    }

    pub async fn run(
        self: Arc<Self>,
        global_events: tokio::sync::broadcast::Sender<GlobalEvent>,
        db_pool: rocket_sync_db_pools::ConnectionPool<Database, PgConnection>,
    ) -> Result<(), VickyError> {
        let mut fairy_handles_rx = self
            .fairy_handles_rx
            .lock()
            .unwrap()
            .take()
            .expect("Scheduler can only be running once");
        let mut global_events_rx = global_events.subscribe();
        let mut waiting_fairies = VecDeque::<FairyHandle>::new();
        loop {
            let Some(db) = Database::get_one_from_pool(&db_pool).await else {
                warn!("Scheduler timed out waiting for database connection");
                continue;
            };

            while {
                // do: try to schedule a task

                let mut task_scheduled = false;

                // empty the queue, all changes up to here will have been accounted for
                global_events_rx.resubscribe();

                let all_features = waiting_fairies
                    .iter()
                    .flat_map(|fairy| fairy.features.iter())
                    .cloned()
                    .collect::<HashSet<_>>();

                let tasks = db.get_all_tasks().await?;
                let poisoned_locks = db.get_poisoned_locks().await?;
                let constraints_helper =
                    ConstraintsHelper::new(&tasks, &poisoned_locks, &all_features)?;

                if let Some(next_task) = constraints_helper.get_next_task() {
                    while let Some((idx, _)) = waiting_fairies
                        .iter()
                        .enumerate()
                        .find(|(_, fairy)| fairy.features.is_superset(&next_task.features))
                    {
                        let fairy = waiting_fairies.remove(idx).unwrap();
                        if fairy.task_tx.send(next_task.clone()).is_ok() {
                            task_scheduled = true;
                            break;
                        }
                    }
                }

                // while: task was scheduled
                task_scheduled
            } {}

            tokio::select! {
                res = global_events_rx.recv() => {
                    // wait for changed or new tasks...
                    match res {
                        Ok(_) => {},
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {},
                        Err(_) => return Err(SchedulerError::ChannelClosed.into()),
                    }
                }
                res = fairy_handles_rx.recv() => {
                    // ... or a new fairy asking for a task
                    let fairy_handle = res.ok_or(SchedulerError::ChannelClosed)?;
                    waiting_fairies.push_back(fairy_handle);
                }
            }
        }
    }
}
