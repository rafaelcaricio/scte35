//! Extensions for existing types to support the builder pattern.

use crate::encoding::Encodable;
use crate::types::SpliceCommand;

/// Extension trait to provide encoding length calculation for SpliceCommand.
pub trait SpliceCommandExt {
    /// Calculate the encoded length of this splice command in bytes.
    fn encoded_length(&self) -> u16;
}

impl SpliceCommandExt for SpliceCommand {
    fn encoded_length(&self) -> u16 {
        // Use the actual Encodable trait implementation for accurate sizing
        self.encoded_size() as u16
    }
}
