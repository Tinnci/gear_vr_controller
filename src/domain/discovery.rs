//! Bounded, session-aware discovery. RSSI is evidence of reception, not identity.
#[cfg(test)]
mod tests;
use super::models::{BluetoothAddressKind, ScannedDevice};
use std::time::{Duration, Instant};

const CAPACITY: usize = 128;
const CACHE_AGE: Duration = Duration::from_secs(60);
const FRESH_AGE: Duration = Duration::from_secs(8);
pub const PUBLISH_INTERVAL: Duration = Duration::from_millis(750);

pub struct Advertisement {
    pub address: u64,
    pub address_kind: BluetoothAddressKind,
    pub name: String,
    pub rssi: i16,
    pub matches_service: bool,
}
struct Entry {
    device: ScannedDevice,
    average: f64,
    seen: Instant,
    current: bool,
}
#[derive(Default)]
pub struct DiscoveryCatalog {
    entries: Vec<Entry>,
    known: Vec<u64>,
    last_used: Option<u64>,
    show_all: bool,
}
impl DiscoveryCatalog {
    pub fn begin(&mut self, known: Vec<u64>, last_used: Option<u64>, show_all: bool, now: Instant) {
        self.entries
            .retain(|entry| now.saturating_duration_since(entry.seen) <= CACHE_AGE);
        self.known = known;
        self.last_used = last_used;
        self.show_all = show_all;
        for entry in &mut self.entries {
            entry.current = false;
            entry.device.available = false;
            entry.device.known = self.known.contains(&entry.device.address);
        }
    }
    pub fn observe(&mut self, advertisement: Advertisement, now: Instant) {
        if advertisement.address == 0 || advertisement.address > 0xFFFF_FFFF_FFFF {
            return;
        }
        let position = self.entries.iter().position(|entry| {
            entry.device.address == advertisement.address
                && entry.device.address_kind == advertisement.address_kind
        });
        // Windows uses -127 for out-of-range reports; never feed it into the average.
        if !(-126..=20).contains(&advertisement.rssi) {
            if let Some(position) = position {
                self.entries[position].device.available = false;
            }
            return;
        }
        let name: String = advertisement
            .name
            .chars()
            .filter(|ch| !ch.is_control())
            .take(64)
            .collect();
        let name = name.trim().to_owned();
        if let Some(position) = position {
            let entry = &mut self.entries[position];
            if entry.current {
                entry.average += 0.25 * (f64::from(advertisement.rssi) - entry.average);
            } else {
                entry.average = f64::from(advertisement.rssi);
            }
            entry.device.signal_strength = entry.average.round() as i16;
            if name.chars().count() > entry.device.name.chars().count() {
                entry.device.name = name;
            }
            entry.device.matches_service |= advertisement.matches_service;
            entry.device.available = true;
            entry.seen = now;
            entry.current = true;
            return;
        }
        let known = self.known.contains(&advertisement.address);
        if self.entries.len() == CAPACITY {
            // Noise must not crowd a newly discovered compatible/previously used device out.
            let victim = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| !entry.device.known && !entry.device.matches_service)
                .min_by_key(|(_, entry)| entry.seen)
                .map(|(index, _)| index);
            if !known && !advertisement.matches_service {
                return;
            }
            let Some(victim) = victim else {
                return;
            };
            self.entries.remove(victim);
        }
        self.entries.push(Entry {
            device: ScannedDevice {
                name,
                address: advertisement.address,
                address_kind: advertisement.address_kind,
                signal_strength: advertisement.rssi,
                matches_service: advertisement.matches_service,
                known,
                available: true,
            },
            average: f64::from(advertisement.rssi),
            seen: now,
            current: true,
        });
    }
    pub fn snapshot(&self, now: Instant) -> Vec<ScannedDevice> {
        self.entries
            .iter()
            .filter(|entry| entry.device.known || entry.device.matches_service || self.show_all)
            .map(|entry| {
                let mut device = entry.device.clone();
                device.available &=
                    entry.current && now.saturating_duration_since(entry.seen) <= FRESH_AGE;
                device
            })
            .collect()
    }
    /// Reorder once at scan completion, never on every RSSI or name update.
    pub fn finish(&mut self, now: Instant) {
        for entry in &mut self.entries {
            entry.device.available &=
                entry.current && now.saturating_duration_since(entry.seen) <= FRESH_AGE;
        }
        let last_used = self.last_used;
        self.entries.sort_by_key(|entry| {
            (
                !entry.device.available,
                Some(entry.device.address) != last_used,
                !entry.device.known,
                !entry.device.matches_service,
                std::cmp::Reverse(entry.device.signal_strength),
                entry.device.address,
            )
        });
    }
}
