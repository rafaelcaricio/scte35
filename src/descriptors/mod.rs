mod segmentation;

use crate::{CueError, TransportPacketWrite};
pub use segmentation::*;
use std::io;

#[cfg(feature = "serde")]
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum SpliceDescriptor {
    Avail,
    DTMF,
    Segmentation(SegmentationDescriptor),
    Time,
    Audio,
    Unknown(u8, u32, Vec<u8>),
}

pub(crate) trait SpliceDescriptorExt {
    fn splice_descriptor_tag(&self) -> u8;

    fn identifier(&self) -> u32 {
        0x43554549 // ASCII "CUEI"
    }
}

impl TransportPacketWrite for SpliceDescriptor {
    fn write_to<W>(&self, buffer: &mut W) -> anyhow::Result<()>
    where
        W: io::Write,
    {
        match self {
            SpliceDescriptor::Avail => unimplemented!(),
            SpliceDescriptor::DTMF => unimplemented!(),
            SpliceDescriptor::Segmentation(segmentation) => segmentation.write_to(buffer),
            SpliceDescriptor::Time => unimplemented!(),
            SpliceDescriptor::Audio => unimplemented!(),
            SpliceDescriptor::Unknown(_, _, _) => unimplemented!(),
        }
    }
}

impl From<SegmentationDescriptor> for SpliceDescriptor {
    fn from(segmentation: SegmentationDescriptor) -> Self {
        SpliceDescriptor::Segmentation(segmentation)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum SpliceDescriptorTag {
    Avail,
    DTMF,
    Segmentation,
    Time,
    Audio,
    Reserved(u8),
    DVB(u8),
}

impl From<u8> for SpliceDescriptorTag {
    fn from(value: u8) -> Self {
        match value {
            0x0 => SpliceDescriptorTag::Avail,
            0x1 => SpliceDescriptorTag::DTMF,
            0x2 => SpliceDescriptorTag::Segmentation,
            0x3 => SpliceDescriptorTag::Time,
            0x4 => SpliceDescriptorTag::Audio,
            0x5..=0xEF => SpliceDescriptorTag::Reserved(value),
            _ => SpliceDescriptorTag::DVB(value),
        }
    }
}

impl From<SpliceDescriptorTag> for u8 {
    fn from(value: SpliceDescriptorTag) -> Self {
        match value {
            SpliceDescriptorTag::Avail => 0x0,
            SpliceDescriptorTag::DTMF => 0x1,
            SpliceDescriptorTag::Segmentation => 0x2,
            SpliceDescriptorTag::Time => 0x3,
            SpliceDescriptorTag::Audio => 0x4,
            SpliceDescriptorTag::Reserved(value) => value,
            SpliceDescriptorTag::DVB(value) => value,
        }
    }
}
