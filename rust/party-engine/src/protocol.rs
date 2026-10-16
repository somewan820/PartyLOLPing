//! Versioned, typed control messages. Encoding is intentionally kept at the
//! Swift/serde boundary; callers cannot accidentally send arbitrary JSON here.

use crate::domain::{MemberId, PingEvent, RoomId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolVersion(pub u16);
impl ProtocolVersion {
    pub const CURRENT: Self = Self(1);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlMessage {
    Hello {
        nonce: Vec<u8>,
    },
    Challenge {
        nonce: Vec<u8>,
        epoch: u64,
    },
    JoinRequest {
        invite: String,
        display_name: String,
    },
    JoinAccepted {
        member_id: MemberId,
        epoch: u64,
    },
    JoinRejected {
        reason: u16,
    },
    Ping(PingEvent),
    Heartbeat {
        sent_at_ms: u64,
    },
    HeartbeatAck {
        sent_at_ms: u64,
    },
    RoomEnded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    pub version: ProtocolVersion,
    pub message_id: String,
    pub room_id: RoomId,
    pub sender_id: MemberId,
    pub sequence: u64,
    pub sent_at_ms: u64,
    pub body: ControlMessage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    UnsupportedVersion,
    EmptyMessageId,
    OutOfOrder,
    WrongRoom,
    WrongSender,
}

impl Envelope {
    pub fn validate(
        &self,
        expected_room: &RoomId,
        expected_sender: &MemberId,
        last_sequence: u64,
    ) -> Result<(), ProtocolError> {
        if self.version != ProtocolVersion::CURRENT {
            return Err(ProtocolError::UnsupportedVersion);
        }
        if self.message_id.is_empty() {
            return Err(ProtocolError::EmptyMessageId);
        }
        if &self.room_id != expected_room {
            return Err(ProtocolError::WrongRoom);
        }
        if &self.sender_id != expected_sender {
            return Err(ProtocolError::WrongSender);
        }
        if self.sequence != last_sequence.saturating_add(1) {
            return Err(ProtocolError::OutOfOrder);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MemberId, RoomId};

    #[test]
    fn envelope_requires_monotonic_sequence_and_matching_scope() {
        let room = RoomId::new("room").unwrap();
        let sender = MemberId::new("member").unwrap();
        let envelope = Envelope {
            version: ProtocolVersion::CURRENT,
            message_id: "message".into(),
            room_id: room.clone(),
            sender_id: sender.clone(),
            sequence: 1,
            sent_at_ms: 10,
            body: ControlMessage::Heartbeat { sent_at_ms: 10 },
        };
        assert!(envelope.validate(&room, &sender, 0).is_ok());
        assert_eq!(
            envelope.validate(&room, &sender, 1),
            Err(ProtocolError::OutOfOrder)
        );
        assert_eq!(
            envelope.validate(&RoomId::new("other").unwrap(), &sender, 0),
            Err(ProtocolError::WrongRoom)
        );
    }
}
