use std::io;
use thiserror::Error;

mod commands;
mod descriptors;
mod info;

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
