//! Bounded callback transport. Overflow is a safety fault, never silently lost input.
use crate::domain::models::AppEvent;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct EventSender {
    sender: mpsc::Sender<AppEvent>,
    overflow: Arc<AtomicBool>,
}

impl EventSender {
    pub fn channel(capacity: usize) -> (Self, mpsc::Receiver<AppEvent>) {
        let (sender, receiver) = mpsc::channel(capacity);
        (
            Self {
                sender,
                overflow: Arc::new(AtomicBool::new(false)),
            },
            receiver,
        )
    }

    pub fn send(&self, event: AppEvent) -> Result<(), mpsc::error::TrySendError<AppEvent>> {
        let discovery = matches!(&event, AppEvent::DevicesUpdated(_));
        self.sender.try_send(event).inspect_err(|error| {
            if !discovery && matches!(error, mpsc::error::TrySendError::Full(_)) {
                self.overflow.store(true, Ordering::Release);
            }
        })
    }

    pub fn take_overflow(&self) -> bool {
        self.overflow.swap(false, Ordering::AcqRel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::ConnectionStatus;
    #[test]
    fn overflow_is_reported_and_reset() {
        let (sender, _receiver) = EventSender::channel(1);
        assert!(sender
            .send(AppEvent::ConnectionStatus(ConnectionStatus::Connected))
            .is_ok());
        assert!(sender
            .send(AppEvent::ConnectionStatus(ConnectionStatus::Disconnected))
            .is_err());
        assert!(sender.take_overflow());
        assert!(!sender.take_overflow());
    }
    #[test]
    fn dropped_discovery_snapshot_does_not_trip_input_safety() {
        let (sender, _receiver) = EventSender::channel(1);
        assert!(sender.send(AppEvent::DevicesUpdated(vec![])).is_ok());
        assert!(sender.send(AppEvent::DevicesUpdated(vec![])).is_err());
        assert!(!sender.take_overflow());
        assert!(sender
            .send(AppEvent::ConnectionStatus(ConnectionStatus::Disconnected))
            .is_err());
        assert!(sender.take_overflow());
    }
}
