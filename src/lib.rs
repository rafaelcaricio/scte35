use std::io;
use std::time::Duration;
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};

mod commands;
mod descriptors;
mod info;
mod time;

pub use commands::SpliceNull;
pub use info::{EncryptionAlgorithm, SAPType, SpliceInfoSection};

pub trait TransportPacketWrite {
    fn write_to<W>(&self, buffer: &mut W) -> Result<(), CueError>
    where
        W: io::Write;
}

#[derive(Error, Debug)]
#[error("Could not execute operation due to {0}")]
pub enum CueError {
    Io(#[from] io::Error),
}

pub trait ClockTimeExt {
    fn to_90k(&self) -> u64;
}

impl ClockTimeExt for Duration {
    fn to_90k(&self) -> u64 {
        (self.as_secs_f64() * 90_000.0).floor() as u64
    }
}

/// Truncate to 6 decimal positions, as shown in the spec.
fn ticks_to_secs(value: u64) -> f64 {
    (value as f64 / 90_000.0 * 1_000_000.0).ceil() as f64 / 1_000_000.0
}

#[cfg(feature = "serde")]
fn serialize_time<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(ticks_to_secs(*value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_time() {
        let duration = Duration::from_secs(1);
        assert_eq!(duration.to_90k(), 90_000);
    }

    #[test]
    fn test_spec_example() {
        let time = Duration::from_secs_f64(21388.766756);
        assert_eq!(time.to_90k(), 0x072bd0050);
    }

    #[test]
    fn test_ticks_to_secs() {
        let time = Duration::from_secs_f64(21388.766756);
        assert_eq!(time.to_90k(), 0x072bd0050);
        assert_eq!(ticks_to_secs(0x072bd0050), 21388.766756);
        assert_eq!(ticks_to_secs(time.to_90k()), 21388.766756);
    }
}
