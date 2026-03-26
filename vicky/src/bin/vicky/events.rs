use rocket::response::stream::{Event, EventStream};
use rocket::{State, get};
use tokio::sync::broadcast::{self, error::RecvError};
use vickylib::vicky::events::GlobalEvent;

#[get("/")]
pub fn get_global_events(
    global_events: &State<broadcast::Sender<GlobalEvent>>,
) -> EventStream![Event + '_] {
    EventStream! {

        let mut global_events_rx = global_events.subscribe();

        loop {
            let read_val = global_events_rx.recv().await;

            match read_val {
                Ok(v) => {
                    yield Event::json(&v);
                },
                Err(RecvError::Closed) => {
                    panic!("global_events closed");
                },
                Err(RecvError::Lagged(_)) => {
                    // Immediate Retry, doing our best effort here.

                },
            }
        }
    }
}
