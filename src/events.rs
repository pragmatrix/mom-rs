//! A library for coordinating concurrent processes.

use anyhow::Result;
use std::{any::Any, ops::Deref, sync::Arc};
use tokio::sync::broadcast::{Receiver, Sender};

pub const CHANNEL_SIZE: usize = 16;

pub fn new_channel() -> (EventSink, EventSource) {
    let (tx, rx) = tokio::sync::broadcast::channel(CHANNEL_SIZE);
    (EventSink { channel: tx }, EventSource { channel: rx })
}

/// A sink that posts events to all `EventSources`.
pub struct EventSink {
    channel: Sender<Event>,
}

impl EventSink {
    /// Post an event to the event sink.
    ///
    /// Returns an error if the event could not be sent.
    pub fn post<T>(&self, event: T) -> Result<()>
    where
        T: 'static + Send + Sync,
    {
        self.channel.send(Event::new(event))?;
        Ok(())
    }
}

/// A cheap to clone event source on which to listen for events.
pub struct EventSource {
    channel: Receiver<Event>,
}

impl Clone for EventSource {
    fn clone(&self) -> Self {
        EventSource {
            channel: self.channel.resubscribe(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    // Arc for cheap cloning.
    content: Box<Arc<dyn Any + Send + Sync>>,
}

impl Event {
    pub fn new<T>(content: T) -> Self
    where
        T: 'static + Send + Sync,
    {
        Event {
            content: Box::new(Arc::new(content)),
        }
    }
}

impl EventSource {
    /// Waits for the `EventSource` to receive an event of type `T` and return the result of a
    /// function that returns an `Option<R>`. If the function returns `None`, the event is ignored
    /// and the function waits for the next event.
    ///
    /// This function locks the `EventSource` exclusively. If you want to listen to multiple events
    /// in parallel, the `EventSource` must be cloned.
    ///
    /// The selection function is called with a reference to the content, so that the channel where
    /// the event is received can be determined from can be cheaply cloned internally.
    pub async fn wait<T, R>(&mut self, mut f: impl FnMut(&T) -> Option<R>) -> Result<R>
    where
        T: 'static + Send + Sync,
    {
        loop {
            let event = self.channel.recv().await?;
            if let Some(event_ref) = event.content.deref().deref().downcast_ref::<T>() {
                if let Some(result) = f(event_ref) {
                    return Ok(result);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{new_channel, race};
    use anyhow::Result;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_event_source() -> Result<()> {
        let (sink, mut source) = new_channel();

        let handle = tokio::spawn(async move {
            let result = timeout(Duration::from_secs(1), source.wait(|v: &i32| Some(*v))).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().unwrap(), 10);
            Ok::<_, anyhow::Error>(())
        });

        sink.post(10i32)?;
        handle.await??;

        Ok(())
    }

    #[tokio::test]
    async fn test_race() -> Result<()> {
        let (sink, mut source) = new_channel();
        let mut source2 = source.clone();

        let handle = tokio::spawn(async move {
            let wait_i32 = source.wait(|_: &i32| {
                println!("i32");
                Some(false)
            });
            let wait_u32 = source2.wait(|_: &u32| {
                println!("u32");
                Some(true)
            });

            let result = race!(wait_i32, wait_u32);

            assert!(result.unwrap());
            Ok::<_, anyhow::Error>(())
        });

        sink.post(10u32)?;
        handle.await??;

        Ok(())
    }
}
