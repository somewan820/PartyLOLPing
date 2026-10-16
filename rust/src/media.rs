use crate::domain::{MemberId, RoomId};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaTicket {
    pub room_id: RoomId,
    pub room_epoch: u64,
    pub sharer: MemberId,
    pub viewer: MemberId,
    pub stream_id: String,
    pub token: Vec<u8>,
    pub issued_at_ms: u64,
    pub ttl_ms: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MediaTicketError {
    EmptyToken,
    Expired,
    WrongRoom,
    WrongEpoch,
    WrongPeer,
    WrongStream,
}
impl MediaTicket {
    pub fn validate(
        &self,
        room: &RoomId,
        epoch: u64,
        sharer: &MemberId,
        viewer: &MemberId,
        stream: &str,
        now_ms: u64,
    ) -> Result<(), MediaTicketError> {
        if self.token.is_empty() {
            return Err(MediaTicketError::EmptyToken);
        }
        if &self.room_id != room {
            return Err(MediaTicketError::WrongRoom);
        }
        if self.room_epoch != epoch {
            return Err(MediaTicketError::WrongEpoch);
        }
        if &self.sharer != sharer || &self.viewer != viewer {
            return Err(MediaTicketError::WrongPeer);
        }
        if self.stream_id != stream {
            return Err(MediaTicketError::WrongStream);
        }
        let expires = self.issued_at_ms.saturating_add(self.ttl_ms);
        if now_ms > expires {
            return Err(MediaTicketError::Expired);
        }
        Ok(())
    }
    pub fn new(
        room_id: RoomId,
        epoch: u64,
        sharer: MemberId,
        viewer: MemberId,
        stream_id: impl Into<String>,
        token: Vec<u8>,
        ttl: Duration,
    ) -> Self {
        Self {
            room_id,
            room_epoch: epoch,
            sharer,
            viewer,
            stream_id: stream_id.into(),
            token,
            issued_at_ms: now_ms(),
            ttl_ms: ttl.as_millis().min(u64::MAX as u128) as u64,
        }
    }
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{MemberId, RoomId};

    #[test]
    fn ticket_is_bound_to_room_epoch_peers_and_stream() {
        let room = RoomId::new("room").unwrap();
        let sharer = MemberId::new("sharer").unwrap();
        let viewer = MemberId::new("viewer").unwrap();
        let ticket = MediaTicket::new(
            room.clone(),
            3,
            sharer.clone(),
            viewer.clone(),
            "stream",
            vec![1, 2],
            Duration::from_secs(30),
        );
        assert!(ticket
            .validate(&room, 3, &sharer, &viewer, "stream", ticket.issued_at_ms)
            .is_ok());
        assert_eq!(
            ticket.validate(&room, 2, &sharer, &viewer, "stream", ticket.issued_at_ms),
            Err(MediaTicketError::WrongEpoch)
        );
        assert_eq!(
            ticket.validate(&room, 3, &viewer, &sharer, "stream", ticket.issued_at_ms),
            Err(MediaTicketError::WrongPeer)
        );
        assert_eq!(
            ticket.validate(&room, 3, &sharer, &viewer, "other", ticket.issued_at_ms),
            Err(MediaTicketError::WrongStream)
        );
        assert_eq!(
            ticket.validate(
                &room,
                3,
                &sharer,
                &viewer,
                "stream",
                ticket.issued_at_ms + 31_000
            ),
            Err(MediaTicketError::Expired)
        );
    }
}
