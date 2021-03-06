use bitstream_io::{BigEndian, BitRecorder};
use std::io;
use std::time::Duration;
use thiserror::Error;

mod commands;
mod descriptors;
mod info;
mod time;

#[cfg(feature = "serde")]
mod serde;

pub use commands::SpliceNull;
pub use descriptors::*;
pub use info::{EncryptionAlgorithm, SAPType, SpliceInfoSection};
pub use time::SpliceTime;

#[derive(Error, Debug)]
#[error("Could not execute operation due to {0}")]
pub enum CueError {
    Io(#[from] io::Error),
}

pub trait ClockTimeExt {
    fn to_90k(&self) -> u64;
}

impl ClockTimeExt for u64 {
    #[inline]
    fn to_90k(&self) -> u64 {
        *self
    }
}

impl ClockTimeExt for Duration {
    #[inline]
    fn to_90k(&self) -> u64 {
        (self.as_secs_f64() * 90_000.0).floor() as u64
    }
}

trait BytesWritten {
    fn bytes_written(&self) -> u32;
}

impl BytesWritten for BitRecorder<u32, BigEndian> {
    #[inline]
    fn bytes_written(&self) -> u32 {
        self.written() / 8
    }
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
}
