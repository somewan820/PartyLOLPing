//! Deterministic, platform-independent transport primitives.
//!
//! The macOS Swift layer owns Network.framework, Bonjour, AppKit and
//! ScreenCaptureKit. This crate owns byte-oriented protocol mechanics so the
//! same implementation can be fuzzed and tested without a running app.

const HEADER_LEN: usize = 4;
const DEFAULT_MAX_FRAME: usize = 64 * 1024;

mod capture;
mod discovery;
mod domain;
mod ffi;
mod handshake;
mod media;
mod network;
mod protocol;
mod socket;

pub use capture::capture_screen_jpeg;
pub use discovery::{browse as browse_bonjour_rooms, BonjourPublisher, DiscoveredRoom};
pub use domain::{
    InviteCode, Member, MemberId, MemberRole, MemberStatus, PingEvent, PingRoute, Room, RoomError,
    RoomId, RoomState, RoutedPing,
};
pub use ffi::{
    party_engine_lan_host_create, party_engine_lan_host_destroy, party_engine_lan_join,
    party_engine_room_create, party_engine_room_create_with_identity, party_engine_room_destroy,
    party_engine_room_host_member, party_engine_room_id, party_engine_room_invite,
    party_engine_room_join, party_engine_room_leave, party_engine_room_ping, PartyEngineByteSlice,
    PartyEngineRoom, PARTY_ENGINE_ERR_BUFFER_TOO_SMALL, PARTY_ENGINE_ERR_CAPACITY,
    PARTY_ENGINE_ERR_DUPLICATE_EVENT, PARTY_ENGINE_ERR_INVALID_ID, PARTY_ENGINE_ERR_INVALID_INVITE,
    PARTY_ENGINE_ERR_INVALID_NAME, PARTY_ENGINE_ERR_INVALID_TARGET,
    PARTY_ENGINE_ERR_MEMBER_NOT_FOUND, PARTY_ENGINE_ERR_NULL, PARTY_ENGINE_ERR_ROOM_ENDED,
    PARTY_ENGINE_ERR_SENDER_OFFLINE, PARTY_ENGINE_ERR_UTF8, PARTY_ENGINE_OK,
};
pub use handshake::{Handshake, HandshakeError, HandshakeMessage, HandshakePhase};
pub use media::{MediaTicket, MediaTicketError};
pub use network::{connect as lan_connect, join as lan_join, LanClient, LanHost};
pub use protocol::{ControlMessage, Envelope, ProtocolError, ProtocolVersion};
pub use socket::{
    party_engine_socket_accept, party_engine_socket_close, party_engine_socket_connect,
    party_engine_socket_listen, party_engine_socket_listener_close,
    party_engine_socket_listener_port, party_engine_socket_read, party_engine_socket_write,
};

#[derive(Debug, PartialEq, Eq)]
pub enum FrameError {
    Empty,
    TooLarge,
}

