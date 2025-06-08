//! Bit-level reading utilities for parsing SCTE-35 binary data.
//!
//! This module provides the `BitReader` struct which enables reading arbitrary
//! numbers of bits from a byte buffer, as required by the SCTE-35 specification.

use std::io::{self, ErrorKind};

/// A reader that can extract values at the bit level from a byte buffer.
///
/// SCTE-35 messages contain fields that are not byte-aligned, requiring
/// bit-level parsing. This reader maintains a bit offset and provides
/// methods to read various bit-width values.
pub(crate) struct BitReader<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> BitReader<'a> {
    /// Creates a new `BitReader` for the given buffer.
    ///
    /// The reader starts at bit offset 0.
    pub fn new(buffer: &'a [u8]) -> Self {
        BitReader { buffer, offset: 0 }
    }

    /// Reads a specified number of bits from the buffer.
    ///
    /// Returns the bits as a `u64`, with the read bits right-aligned.
    /// Advances the bit offset by `num_bits`.
    ///
    /// # Errors
    ///
    /// Returns an error if reading would exceed the buffer bounds.
    pub fn read_bits(&mut self, num_bits: usize) -> Result<u64, io::Error> {
        let mut value: u64 = 0;
        let mut bits_read = 0;

        while bits_read < num_bits {
            let byte_index = self.offset / 8;
            let bit_offset = self.offset % 8;

            if byte_index >= self.buffer.len() {
                return Err(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "Buffer underflow while reading bits",
                ));
            }

            let byte = self.buffer[byte_index];
            let bits_to_read = std::cmp::min(num_bits - bits_read, 8 - bit_offset);
            let mask = if bits_to_read >= 8 {
                0xFF
            } else {
                (1u8 << bits_to_read) - 1
            };
            let bits_value = (byte >> (8 - bit_offset - bits_to_read)) & mask;

            value = (value << bits_to_read) | (bits_value as u64);
            self.offset += bits_to_read;
            bits_read += bits_to_read;
        }

        Ok(value)
    }

    /// Reads an unsigned integer with a specified number of bits (MSB first).
    ///
    /// UIMSBF: Unsigned Integer, Most Significant Bit First.
    pub fn read_uimsbf(&mut self, num_bits: usize) -> Result<u64, io::Error> {
        self.read_bits(num_bits)
    }

    /// Reads a bit string with a specified number of bits (MSB first).
    ///
    /// BSLBF: Bit String, Left Bit First.
    pub fn read_bslbf(&mut self, num_bits: usize) -> Result<u64, io::Error> {
        self.read_bits(num_bits)
    }

    /// Reads a reserved field with a specified number of bits.
    ///
    /// RPCHOF: Reserved for future use, set to '1'.
    /// Note: RPCHOF typically implies LSB first within the byte, but SCTE-35 spec
    /// doesn't explicitly state this. Assuming standard MSB first based on other fields.
    pub fn read_rpchof(&mut self, num_bits: usize) -> Result<u64, io::Error> {
        self.read_bits(num_bits)
    }

    /// Skips a specified number of bits in the buffer.
    ///
    /// Advances the bit offset without reading the bits.
    ///
    /// # Errors
    ///
    /// Returns an error if skipping would exceed the buffer bounds.
    pub fn skip_bits(&mut self, num_bits: usize) -> Result<(), io::Error> {
        let new_offset = self.offset + num_bits;
        if new_offset / 8 > self.buffer.len() {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "Buffer underflow while skipping bits",
            ));
        }
        self.offset = new_offset;
        Ok(())
    }

    /// Gets the current bit offset in the buffer.
    pub fn get_offset(&self) -> usize {
        self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_reader_basic() {
        let buffer = vec![0b10101010, 0b11110000];
        let mut reader = BitReader::new(&buffer);

        // Read 4 bits: should get 0b1010 = 10
        assert_eq!(reader.read_bits(4).unwrap(), 10);

        // Read 4 more bits: should get 0b1010 = 10
        assert_eq!(reader.read_bits(4).unwrap(), 10);

        // Read 8 bits: should get 0b11110000 = 240
        assert_eq!(reader.read_bits(8).unwrap(), 240);
    }

    #[test]
    fn test_bit_reader_cross_byte() {
        let buffer = vec![0b10101010, 0b11110000];
        let mut reader = BitReader::new(&buffer);

        // Read 6 bits: should get 0b101010 = 42
        assert_eq!(reader.read_bits(6).unwrap(), 42);

        // Read 6 bits across byte boundary: should get 0b101111 = 47
        assert_eq!(reader.read_bits(6).unwrap(), 47);
    }

    #[test]
    fn test_bit_reader_skip() {
        let buffer = vec![0b10101010, 0b11110000];
        let mut reader = BitReader::new(&buffer);

        // Skip 4 bits
        reader.skip_bits(4).unwrap();

        // Read 4 bits: should get 0b1010 = 10
        assert_eq!(reader.read_bits(4).unwrap(), 10);
    }

    #[test]
    fn test_bit_reader_overflow() {
        let buffer = vec![0b10101010];
        let mut reader = BitReader::new(&buffer);

        // Try to read more bits than available
        assert!(reader.read_bits(16).is_err());
    }
}
