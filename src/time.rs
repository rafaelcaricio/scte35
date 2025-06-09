//! Time-related structures and utilities for SCTE-35 messages.
//!
//! This module contains structures for representing time information in SCTE-35,
//! including splice times, durations, and date/time values.

use std::time::Duration;

/// Represents a splice time with optional PTS (Presentation Time Stamp).
///
/// Used to indicate when a splice should occur, either immediately or at a specific time.
#[derive(Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct SpliceTime {
    /// Indicates whether a specific time is provided (1 = time specified, 0 = immediate)
    pub time_specified_flag: u8,
    /// Presentation timestamp in 90kHz ticks (present when time_specified_flag = 1)
    pub pts_time: Option<u64>,
}

impl SpliceTime {
    /// Converts the PTS time to a [`std::time::Duration`].
    ///
    /// PTS (Presentation Time Stamp) values are stored as 90kHz ticks in SCTE-35 messages.
    /// This method converts those ticks to a standard Rust Duration.
    ///
    /// # Returns
    ///
    /// - `Some(Duration)` if a PTS time is specified
    /// - `None` if no time is specified (time_specified_flag is 0)
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35_parsing::SpliceTime;
    /// use std::time::Duration;
    ///
    /// let splice_time = SpliceTime {
    ///     time_specified_flag: 1,
    ///     pts_time: Some(90_000), // 1 second in 90kHz ticks
    /// };
    ///
    /// let duration = splice_time.to_duration().unwrap();
    /// assert_eq!(duration, Duration::from_secs(1));
    /// ```
    pub fn to_duration(&self) -> Option<Duration> {
        self.pts_time.map(|pts| {
            let seconds = pts / 90_000;
            let nanos = ((pts % 90_000) * 1_000_000_000) / 90_000;
            Duration::new(seconds, nanos as u32)
        })
    }
}


/// Represents the duration of a commercial break or other timed segment.
///
/// The duration is specified in 90kHz ticks and can optionally indicate
/// whether the break should automatically return to normal programming.
#[derive(Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct BreakDuration {
    /// Indicates if the break should automatically return to network programming (1 = auto return, 0 = no auto return)
    pub auto_return: u8,
    /// Reserved bits for future use
    pub reserved: u8,
    /// Duration of the break in 90kHz ticks
    pub duration: u64,
}

impl BreakDuration {
    /// Converts the break duration to a [`std::time::Duration`].
    ///
    /// Break durations are stored as 90kHz ticks in SCTE-35 messages.
    /// This method converts those ticks to a standard Rust Duration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35_parsing::BreakDuration;
    /// use std::time::Duration;
    ///
    /// let break_duration = BreakDuration {
    ///     auto_return: 1,
    ///     reserved: 0,
    ///     duration: 2_700_000, // 30 seconds in 90kHz ticks
    /// };
    ///
    /// let duration = break_duration.to_duration();
    /// assert_eq!(duration, Duration::from_secs(30));
    /// ```
    pub fn to_duration(&self) -> Duration {
        let seconds = self.duration / 90_000;
        let nanos = ((self.duration % 90_000) * 1_000_000_000) / 90_000;
        Duration::new(seconds, nanos as u32)
    }
}

impl From<BreakDuration> for Duration {
    fn from(break_duration: BreakDuration) -> Self {
        break_duration.to_duration()
    }
}

impl From<&BreakDuration> for Duration {
    fn from(break_duration: &BreakDuration) -> Self {
        break_duration.to_duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splice_time_to_duration() {
        // Test with time specified
        let splice_time = SpliceTime {
            time_specified_flag: 1,
            pts_time: Some(90_000), // 1 second
        };
        assert_eq!(splice_time.to_duration(), Some(Duration::from_secs(1)));

        // Test with no time specified
        let splice_time = SpliceTime {
            time_specified_flag: 0,
            pts_time: None,
        };
        assert_eq!(splice_time.to_duration(), None);

        // Test with fractional seconds
        let splice_time = SpliceTime {
            time_specified_flag: 1,
            pts_time: Some(135_000), // 1.5 seconds
        };
        assert_eq!(splice_time.to_duration(), Some(Duration::from_millis(1500)));
    }

    #[test]
    fn test_break_duration_to_duration() {
        let break_duration = BreakDuration {
            auto_return: 1,
            reserved: 0,
            duration: 2_700_000, // 30 seconds
        };
        assert_eq!(break_duration.to_duration(), Duration::from_secs(30));

        // Test From trait implementations
        let duration: Duration = break_duration.into();
        assert_eq!(duration, Duration::from_secs(30));

        let break_duration_ref = &BreakDuration {
            auto_return: 0,
            reserved: 0,
            duration: 450_000, // 5 seconds
        };
        let duration: Duration = break_duration_ref.into();
        assert_eq!(duration, Duration::from_secs(5));
    }
}
