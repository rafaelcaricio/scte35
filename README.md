# SCTE-35 lib and parser for Rust

> Work in progress!

This library provide access to parse and encoding of data using the SCTE-35 standard. This standard is used by
cable providers and broadcasters to insert signaling information into the video stream for advertising and
other purposes. More information can be found at
[Digital Program Insertion Cueing Message for Cable](https://www.scte.org/documents/standards/scte-35/).

## Main Features

- Parsing of SCTE-35 data
- Encoding of SCTE-35 data
- Serde integration for serialization into JSON or any other [serde supported formats](https://docs.rs/serde/1.0.137/serde/#data-formats).

## Implementation Overview

Implemented parts of the standard are:

 - [x] Splice Info section
 - Splice Commands:
   - [x] Splice Null
   - [ ] Splice Insert
   - [ ] Splice Schedule
   - [ ] Time Signal
   - [ ] Bandwidth Reservation
   - [ ] Splice Time
 - Splice Descriptors:
   - [ ] Avail
   - [ ] DTMF
   - [ ] Segmentation Descriptor
     - [ ] Cablelabs
     - [ ] MPU
     - [ ] MID
     - [ ] ADS
     - [ ] SCR
 - Encryption Information section
     - Encryption Algorithms:
       - [ ] DES – ECB mode
       - [ ] DES – CBC mode
       - [ ] Triple DES EDE3 – ECB mode
       - [ ] Customized encryption algorithm
     - [ ] CRC encryption calculation
 - [ ] CRC calculation