# Builders Integration Tests Plan

This document tracks the comprehensive validation of our SCTE-35 builder API against real-world payloads. Each test ensures that our builders can recreate the exact same SCTE-35 messages that we can successfully parse, providing confidence that users can generate valid payloads using our API.

## Overview

We have 22 round-trip tests that validate our encoding implementation against real SCTE-35 payloads. For each test, we need to create a corresponding builder test that:

1. **Analyzes** the original payload by parsing it to understand exact parameters
2. **Builds** equivalent payload using our builder API with the same parameters  
3. **Encodes** to base64 and compares with the expected output
4. **Validates** the round-trip works correctly

This ensures complete test coverage for the builder → encoder → base64 pipeline.

## Test Categories

### High Priority Tests (19 tests)
Core builder functionality for creating real SCTE-35 payloads.

### Medium Priority Tests (3 tests)
Advanced features and validation scenarios.

## Task List

### SpliceInsert Builder Tests

#### ✅ Task 1: test_splice_insert_with_break_duration
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent payload using SpliceInsertBuilder with break duration and avail descriptor
- **Expected Base64**: `/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=`
- **Builder Components**: SpliceInsertBuilder + break duration + AvailDescriptor
- **Notes**: This is Sample 14.2 from SCTE-35 spec

#### ✅ Task 2: test_splice_insert_immediate  
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent payload using SpliceInsertBuilder immediate mode
- **Expected Base64**: Builder-generated payload (no fixed reference)
- **Builder Components**: SpliceInsertBuilder with immediate()
- **Notes**: Validates immediate splice functionality

#### ✅ Task 3: test_splice_insert_with_avail_descriptor
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent SpliceInsert with AvailDescriptor
- **Expected Base64**: `/DAqAAAAAAAAAP/wDwUAAHn+f8/+QubGOQAAAAAACgAIQ1VFSQAAAADizteX`
- **Builder Components**: SpliceInsertBuilder + AvailDescriptor
- **Notes**: Tests avail descriptor integration

#### ✅ Task 4: test_sample_14_2_splice_insert
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent SpliceInsert with break duration and avail descriptor (verification of Task 1)
- **Expected Base64**: `/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=`
- **Builder Components**: SpliceInsertBuilder + break duration + AvailDescriptor
- **Notes**: Same as Task 1 but verify exact parameter matching

### TimeSignal Builder Tests

#### ✅ Task 5: test_time_signal_immediate
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent payload using TimeSignalBuilder immediate mode
- **Expected Base64**: Builder-generated payload (no fixed reference)
- **Builder Components**: TimeSignalBuilder with immediate()
- **Notes**: Validates immediate time signal functionality

#### ✅ Task 6: test_time_signal_with_pts
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent payload using TimeSignalBuilder with PTS time
- **Expected Base64**: Builder-generated payload (no fixed reference)
- **Builder Components**: TimeSignalBuilder with at_pts()
- **Notes**: Validates PTS time specification

#### ✅ Task 7: test_time_signal_with_segmentation_descriptor
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent TimeSignal with segmentation descriptor
- **Expected Base64**: `/DAnAAAAAAAAAP/wBQb+AA27oAARAg9DVUVJAAAAAX+HCQA0AAE0xUZn`
- **Builder Components**: TimeSignalBuilder + SegmentationDescriptorBuilder
- **Notes**: Provider Placement Opportunity Start (type 0x34)

#### ✅ Task 8: test_sample_14_1_time_signal_placement_opportunity_start
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent TimeSignal with segmentation descriptor (Provider PO Start + duration + UPID)
- **Expected Base64**: `/DA0AAAAAAAA///wBQb+cr0AUAAeAhxDVUVJSAAAjn/PAAGlmbAICAAAAAAsoKGKNAIAmsnRfg==`
- **Builder Components**: TimeSignalBuilder + SegmentationDescriptorBuilder with duration and UPID
- **Notes**: Sample 14.1 from SCTE-35 spec with TI UPID

#### ✅ Task 9: test_sample_14_3_time_signal_placement_opportunity_end
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent TimeSignal with segmentation descriptor (Provider PO End)
- **Expected Base64**: `/DAvAAAAAAAA///wBQb+dGKQoAAZAhdDVUVJSAAAjn+fCAgAAAAALKChijUCAKnMZ1g=`
- **Builder Components**: TimeSignalBuilder + SegmentationDescriptorBuilder (PO End type)
- **Notes**: Sample 14.3 from SCTE-35 spec

#### ✅ Task 10: test_sample_14_4_time_signal_program_start_end
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent TimeSignal with 2 segmentation descriptors (Program End + Program Start)
- **Expected Base64**: `/DBIAAAAAAAA///wBQb+ek2ItgAyAhdDVUVJSAAAGH+fCAgAAAAALMvDRBEAAAIXQ1VFSUgAABl/nwgIAAAAACyk26AQAACZcuND`
- **Builder Components**: TimeSignalBuilder + 2x SegmentationDescriptorBuilder
- **Notes**: Sample 14.4 - tests multiple descriptors in single message

#### ✅ Task 11: test_sample_14_5_time_signal_program_overlap_start
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent TimeSignal with segmentation descriptor (Program Overlap Start)
- **Expected Base64**: `/DAvAAAAAAAA///wBQb+rr//ZAAZAhdDVUVJSAAACH+fCAgAAAAALKVs9RcAAJUdsKg=`
- **Builder Components**: TimeSignalBuilder + SegmentationDescriptorBuilder (Overlap Start type)
- **Notes**: Sample 14.5 from SCTE-35 spec

