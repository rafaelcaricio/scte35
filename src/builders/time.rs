//! Time-related builder utilities for SCTE-35 messages.

use super::error::{BuilderError, BuilderResult, DurationExt};
use crate::time::{BreakDuration, SpliceTime};
use std::time::Duration;

/// Builder for creating splice time structures.
#[derive(Debug)]
pub struct SpliceTimeBuilder {
    pts_time: Option<Duration>,
}

impl SpliceTimeBuilder {
    /// Create a new splice time builder.
    pub fn new() -> Self {
        Self { pts_time: None }
    }

    /// Set the splice time to be immediate (no PTS specified).
    pub fn immediate(mut self) -> Self {
        self.pts_time = None;
        self
    }

    /// Set the splice time to occur at a specific PTS time.
    pub fn at_pts(mut self, pts_time: Duration) -> BuilderResult<Self> {
        self.pts_time = Some(pts_time);
        Ok(self)
    }

    /// Build the splice time structure.
    pub fn build(self) -> BuilderResult<SpliceTime> {
        let pts_time = match self.pts_time {
            Some(duration) => {
                let ticks = duration.to_pts_ticks();
                if ticks > 0x1_FFFF_FFFF {
                    return Err(BuilderError::DurationTooLarge {
                        field: "pts_time",
                        duration,
                    });
                }
                Some(ticks)
            }
            None => None,
        };

        Ok(SpliceTime {
            time_specified_flag: pts_time.is_some() as u8,
            pts_time,
        })
    }
}

impl Default for SpliceTimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating break duration structures.
#[derive(Debug)]
pub struct BreakDurationBuilder {
    duration: Duration,
    auto_return: bool,
}

impl BreakDurationBuilder {
    /// Create a new break duration builder with the specified duration.
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            auto_return: true,
        }
    }

    /// Set whether the break should auto-return.
    pub fn auto_return(mut self, auto_return: bool) -> Self {
        self.auto_return = auto_return;
        self
    }

    /// Build the break duration structure.
    pub fn build(self) -> BuilderResult<BreakDuration> {
        let ticks = self.duration.to_pts_ticks();
        if ticks > 0x1_FFFF_FFFF {
            return Err(BuilderError::DurationTooLarge {
                field: "duration",
                duration: self.duration,
            });
        }

        Ok(BreakDuration {
            auto_return: self.auto_return as u8,
            reserved: 0,
            duration: ticks,
        })
    }
}
