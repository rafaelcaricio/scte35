//! Bit-level writer for encoding binary data.

use super::error::{EncodingError, EncodingResult};

/// A writer that can write individual bits to a byte buffer.
///
/// This is the encoding counterpart to `BitReader`, handling the complexity
/// of writing arbitrary bit-width values across byte boundaries.
pub struct BitWriter {
    /// The output buffer.
    buffer: Vec<u8>,
    /// Current bit position within the current byte (0-7).
    bit_position: u8,
    /// Current byte being written.
    current_byte: u8,
}

impl BitWriter {
    /// Creates a new `BitWriter` with an empty buffer.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            bit_position: 0,
            current_byte: 0,
        }
    }
    
    /// Creates a new `BitWriter` with a pre-allocated buffer capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            bit_position: 0,
            current_byte: 0,
        }
    }
    
    /// Writes a value using the specified number of bits.
    ///
    /// # Arguments
    /// * `value` - The value to write
    /// * `bits` - Number of bits to write (1-64)
    ///
    /// # Errors
    /// Returns an error if `bits` is 0 or greater than 64.
    pub fn write_bits(&mut self, value: u64, bits: u8) -> EncodingResult<()> {
        if bits == 0 || bits > 64 {
            return Err(EncodingError::InvalidFieldValue {
                field: "bits",
                value: bits.to_string(),
            });
        }
        
        // Mask the value to ensure we only use the specified number of bits
        let masked_value = if bits == 64 {
            value
        } else {
            value & ((1u64 << bits) - 1)
        };
        
        let mut remaining_bits = bits;
        let mut value_to_write = masked_value;
        
        while remaining_bits > 0 {
            let bits_available_in_current_byte = 8 - self.bit_position;
            let bits_to_write = remaining_bits.min(bits_available_in_current_byte);
            
            // Shift the value to get the bits we want to write
            let shift_amount = remaining_bits - bits_to_write;
            let bits_value = (value_to_write >> shift_amount) as u8;
            let mask = ((1u16 << bits_to_write) - 1) as u8;
            
            // Write bits to current byte
            self.current_byte |= (bits_value & mask) << (bits_available_in_current_byte - bits_to_write);
            self.bit_position += bits_to_write;
            
            // If we've filled the current byte, add it to the buffer
            if self.bit_position == 8 {
                self.buffer.push(self.current_byte);
                self.current_byte = 0;
                self.bit_position = 0;
            }
            
            remaining_bits -= bits_to_write;
            value_to_write &= (1u64 << shift_amount) - 1;
        }
        
        Ok(())
    }
    
    /// Writes a single bit.
    pub fn write_bit(&mut self, bit: bool) -> EncodingResult<()> {
        self.write_bits(if bit { 1 } else { 0 }, 1)
    }
    
    /// Writes a complete byte array.
    pub fn write_bytes(&mut self, bytes: &[u8]) -> EncodingResult<()> {
        for &byte in bytes {
            self.write_bits(byte as u64, 8)?;
        }
        Ok(())
    }
    
    /// Aligns to the next byte boundary by padding with zeros if necessary.
    pub fn align_to_byte(&mut self) -> EncodingResult<()> {
        if self.bit_position > 0 {
            let padding_bits = 8 - self.bit_position;
            self.write_bits(0, padding_bits)?;
        }
        Ok(())
    }
    
    /// Finishes writing and returns the complete buffer.
    ///
    /// This will pad the last byte with zeros if necessary.
    pub fn finish(mut self) -> Vec<u8> {
        if self.bit_position > 0 {
            self.buffer.push(self.current_byte);
        }
        self.buffer
    }
    
    /// Returns the current size of the buffer in bytes.
    ///
    /// Note: This includes any partially written byte.
    pub fn len(&self) -> usize {
        self.buffer.len() + if self.bit_position > 0 { 1 } else { 0 }
    }
    
    /// Returns true if no bits have been written yet.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty() && self.bit_position == 0
    }
    
    /// Returns the current bit position within the current byte.
    pub fn bit_position(&self) -> u8 {
        self.bit_position
    }
}

impl Default for BitWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_write_single_byte() {
        let mut writer = BitWriter::new();
        writer.write_bits(0xAB, 8).unwrap();
        let buffer = writer.finish();
        assert_eq!(buffer, vec![0xAB]);
    }
    
    #[test]
    fn test_write_bits_across_bytes() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b101, 3).unwrap(); // 101
        writer.write_bits(0b11001, 5).unwrap(); // 11001
        writer.write_bits(0b0110, 4).unwrap(); // 0110
        writer.write_bits(0b1111, 4).unwrap(); // 1111
        let buffer = writer.finish();
        // Should produce: 10111001 01101111
        assert_eq!(buffer, vec![0b10111001, 0b01101111]);
    }
    
    #[test]
    fn test_write_bit() {
        let mut writer = BitWriter::new();
        writer.write_bit(true).unwrap();
        writer.write_bit(false).unwrap();
        writer.write_bit(true).unwrap();
        writer.write_bit(true).unwrap();
        writer.write_bit(false).unwrap();
        writer.write_bit(true).unwrap();
        writer.write_bit(false).unwrap();
        writer.write_bit(true).unwrap();
        let buffer = writer.finish();
        assert_eq!(buffer, vec![0b10110101]);
    }
    
    #[test]
    fn test_align_to_byte() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b101, 3).unwrap();
        writer.align_to_byte().unwrap();
        writer.write_bits(0xFF, 8).unwrap();
        let buffer = writer.finish();
        assert_eq!(buffer, vec![0b10100000, 0xFF]);
    }
    
    #[test]
    fn test_write_bytes() {
        let mut writer = BitWriter::new();
        writer.write_bytes(&[0xAB, 0xCD, 0xEF]).unwrap();
        let buffer = writer.finish();
        assert_eq!(buffer, vec![0xAB, 0xCD, 0xEF]);
    }
    
    #[test]
    fn test_partial_byte_finish() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b10110, 5).unwrap();
        let buffer = writer.finish();
        // Should pad with zeros: 10110000
        assert_eq!(buffer, vec![0b10110000]);
    }
    
    #[test]
    fn test_value_masking() {
        let mut writer = BitWriter::new();
        // Write a value that's larger than can fit in 4 bits
        writer.write_bits(0xFF, 4).unwrap();
        let buffer = writer.finish();
        // Should only write the lower 4 bits: 1111
        assert_eq!(buffer, vec![0b11110000]);
    }
    
    #[test]
    fn test_invalid_bits() {
        let mut writer = BitWriter::new();
        assert!(writer.write_bits(0, 0).is_err());
        assert!(writer.write_bits(0, 65).is_err());
    }
}