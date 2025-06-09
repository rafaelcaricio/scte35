//! Builders for SCTE-35 splice commands.

use crate::types::{SpliceInsert, TimeSignal, SpliceInsertComponent};
use crate::time::{SpliceTime, BreakDuration};
use super::error::{BuilderError, BuilderResult, DurationExt};
use std::time::Duration;

/// Builder for creating splice insert commands.
///
/// Splice insert is the most commonly used command for ad insertion points.
#[derive(Debug)]
pub struct SpliceInsertBuilder {
    splice_event_id: Option<u32>,
    out_of_network: bool,
    program_splice: bool,
    splice_immediate: bool,
    splice_time: Option<Duration>,
    components: Vec<ComponentTiming>,
    duration: Option<Duration>,
    auto_return: bool,
    unique_program_id: u16,
    avail_num: u8,
    avails_expected: u8,
}

#[derive(Clone, Debug)]
struct ComponentTiming {
    component_tag: u8,
    splice_time: Option<Duration>,
}

impl SpliceInsertBuilder {
    /// Create a new splice insert builder with the given event ID.
    pub fn new(splice_event_id: u32) -> Self {
        Self {
            splice_event_id: Some(splice_event_id),
            out_of_network: true,  // Most common case
            program_splice: true,  // Most common case
            splice_immediate: false,
            splice_time: None,
            components: Vec::new(),
            duration: None,
            auto_return: true,
            unique_program_id: 0,
            avail_num: 0,
            avails_expected: 0,
        }
    }

    /// Mark this event as cancelled.
    pub fn cancel_event(mut self) -> Self {
        self.splice_event_id = None;  // Indicates cancellation
        self
    }

    /// Set whether the splice is out of network.
    pub fn out_of_network(mut self, out: bool) -> Self {
        self.out_of_network = out;
        self
    }

    /// Set the splice to occur immediately.
    pub fn immediate(mut self) -> Self {
        self.splice_immediate = true;
        self.splice_time = None;
        self
    }

    /// Set the splice to occur at a specific PTS time.
    pub fn at_pts(mut self, pts_time: Duration) -> BuilderResult<Self> {
        self.splice_immediate = false;
        self.splice_time = Some(pts_time);
        Ok(self)
    }

    /// Configure component-level splice timing.
    pub fn component_splice(mut self, components: Vec<(u8, Option<Duration>)>) -> BuilderResult<Self> {
        if components.len() > 255 {
            return Err(BuilderError::InvalidComponentCount { max: 255, actual: components.len() });
        }
        self.program_splice = false;
        self.components = components.into_iter()
            .map(|(tag, time)| ComponentTiming { component_tag: tag, splice_time: time })
            .collect();
        Ok(self)
    }

    /// Set the duration of the break.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set whether the break should auto-return.
    pub fn auto_return(mut self, auto_return: bool) -> Self {
        self.auto_return = auto_return;
        self
    }

    /// Set the unique program ID.
    pub fn unique_program_id(mut self, id: u16) -> Self {
        self.unique_program_id = id;
        self
    }

    /// Set the avail number and expected count.
    pub fn avail(mut self, num: u8, expected: u8) -> Self {
        self.avail_num = num;
        self.avails_expected = expected;
        self
    }

    /// Build the splice insert command.
    pub fn build(self) -> BuilderResult<SpliceInsert> {
        let (splice_event_id, cancel) = match self.splice_event_id {
            Some(id) => (id, 0),
            None => (0, 1),  // Cancellation
        };

        let splice_time = if self.program_splice && !self.splice_immediate {
            let pts = match self.splice_time {
                Some(duration) => {
                    let ticks = duration.to_pts_ticks();
                    if ticks > 0x1_FFFF_FFFF {
                        return Err(BuilderError::DurationTooLarge { field: "splice_time", duration });
                    }
                    Some(ticks)
                }
                None => None,
            };
            Some(SpliceTime {
                time_specified_flag: 1,
                pts_time: pts,
            })
        } else {
            None
        };

        let mut components = Vec::new();
        if !self.program_splice {
            for c in self.components {
                let splice_time = if !self.splice_immediate {
                    let pts = match c.splice_time {
                        Some(duration) => {
                            let ticks = duration.to_pts_ticks();
                            if ticks > 0x1_FFFF_FFFF {
                                return Err(BuilderError::DurationTooLarge { field: "component_splice_time", duration });
                            }
                            Some(ticks)
                        }
                        None => None,
                    };
                    Some(SpliceTime {
                        time_specified_flag: 1,
                        pts_time: pts,
                    })
                } else {
                    None
                };
                components.push(SpliceInsertComponent {
                    component_tag: c.component_tag,
                    splice_time,
                });
            }
        }

        let break_duration = match self.duration {
            Some(duration) => {
                let ticks = duration.to_pts_ticks();
                if ticks > 0x1_FFFF_FFFF {
                    return Err(BuilderError::DurationTooLarge { field: "duration", duration });
                }
                Some(BreakDuration {
                    auto_return: self.auto_return as u8,
                    reserved: 0,
                    duration: ticks,
                })
            }
            None => None,
        };

        Ok(SpliceInsert {
            splice_event_id,
            splice_event_cancel_indicator: cancel,
            reserved: 0,
            out_of_network_indicator: self.out_of_network as u8,
            program_splice_flag: self.program_splice as u8,
            duration_flag: self.duration.is_some() as u8,
            splice_immediate_flag: self.splice_immediate as u8,
            reserved2: 0,
            splice_time,
            component_count: components.len() as u8,
            components,
            break_duration,
            unique_program_id: self.unique_program_id,
            avail_num: self.avail_num,
            avails_expected: self.avails_expected,
        })
    }
}

/// Builder for creating time signal commands.
#[derive(Debug)]
pub struct TimeSignalBuilder {
    pts_time: Option<Duration>,
}

impl TimeSignalBuilder {
    /// Create a new time signal builder.
    pub fn new() -> Self {
        Self { pts_time: None }
    }

    /// Set the time signal to occur immediately.
    pub fn immediate(self) -> Self {
        self  // No time specified
    }

    /// Set the time signal to occur at a specific PTS time.
    pub fn at_pts(mut self, pts_time: Duration) -> BuilderResult<Self> {
        self.pts_time = Some(pts_time);
        Ok(self)
    }

    /// Build the time signal command.
    pub fn build(self) -> BuilderResult<TimeSignal> {
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

        Ok(TimeSignal {
            splice_time: SpliceTime {
                time_specified_flag: pts_time.is_some() as u8,
                pts_time,
            },
        })
    }
}

impl Default for TimeSignalBuilder {
    fn default() -> Self {
        Self::new()
    }
}