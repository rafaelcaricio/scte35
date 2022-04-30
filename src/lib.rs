use std::io;

mod commands;
mod descriptors;
mod info;

pub trait TransportPacketWrite {
    fn write_to<W>(&self, buffer: &mut W) -> Result<(), CueError>
    where
        W: io::Write;
}

#[derive(Debug)]
pub enum CueError {
    Io(io::Error),
}

impl From<io::Error> for CueError {
    fn from(err: io::Error) -> CueError {
        CueError::Io(err)
    }
}
