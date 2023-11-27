use etcd_client::{Client};
use rocket::{get, post, State, serde::json::Json};
use serde::{Deserialize, Serialize};
use rocket::response::stream::{EventStream, Event};
use std::time;
use tokio::sync::broadcast::{error::{TryRecvError}, self};


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GlobalEvent {
    TaskAdd,
    TaskUpdate {
        uuid: uuid::Uuid
    }
}

#[get("/")]
pub fn get_global_events(global_events: &State<broadcast::Sender<GlobalEvent>>) -> EventStream![Event + '_] {
    EventStream! {

        let mut global_events_rx = global_events.subscribe();
        
        loop {

            let read_val = global_events_rx.try_recv();

            match read_val {
                Ok(v) => {
                    yield Event::json(&v);
                },
                Err(TryRecvError::Closed) => {
                    break;
                },
                Err(TryRecvError::Lagged(_)) => {
                    // Immediate Retry, doing our best efford ehre.
                },
                Err(TryRecvError::Empty) => {
                    tokio::time::sleep(time::Duration::from_millis(100)).await;
                },
            }
        }
    }
}