pub fn encode_frame(payload: &[u8], max_frame_length: usize) -> Result<Vec<u8>, FrameError> {
    if payload.is_empty() {
        return Err(FrameError::Empty);
    }
    if payload.len() > max_frame_length {
        return Err(FrameError::TooLarge);
    }

    let length = payload.len() as u32;
    let mut frame = Vec::with_capacity(HEADER_LEN + payload.len());
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

/// Returns the payload length for one complete frame.
///
/// Return values:
/// * `>= 0`: complete frame payload length
/// * `-1`: incomplete frame
/// * `-2`: empty/invalid frame
/// * `-3`: frame exceeds the configured maximum
#[no_mangle]
pub unsafe extern "C" fn party_engine_frame_payload_length(
    frame: *const u8,
    frame_len: usize,
    max_frame_length: usize,
) -> i32 {
    if frame.is_null() {
        return -2;
    }
    if frame_len < HEADER_LEN {
        return -1;
    }
    let bytes = std::slice::from_raw_parts(frame, frame_len);
    let length = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    if length == 0 {
        return -2;
    }
    if length > max_frame_length {
        return -3;
    }
    if frame_len < HEADER_LEN + length {
        return -1;
    }
    if length > i32::MAX as usize {
        return -3;
    }
    length as i32
}

/// Copies the payload of one complete frame into a caller-owned buffer.
/// Returns the number of bytes copied, or a negative error code matching
/// `party_engine_frame_payload_length`.
#[no_mangle]
pub unsafe extern "C" fn party_engine_copy_payload(
    frame: *const u8,
    frame_len: usize,
    max_frame_length: usize,
    output: *mut u8,
    output_len: usize,
) -> i32 {
    let payload_len = party_engine_frame_payload_length(frame, frame_len, max_frame_length);
    if payload_len < 0 {
        return payload_len;
    }
    let payload_len = payload_len as usize;
    if output.is_null() || output_len < payload_len {
        return -2;
    }
    let bytes = std::slice::from_raw_parts(frame, frame_len);
    std::ptr::copy_nonoverlapping(bytes.as_ptr().add(HEADER_LEN), output, payload_len);
    payload_len as i32
}

/// Returns 1 only when the next sequence number is exactly the expected one.
/// Keeping this check in Rust gives the protocol state machine one deterministic
/// implementation for native and future non-Swift adapters.
#[no_mangle]
pub extern "C" fn party_engine_sequence_is_expected(expected: u64, actual: u64) -> u8 {
    u8::from(expected == actual)
}

#[no_mangle]
pub extern "C" fn party_engine_should_retry(attempt: i32, available_delays: i32) -> u8 {
    u8::from(attempt >= 0 && attempt < available_delays)
}

/// Connection states/events shared by the Swift Network.framework adapter.
/// Return value is the next state, or -1 for an invalid transition.
#[no_mangle]
pub extern "C" fn party_engine_connection_transition(state: i32, event: i32) -> i32 {
    const IDLE: i32 = 0;
    const PREPARING: i32 = 1;
    const READY: i32 = 2;
    const WAITING: i32 = 3;
    const FAILED: i32 = 4;
    const CLOSED: i32 = 5;
    const SETUP: i32 = 0;
    const PREPARE: i32 = 1;
    const READY_EVENT: i32 = 2;
    const WAIT: i32 = 3;
    const FAIL: i32 = 4;
    const CANCEL: i32 = 5;

    if state == CLOSED || state == FAILED {
        return match event {
            FAIL => FAILED,
            CANCEL => CLOSED,
            _ => -1,
        };
    }
    match event {
        SETUP => IDLE,
        PREPARE => PREPARING,
        READY_EVENT => READY,
        WAIT => WAITING,
        FAIL => FAILED,
        CANCEL => CLOSED,
        _ => -1,
    }
}

#[no_mangle]
pub extern "C" fn party_engine_handshake_transition(state: i32, event: i32) -> i32 {
    const IDLE: i32 = 0;
    const HELLO: i32 = 1;
    const CHALLENGE: i32 = 2;
    const ESTABLISHED: i32 = 3;
    const JOINED: i32 = 4;
    const REJECTED: i32 = 5;
    const HELLO_EVENT: i32 = 0;
    const CHALLENGE_EVENT: i32 = 1;
    const JOIN_EVENT: i32 = 2;
    const ACCEPTED_EVENT: i32 = 3;
    const REJECT_EVENT: i32 = 4;

    if state == JOINED || state == REJECTED {
        return -1;
    }
    if event == REJECT_EVENT {
        return REJECTED;
    }
    match (state, event) {
        (IDLE, HELLO_EVENT) => HELLO,
        (HELLO, CHALLENGE_EVENT) => CHALLENGE,
        (CHALLENGE, JOIN_EVENT) => ESTABLISHED,
        (ESTABLISHED, ACCEPTED_EVENT) => JOINED,
        _ => -1,
    }
}

#[derive(Debug)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    max_frame_length: usize,
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_FRAME)
    }
}

