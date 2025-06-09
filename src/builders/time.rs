//! Time-related builder utilities for SCTE-35 messages.

use crate::time::{SpliceTime, BreakDuration, DateTime};
use super::error::{BuilderError, BuilderResult, DurationExt};
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
                    return Err(BuilderError::DurationTooLarge { field: "pts_time", duration });
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
            return Err(BuilderError::DurationTooLarge { field: "duration", duration: self.duration });
        }

        Ok(BreakDuration {
            auto_return: self.auto_return as u8,
            reserved: 0,
            duration: ticks,
        })
    }
}

/// Builder for creating date/time structures.
#[derive(Debug)]
pub struct DateTimeBuilder {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    utc_flag: bool,
}

impl DateTimeBuilder {
    /// Create a new date/time builder with the specified date and time.
    pub fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> BuilderResult<Self> {
        if month == 0 || month > 12 {
            return Err(BuilderError::InvalidValue {
                field: "month",
                reason: "Month must be 1-12".to_string(),
            });
        }
        if day == 0 || day > 31 {
            return Err(BuilderError::InvalidValue {
                field: "day",
                reason: "Day must be 1-31".to_string(),
            });
        }
        if hour > 23 {
            return Err(BuilderError::InvalidValue {
                field: "hour",
                reason: "Hour must be 0-23".to_string(),
            });
        }
        if minute > 59 {
            return Err(BuilderError::InvalidValue {
                field: "minute",
                reason: "Minute must be 0-59".to_string(),
            });
        }
        if second > 59 {
            return Err(BuilderError::InvalidValue {
                field: "second",
                reason: "Second must be 0-59".to_string(),
            });
        }

        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            utc_flag: false,
        })
    }

    /// Set whether this time is in UTC.
    pub fn utc(mut self, utc: bool) -> Self {
        self.utc_flag = utc;
        self
    }

    /// Build the date/time structure.
    pub fn build(self) -> DateTime {
        DateTime {
            utc_flag: self.utc_flag as u8,
            year: self.year,
            month: self.month,
            day: self.day,
            hour: self.hour,
            minute: self.minute,
            second: self.second,
            frames: 0,
            milliseconds: 0,
        }
    }
}