use rocket::response::stream::{Event, EventStream};
use rocket::{State, get};
use std::time;
use tokio::sync::broadcast::{self, error::TryRecvError};
use vickylib::vicky::events::GlobalEvent;

#[get("/")]
pub fn get_global_events(
    global_events: &State<broadcast::Sender<GlobalEvent>>,
) -> EventStream![Event + '_] {
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
                    // Immediate Retry, doing our best effort here.
                },
                Err(TryRecvError::Empty) => {
                    tokio::time::sleep(time::Duration::from_millis(100)).await;
                },
            }
        }
    }
}