impl FrameDecoder {
    pub fn new(max_frame_length: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_frame_length,
        }
    }

    /// Adds an arbitrary network chunk and returns every complete payload.
    /// Partial headers and payloads remain buffered for the next call.
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<Vec<u8>>, FrameError> {
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();

        loop {
            if self.buffer.len() < HEADER_LEN {
                break;
            }

            let length = u32::from_be_bytes([
                self.buffer[0],
                self.buffer[1],
                self.buffer[2],
                self.buffer[3],
            ]) as usize;

            if length == 0 {
                return Err(FrameError::Empty);
            }
            if length > self.max_frame_length {
                return Err(FrameError::TooLarge);
            }

            let total = HEADER_LEN + length;
            if self.buffer.len() < total {
                break;
            }

            let payload = self.buffer[HEADER_LEN..total].to_vec();
            self.buffer.drain(..total);
            frames.push(payload);
        }

        Ok(frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_split_header_and_payload() {
        let encoded = encode_frame(b"hello", DEFAULT_MAX_FRAME).unwrap();
        let mut decoder = FrameDecoder::default();
        assert!(decoder.push(&encoded[..2]).unwrap().is_empty());
        assert!(decoder.push(&encoded[2..6]).unwrap().is_empty());
        assert_eq!(
            decoder.push(&encoded[6..]).unwrap(),
            vec![b"hello".to_vec()]
        );
    }

    #[test]
    fn decodes_multiple_frames_in_one_chunk() {
        let mut bytes = encode_frame(b"one", DEFAULT_MAX_FRAME).unwrap();
        bytes.extend_from_slice(&encode_frame(b"two", DEFAULT_MAX_FRAME).unwrap());
        let mut decoder = FrameDecoder::default();
        assert_eq!(
            decoder.push(&bytes).unwrap(),
            vec![b"one".to_vec(), b"two".to_vec()]
        );
    }

    #[test]
    fn rejects_empty_and_oversized_frames() {
        assert_eq!(encode_frame(b"", DEFAULT_MAX_FRAME), Err(FrameError::Empty));
        let mut decoder = FrameDecoder::new(3);
        assert_eq!(decoder.push(&[0, 0, 0, 4]), Err(FrameError::TooLarge));
    }

    #[test]
    fn accepts_only_the_expected_sequence() {
        assert_eq!(party_engine_sequence_is_expected(7, 7), 1);
        assert_eq!(party_engine_sequence_is_expected(7, 6), 0);
        assert_eq!(party_engine_sequence_is_expected(7, 8), 0);
    }

    #[test]
    fn retry_is_bounded_by_available_delays() {
        assert_eq!(party_engine_should_retry(0, 5), 1);
        assert_eq!(party_engine_should_retry(4, 5), 1);
        assert_eq!(party_engine_should_retry(5, 5), 0);
        assert_eq!(party_engine_should_retry(-1, 5), 0);
    }

    #[test]
    fn terminal_connection_states_cannot_reopen() {
        assert_eq!(party_engine_connection_transition(4, 2), -1);
        assert_eq!(party_engine_connection_transition(5, 2), -1);
        assert_eq!(party_engine_connection_transition(4, 5), 5);
    }

    #[test]
    fn handshake_cannot_skip_or_repeat_phases() {
        assert_eq!(party_engine_handshake_transition(0, 0), 1);
        assert_eq!(party_engine_handshake_transition(1, 1), 2);
        assert_eq!(party_engine_handshake_transition(2, 2), 3);
        assert_eq!(party_engine_handshake_transition(3, 3), 4);
        assert_eq!(party_engine_handshake_transition(1, 2), -1);
        assert_eq!(party_engine_handshake_transition(4, 0), -1);
    }
}
