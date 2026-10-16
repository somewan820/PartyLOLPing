use crate::domain::{InviteCode, MemberId, RoomId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandshakePhase {
    Idle,
    HelloSent,
    Challenged,
    Established,
    Rejected,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandshakeMessage {
    Hello {
        nonce: Vec<u8>,
    },
    Challenge {
        room_id: RoomId,
        epoch: u64,
        nonce: Vec<u8>,
    },
    Join {
        invite: InviteCode,
        display_name: String,
    },
    Accepted {
        member_id: MemberId,
    },
    Rejected,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandshakeError {
    InvalidTransition,
    InvalidInvite,
    MissingChallenge,
}

pub struct Handshake {
    pub phase: HandshakePhase,
    pub room_id: RoomId,
    pub epoch: u64,
}
impl Handshake {
    pub fn new(room_id: RoomId, epoch: u64) -> Self {
        Self {
            phase: HandshakePhase::Idle,
            room_id,
            epoch,
        }
    }
    pub fn apply(
        &mut self,
        message: &HandshakeMessage,
        expected_invite: Option<&InviteCode>,
    ) -> Result<(), HandshakeError> {
        match (&self.phase, message) {
            (HandshakePhase::Idle, HandshakeMessage::Hello { .. }) => {
                self.phase = HandshakePhase::HelloSent
            }
            (HandshakePhase::HelloSent, HandshakeMessage::Challenge { room_id, epoch, .. })
                if room_id == &self.room_id && *epoch == self.epoch =>
            {
                self.phase = HandshakePhase::Challenged
            }
            (HandshakePhase::Challenged, HandshakeMessage::Join { invite, .. }) => {
                if expected_invite != Some(invite) {
                    return Err(HandshakeError::InvalidInvite);
                }
                self.phase = HandshakePhase::Established;
            }
            (HandshakePhase::Established, HandshakeMessage::Accepted { .. }) => {}
            (_, HandshakeMessage::Rejected) => self.phase = HandshakePhase::Rejected,
            _ => return Err(HandshakeError::InvalidTransition),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_requires_hello_challenge_and_invite() {
        let room = RoomId::new("room").unwrap();
        let invite = InviteCode::new("123456").unwrap();
        let mut h = Handshake::new(room.clone(), 7);
        assert!(h
            .apply(&HandshakeMessage::Hello { nonce: vec![1] }, None)
            .is_ok());
        assert!(h
            .apply(
                &HandshakeMessage::Challenge {
                    room_id: room,
                    epoch: 7,
                    nonce: vec![2]
                },
                None
            )
            .is_ok());
        assert_eq!(
            h.apply(
                &HandshakeMessage::Join {
                    invite: InviteCode::new("654321").unwrap(),
                    display_name: "x".into()
                },
                Some(&invite)
            ),
            Err(HandshakeError::InvalidInvite)
        );
        assert!(h
            .apply(
                &HandshakeMessage::Join {
                    invite,
                    display_name: "x".into()
                },
                Some(&InviteCode::new("123456").unwrap())
            )
            .is_ok());
    }
}