#### ✅ Task 12: test_time_signal_with_multiple_segmentation_descriptors
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent TimeSignal with 2 segmentation descriptors
- **Expected Base64**: `/DBIAAAAAAAAAP/wBQb/tB67hgAyAhdDVUVJQAABEn+fCAgAAAAALzE8BTUAAAIXQ1VFSUAAAEV/nwgIAAAAAC8xPN4jAAAfiOPE`
- **Builder Components**: TimeSignalBuilder + 2x SegmentationDescriptorBuilder
- **Notes**: Tests complex multi-descriptor scenarios

### SpliceNull Builder Tests

#### ✅ Task 13: test_splice_null
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent payload using SpliceInfoSectionBuilder with splice_null()
- **Expected Base64**: Builder-generated payload (no fixed reference)
- **Builder Components**: SpliceInfoSectionBuilder with splice_null()
- **Notes**: Basic null command validation

#### ✅ Task 14: test_splice_null_heartbeat
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent SpliceNull command (minimal heartbeat message)
- **Expected Base64**: `/DARAAAAAAAAAP/wAAAAAHpPv/8=`
- **Builder Components**: SpliceInfoSectionBuilder with splice_null()
- **Notes**: Minimal SCTE-35 message format

### Complex Scenario Tests

#### ✅ Task 15: test_segmentation_descriptor
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent payload using SpliceInsertBuilder + SegmentationDescriptorBuilder
- **Expected Base64**: Builder-generated payload (no fixed reference)
- **Builder Components**: SpliceInsertBuilder + SegmentationDescriptorBuilder
- **Notes**: Basic segmentation descriptor integration

#### ✅ Task 16: test_complex_message_multiple_descriptors
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent payload using SpliceInsertBuilder + multiple SegmentationDescriptorBuilder instances
- **Expected Base64**: Builder-generated payload (no fixed reference)
- **Builder Components**: SpliceInsertBuilder + 2x SegmentationDescriptorBuilder
- **Notes**: Tests multiple descriptors with different types

#### ✅ Task 17: test_long_segmentation_descriptor
- **Status**: Pending
- **Priority**: High
- **Description**: Build equivalent payload using SpliceInsertBuilder + SegmentationDescriptorBuilder with UPID and duration
- **Expected Base64**: Builder-generated payload (no fixed reference)
- **Builder Components**: SpliceInsertBuilder + SegmentationDescriptorBuilder with AdId UPID + duration + segment info
- **Notes**: Tests complex descriptor with all optional fields

### Manual Construction Tests (No Builders Available)

#### ✅ Task 18: test_bandwidth_reservation
- **Status**: Pending
- **Priority**: Medium
- **Description**: Build equivalent payload manually (no builder available)
- **Expected Base64**: Builder-generated payload (no fixed reference)
- **Builder Components**: Manual SpliceInfoSection construction
- **Notes**: BandwidthReservation has no builder - test manual construction

#### ✅ Task 19: test_private_command
- **Status**: Pending
- **Priority**: Medium
- **Description**: Build equivalent payload manually (no builder available)
- **Expected Base64**: Builder-generated payload (no fixed reference)
- **Builder Components**: Manual SpliceInfoSection construction
- **Notes**: PrivateCommand has no builder - test manual construction

### Validation Tests

#### ✅ Task 20: test_round_trip_with_crc_recalculation
- **Status**: Pending
- **Priority**: Medium
- **Description**: Build equivalent payload with CRC validation
- **Expected Base64**: Various test payloads
- **Builder Components**: Any builder + CRC validation
- **Notes**: Validates CRC recalculation works correctly with builders

#### ✅ Task 21: test_encoding_efficiency
- **Status**: Pending
- **Priority**: Medium
- **Description**: Build equivalent payload and validate size prediction matches actual encoded size
- **Expected Base64**: Various test payloads
- **Builder Components**: Any builder + size calculation validation
- **Notes**: Ensures encoded_size() method accuracy with builder-generated payloads

#### ✅ Task 22: test_external_tool_compatibility
- **Status**: Pending
- **Priority**: Medium
- **Description**: Build equivalent payload and validate it can be parsed by external tools
- **Expected Base64**: Various test payloads
- **Builder Components**: Any builder + external tool validation
- **Notes**: Ensures builder-generated payloads work with reference implementations

## Implementation Strategy

### Phase 1: Core Builders (Tasks 1-17)
Focus on high-priority tests that validate the main builder API functionality.

### Phase 2: Manual Construction (Tasks 18-19)
Test scenarios where builders are not available, ensuring complete coverage.

### Phase 3: Advanced Validation (Tasks 20-22)
Validate advanced features like CRC handling and external compatibility.

## File Locations

- **Test File**: `src/encoding/round_trip_tests.rs`
- **Builder Tests**: Will be added to `src/builders/tests.rs` or new dedicated test files
- **Reference Payloads**: All base64 payloads are stored in the task descriptions above

## Progress Tracking

- **Total Tasks**: 22
- **Completed**: 0
- **In Progress**: 0
- **Pending**: 22

## Notes

- Each task should include parameter analysis by parsing the original payload
- All builder-generated payloads must exactly match expected base64 outputs
- Tests should validate both the builder API usability and output correctness
- Complex scenarios with multiple descriptors are critical for real-world usage validation