use std::collections::{HashMap, VecDeque};
use std::time;

use crate::{errors::VickyError, s3::client::S3Client};
use log::error;
use rocket::futures::lock::Mutex;
use tokio::sync::broadcast::{self, error::TryRecvError, Sender};
use uuid::Uuid;

const LOG_BUFFER: usize = 10000;

pub struct LogDrain {
    pub send_handle: Sender<(Uuid, String)>,

    live_log_buffers: Mutex<HashMap<Uuid, VecDeque<String>>>,
    push_log_buffers: Mutex<HashMap<Uuid, Vec<String>>>,

    s3_client: S3Client,
}

impl LogDrain {
    pub fn new(s3_client: S3Client) -> &'static LogDrain {
        let (tx, rx1) = broadcast::channel(1000);
        let s3_client_m = Box::leak(Box::new(s3_client.clone()));

        let ld: LogDrain = LogDrain {
            send_handle: tx,
            live_log_buffers: Mutex::new(HashMap::new()),
            push_log_buffers: Mutex::new(HashMap::new()),

            s3_client,
        };

        let ldr = Box::leak(Box::new(ld));
        let rx1r = Box::leak(Box::new(rx1));

        tokio::spawn(async {
            loop {
                let read_val = rx1r.try_recv();

                match read_val {
                    Ok((task_id, log_text)) => {
                        {
                            let mut llb = ldr.live_log_buffers.lock().await;

                            let live_log_buffer = llb.entry(task_id).or_insert_with(VecDeque::new);
                            if live_log_buffer.len() == LOG_BUFFER {
                                live_log_buffer.pop_front();
                            }
                            live_log_buffer.push_back(log_text.clone());
                        }

                        {
                            let mut push_log_buffers = ldr.push_log_buffers.lock().await;

                            let push_log_buffer =
                                push_log_buffers.entry(task_id).or_insert_with(Vec::new);
                            push_log_buffer.push(log_text.clone());

                            // TODO: Figure out a good buffer length for our use case.
                            if push_log_buffer.len() > 16 {
                                // Push buffer to S3

                                match s3_client_m
                                    .upload_log_parts(task_id, push_log_buffer.to_vec())
                                    .await
                                {
                                    Ok(_) => push_log_buffer.clear(),
                                    Err(err) => {
                                        error!("failed to upload log parts for {task_id}: {err}");
                                    }
                                }
                            }
                        }
                    }
                    Err(TryRecvError::Closed) => {
                        // TODO: Do something about this.
                        // Technically, this should not happen, because we control all of the send handles.
                    }
                    Err(TryRecvError::Lagged(_)) => {
                        // Immediate Retry, doing our best effort here.
                    }
                    Err(TryRecvError::Empty) => {
                        tokio::time::sleep(time::Duration::from_millis(10)).await;
                    }
                }
            }
        });

        ldr
    }

    pub fn push_logs(&self, task_id: Uuid, logs: Vec<String>) -> Result<(), VickyError> {
        for log in logs {
            self.send_handle.send((task_id, log))?;
        }

        Ok(())
    }

    pub async fn get_logs(&self, task_id: Uuid) -> Option<Vec<String>> {
        let new_vec: Vec<String> = self
            .live_log_buffers
            .lock()
            .await
            .get(&task_id)?
            .clone()
            .into();
        Some(new_vec)
    }

    pub async fn finish_logs(&self, task_id: Uuid) -> Result<(), VickyError> {
        let mut push_log_buffers = self.push_log_buffers.lock().await;
        if let Some(push_log_buffer) = push_log_buffers.get_mut(&task_id) {
            if !push_log_buffer.is_empty() {
                self.s3_client
                    .upload_log_parts(task_id, push_log_buffer.to_vec())
                    .await?;
            }
            push_log_buffers.remove(&task_id);
        }
        Ok(())
    }
}
