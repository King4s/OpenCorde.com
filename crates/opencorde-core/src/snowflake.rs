//! # Snowflake ID Generator
//! 64-bit time-ordered IDs.
//!
//! Format: [42 bits: timestamp][5 bits: worker][5 bits: process][12 bits: sequence]
//! Custom epoch: 2024-01-01T00:00:00Z (1704067200000 ms)
//! Provides ~139 years of valid ID generation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Custom epoch: 2024-01-01 00:00:00 UTC in milliseconds
const EPOCH_MILLIS: i64 = 1704067200000;

/// Snowflake: 64-bit time-ordered unique identifier
/// Format: [42 bits: timestamp][5 bits: worker][5 bits: process][12 bits: sequence]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Snowflake(i64);

impl Snowflake {
    /// Creates a new Snowflake from an i64 value.
    pub fn new(id: i64) -> Self {
        Snowflake(id)
    }

    /// Extracts the timestamp (in milliseconds since custom epoch) from the Snowflake.
    pub fn timestamp(&self) -> i64 {
        self.0 >> 22
    }

    /// Converts the Snowflake timestamp to a UTC DateTime.
    pub fn created_at(&self) -> DateTime<Utc> {
        let millis = self.timestamp() + EPOCH_MILLIS;
        DateTime::from_timestamp_millis(millis).expect("valid timestamp")
    }

    /// Returns the internal i64 representation.
    pub fn as_i64(&self) -> i64 {
        self.0
    }
}

impl Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Snowflake {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        i64::from_str(s).map(Snowflake)
    }
}

impl From<i64> for Snowflake {
    fn from(value: i64) -> Self {
        Snowflake(value)
    }
}

impl From<Snowflake> for i64 {
    fn from(value: Snowflake) -> Self {
        value.0
    }
}

/// Snowflake generator: creates monotonically increasing IDs.
/// NOT thread-safe — use behind a Mutex or Arc<Mutex<>> in multi-threaded code.
#[derive(Debug, Clone)]
pub struct SnowflakeGenerator {
    worker_id: u8,
    process_id: u8,
    sequence: u16,
    last_timestamp: i64,
}

impl SnowflakeGenerator {
    /// Creates a new generator with the given worker and process IDs.
    /// IDs must be 0-31 (5 bits each).
    ///
    /// # Panics
    /// Panics if worker_id or process_id > 31.
    pub fn new(worker_id: u8, process_id: u8) -> Self {
        assert!(worker_id < 32, "worker_id must be < 32");
        assert!(process_id < 32, "process_id must be < 32");

        let now = Self::current_timestamp();
        SnowflakeGenerator {
            worker_id,
            process_id,
            sequence: 0,
            last_timestamp: now,
        }
    }

    /// Generates the next Snowflake ID.
    ///
    /// # Panics
    /// Panics if the system clock goes backward.
    pub fn next_id(&mut self) -> Snowflake {
        let now = Self::current_timestamp();

        if now < self.last_timestamp {
            panic!("System clock went backward");
        }

        if now == self.last_timestamp {
            // Same millisecond: increment sequence
            self.sequence = self.sequence.wrapping_add(1);
            if self.sequence == 0 {
                // Sequence overflowed, wait for next millisecond
                self.last_timestamp = Self::wait_next_millis(now);
                self.sequence = 0;
            }
        } else {
            // New millisecond: reset sequence
            self.last_timestamp = now;
            self.sequence = 0;
        }

        let timestamp_part = (now - EPOCH_MILLIS) << 22;
        let worker_part = (self.worker_id as i64) << 17;
        let process_part = (self.process_id as i64) << 12;
        let sequence_part = self.sequence as i64;

        Snowflake(timestamp_part | worker_part | process_part | sequence_part)
    }

    fn current_timestamp() -> i64 {
        Utc::now().timestamp_millis()
    }

    fn wait_next_millis(current: i64) -> i64 {
        let mut next = Self::current_timestamp();
        while next <= current {
            next = Self::current_timestamp();
        }
        next
    }
}

use std::fmt::Display;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snowflake_creation() {
        let sf = Snowflake::new(12345);
        assert_eq!(sf.as_i64(), 12345);
    }

    #[test]
    fn test_snowflake_from_i64() {
        let sf: Snowflake = 54321i64.into();
        assert_eq!(sf.as_i64(), 54321);
    }

    #[test]
    fn test_snowflake_into_i64() {
        let sf = Snowflake::new(99999);
        let val: i64 = sf.into();
        assert_eq!(val, 99999);
    }

    #[test]
    fn test_snowflake_display() {
        let sf = Snowflake::new(12345);
        assert_eq!(format!("{}", sf), "12345");
    }

    #[test]
    fn test_snowflake_from_str() {
        let sf: Snowflake = "67890".parse().unwrap();
        assert_eq!(sf.as_i64(), 67890);
    }

    #[test]
    fn test_snowflake_timestamp_extraction() {
        // Create a known timestamp (e.g., 1000 ms after epoch)
        let timestamp_ms = 1000i64;
        let worker = 1u8;
        let process = 2u8;
        let sequence = 3u16;

        let id = ((timestamp_ms) << 22)
            | ((worker as i64) << 17)
            | ((process as i64) << 12)
            | (sequence as i64);

        let sf = Snowflake(id);
        assert_eq!(sf.timestamp(), timestamp_ms);
    }

    #[test]
    fn test_generator_monotonicity() {
        let mut generator = SnowflakeGenerator::new(1, 1);
        let mut last_id: i64 = 0;

        for _ in 0..100 {
            let id = generator.next_id();
            assert!(id.as_i64() > last_id, "IDs must be strictly increasing");
            last_id = id.as_i64();
        }
    }

    #[test]
    fn test_generator_uniqueness() {
        let mut generator = SnowflakeGenerator::new(5, 10);
        let mut ids = Vec::new();

        for _ in 0..50 {
            ids.push(generator.next_id().as_i64());
        }

        // Check all are unique
        let len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), len, "All generated IDs must be unique");
    }

    #[test]
    fn test_generator_different_workers() {
        let mut gen1 = SnowflakeGenerator::new(1, 0);
        let mut gen2 = SnowflakeGenerator::new(2, 0);

        let id1 = gen1.next_id();
        let id2 = gen2.next_id();

        // Should be different due to different worker IDs
        assert_ne!(id1.as_i64(), id2.as_i64());
    }

    #[test]
    fn test_snowflake_created_at() {
        let sf = Snowflake::new(0);
        let dt = sf.created_at();
        // Should be at or very close to epoch
        let expected = DateTime::from_timestamp_millis(EPOCH_MILLIS).unwrap();
        assert_eq!(dt, expected);
    }

    #[test]
    fn test_snowflake_serialization() {
        let sf = Snowflake::new(999888777);
        let json = serde_json::to_string(&sf).unwrap();
        // Snowflake serializes as a string (no outer quotes added by serde_json::to_string)
        assert_eq!(json, "999888777");

        let deserialized: Snowflake = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.as_i64(), 999888777);
    }

    #[test]
    fn test_snowflake_equality_and_ordering() {
        let sf1 = Snowflake::new(100);
        let sf2 = Snowflake::new(100);
        let sf3 = Snowflake::new(200);

        assert_eq!(sf1, sf2);
        assert!(sf1 < sf3);
        assert!(sf3 > sf1);
    }
}
