//! Bounded retry policy. An explicit disconnect cancels all automatic attempts.
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct ReconnectPolicy {
    address: Option<u64>,
    next_attempt: Option<Instant>,
    attempts: u8,
    manually_disconnected: bool,
}

impl ReconnectPolicy {
    pub fn start(&mut self, address: u64, now: Instant) {
        self.address = Some(address);
        self.manually_disconnected = false;
        self.attempts = 0;
        self.next_attempt = Some(now);
    }

    pub fn connected(&mut self, address: u64) {
        self.address = Some(address);
        self.attempts = 0;
        self.next_attempt = None;
        self.manually_disconnected = false;
    }

    pub fn disconnect(&mut self) {
        self.manually_disconnected = true;
        self.next_attempt = None;
    }

    pub fn lost(&mut self, enabled: bool, now: Instant) {
        if enabled
            && !self.manually_disconnected
            && self.next_attempt.is_none()
            && self.attempts < 3
        {
            self.next_attempt = Some(now + Duration::from_secs(2));
        }
    }

    pub fn take_due(&mut self, now: Instant) -> Option<(u64, u8)> {
        if self.next_attempt.is_some_and(|deadline| now >= deadline) && self.attempts < 3 {
            self.next_attempt = None;
            self.attempts += 1;
            return self.address.map(|address| (address, self.attempts));
        }
        None
    }

    pub fn disable(&mut self) {
        self.next_attempt = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_stop_after_three_attempts_and_manual_disconnect() {
        let now = Instant::now();
        let mut policy = ReconnectPolicy::default();
        policy.start(42, now);
        for attempt in 1..=3 {
            assert_eq!(
                policy.take_due(now + Duration::from_secs(10)),
                Some((42, attempt))
            );
            policy.lost(true, now);
        }
        assert_eq!(policy.take_due(now + Duration::from_secs(10)), None);
        policy.connected(42);
        policy.disconnect();
        policy.lost(true, now);
        assert_eq!(policy.take_due(now + Duration::from_secs(10)), None);
    }
}
