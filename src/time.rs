use crate::{BytesWritten, ClockTimeExt, CueError, TransportPacketWrite};
use bitstream_io::{BigEndian, BitRecorder, BitWrite, BitWriter};
#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};
use std::time::Duration;
use std::{fmt, io};

#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SpliceTime {
    time_specified_flag: bool,
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serialize_time"))]
    pts_time: u64,
}

impl SpliceTime {
    pub fn new() -> Self {
        Self {
            time_specified_flag: false,
            pts_time: 0,
        }
    }

    pub fn from_ticks(ticks: u64) -> Self {
        let mut splice_time = Self::new();
        splice_time.set_pts_time(Some(ticks));
        splice_time
    }

    #[inline]
    pub fn set_pts_time<T>(&mut self, pts_time: Option<T>)
    where
        T: ClockTimeExt,
    {
        match pts_time {
            None => {
                self.time_specified_flag = false;
                self.pts_time = 0;
            }
            Some(duration) => {
                self.time_specified_flag = true;
                self.pts_time = duration.to_90k();
            }
        }
    }

    pub fn time_specified_flag(&self) -> bool {
        self.time_specified_flag
    }

    pub fn pts_time(&self) -> Option<u64> {
        if self.time_specified_flag {
            Some(self.pts_time)
        } else {
            None
        }
    }
}

impl TransportPacketWrite for SpliceTime {
    fn write_to<W>(&self, buffer: &mut W) -> anyhow::Result<u32>
    where
        W: io::Write,
    {
        let mut recorder = BitRecorder::<u32, BigEndian>::new();

        recorder.write_bit(self.time_specified_flag)?;
        if self.time_specified_flag {
            recorder.write(6, 0x3f)?;
            recorder.write(33, self.pts_time)?;
        } else {
            recorder.write(7, 0x7f)?;
        }

        let mut buffer = BitWriter::endian(buffer, BigEndian);
        recorder.playback(&mut buffer)?;

        Ok(recorder.bytes_written())
    }
}

impl<T> From<T> for SpliceTime
    where
        T: ClockTimeExt,
{
    fn from(pts: T) -> Self {
        let mut t = Self::new();
        t.set_pts(Some(pts));
        t
    }
}

// Copyright (c) 2016 The humantime Developers
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
/// Formats duration into a human-readable string
///
/// Note: this format is guaranteed to have same value when using
/// parse_duration, but we can change some details of the exact composition
/// of the value.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use humantime::format_duration;
///
/// let val1 = Duration::new(9420, 0);
/// assert_eq!(format_duration(val1).to_string(), "2h 37m");
/// let val2 = Duration::new(0, 32_000_000);
/// assert_eq!(format_duration(val2).to_string(), "32ms");
/// ```
pub(crate) fn format_duration(val: Duration) -> FormattedDuration {
    FormattedDuration(val)
}

fn item_plural(f: &mut fmt::Formatter, started: &mut bool, name: &str, value: u64) -> fmt::Result {
    if value > 0 {
        if *started {
            f.write_str(" ")?;
        }
        write!(f, "{}{}", value, name)?;
        if value > 1 {
            f.write_str("s")?;
        }
        *started = true;
    }
    Ok(())
}
fn item(f: &mut fmt::Formatter, started: &mut bool, name: &str, value: u32) -> fmt::Result {
    if value > 0 {
        if *started {
            f.write_str(" ")?;
        }
        write!(f, "{}{}", value, name)?;
        *started = true;
    }
    Ok(())
}

/// A wrapper type that allows you to Display a Duration
#[derive(Debug, Clone)]
pub(crate) struct FormattedDuration(Duration);

impl FormattedDuration {
    /// Returns a reference to the [`Duration`][] that is being formatted.
    pub fn get_ref(&self) -> &Duration {
        &self.0
    }
}

impl fmt::Display for FormattedDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let secs = self.0.as_secs();
        let nanos = self.0.subsec_nanos();

        if secs == 0 && nanos == 0 {
            f.write_str("0s")?;
            return Ok(());
        }

        let years = secs / 31_557_600; // 365.25d
        let ydays = secs % 31_557_600;
        let months = ydays / 2_630_016; // 30.44d
        let mdays = ydays % 2_630_016;
        let days = mdays / 86400;
        let day_secs = mdays % 86400;
        let hours = day_secs / 3600;
        let minutes = day_secs % 3600 / 60;
        let seconds = day_secs % 60;

        let millis = nanos / 1_000_000;
        let micros = nanos / 1000 % 1000;
        let nanosec = nanos % 1000;

        let started = &mut false;
        item_plural(f, started, "year", years)?;
        item_plural(f, started, "month", months)?;
        item_plural(f, started, "day", days)?;
        item(f, started, "h", hours as u32)?;
        item(f, started, "m", minutes as u32)?;
        item(f, started, "s", seconds as u32)?;
        item(f, started, "milli", millis)?;
        item(f, started, "us", micros)?;
        item(f, started, "ns", nanosec)?;
        Ok(())
    }
}
