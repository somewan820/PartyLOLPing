//! In-memory room facts and routing rules.  This module deliberately has no
//! transport or platform dependencies; the Swift adapter can own one `Room`.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, RoomError> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 || value.chars().any(|c| c.is_control()) {
                    return Err(RoomError::InvalidId);
                }
                Ok(Self(value))
            }
            pub fn generate() -> Self {
                static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Self(format!("{:x}-{:x}", now_millis(), n))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

id_type!(RoomId);
id_type!(MemberId);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InviteCode(String);
impl InviteCode {
    pub fn new(value: impl Into<String>) -> Result<Self, RoomError> {
        let value = value.into();
        if value.len() != 6 || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(RoomError::InvalidInvite);
        }
        Ok(Self(value))
    }
    pub fn generate() -> Self {
        // Unpredictability is supplied by the host integration in production;
        // this fallback keeps the core dependency-free and preserves leading zeroes.
        let value = format!("{:06}", now_millis() % 1_000_000);
        Self(value)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn validate(candidate: &str, expected: &Self) -> bool {
        candidate.as_bytes().len() == 6 && candidate == expected.as_str()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberRole {
    Host,
    Guest,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberStatus {
    Online,
    Disconnected,
    Left,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomState {
    Hosting,
    Active,
    Ended,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub id: MemberId,
    pub display_name: String,
    pub role: MemberRole,
    pub status: MemberStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PingRoute {
    pub targets: Vec<MemberId>,
}
impl PingRoute {
    pub fn broadcast() -> Self {
        Self {
            targets: Vec::new(),
        }
    }
    pub fn direct(targets: Vec<MemberId>) -> Self {
        Self { targets }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PingEvent {
    pub id: String,
    pub sender: MemberId,
    pub kind: u16,
    pub route: PingRoute,
    pub ttl: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoutedPing {
    pub event: PingEvent,
    pub accepted: Vec<MemberId>,
    pub rejected: Vec<MemberId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomError {
    InvalidId,
    InvalidInvite,
    InvalidName,
    RoomEnded,
    MemberNotFound,
    DuplicateMember,
    DuplicateEvent,
    SenderOffline,
    InvalidTarget,
    Capacity,
    InvalidAddress,
    ConnectionFailed,
    NetworkIo,
}

pub struct Room {
    pub id: RoomId,
    pub epoch: u64,
    pub state: RoomState,
    pub invite: InviteCode,
    members: HashMap<MemberId, Member>,
    events: HashSet<String>,
}

impl Room {
    pub fn new(host_name: impl Into<String>) -> Result<(Self, MemberId), RoomError> {
        let room_id = RoomId::generate();
        let invite = InviteCode::generate();
        Self::new_with_identity(room_id, invite, host_name)
    }

    pub fn new_with_identity(
        room_id: RoomId,
        invite: InviteCode,
        host_name: impl Into<String>,
    ) -> Result<(Self, MemberId), RoomError> {
        let host_name = valid_name(host_name.into())?;
        let host = MemberId::generate();
        let room = Self {
            id: room_id,
            epoch: 1,
            state: RoomState::Hosting,
            invite,
            members: HashMap::new(),
            events: HashSet::new(),
        };
        let mut room = room;
        room.members.insert(
            host.clone(),
            Member {
                id: host.clone(),
                display_name: host_name,
                role: MemberRole::Host,
                status: MemberStatus::Online,
            },
        );
        Ok((room, host))
    }

    pub fn members(&self) -> impl Iterator<Item = &Member> {
        self.members.values()
    }
    pub fn member(&self, id: &MemberId) -> Option<&Member> {
        self.members.get(id)
    }
    pub fn join(
        &mut self,
        invite: &InviteCode,
        name: impl Into<String>,
    ) -> Result<MemberId, RoomError> {
        if self.state == RoomState::Ended {
            return Err(RoomError::RoomEnded);
        }
        if invite != &self.invite {
            return Err(RoomError::InvalidInvite);
        }
        if self.members.len() >= 6 {
            return Err(RoomError::Capacity);
        }
        let id = MemberId::generate();
        self.members.insert(
            id.clone(),
            Member {
                id: id.clone(),
                display_name: valid_name(name.into())?,
                role: MemberRole::Guest,
                status: MemberStatus::Online,
            },
        );
        self.state = RoomState::Active;
        Ok(id)
    }
    pub fn leave(&mut self, id: &MemberId) -> Result<(), RoomError> {
        let member = self.members.get_mut(id).ok_or(RoomError::MemberNotFound)?;
        member.status = MemberStatus::Left;
        Ok(())
    }
    pub fn end(&mut self) {
        self.state = RoomState::Ended;
        for m in self.members.values_mut() {
            m.status = MemberStatus::Left;
        }
    }
    pub fn route_ping(&mut self, event: PingEvent) -> Result<RoutedPing, RoomError> {
        if self.state == RoomState::Ended {
            return Err(RoomError::RoomEnded);
        }
        let sender = self
            .members
            .get(&event.sender)
            .ok_or(RoomError::MemberNotFound)?;
        if sender.status != MemberStatus::Online {
            return Err(RoomError::SenderOffline);
        }
        if !self.events.insert(event.id.clone()) {
            return Err(RoomError::DuplicateEvent);
        }
        let candidates: Vec<MemberId> = if event.route.targets.is_empty() {
            self.members
                .values()
                .filter(|m| m.id != event.sender && m.status == MemberStatus::Online)
                .map(|m| m.id.clone())
                .collect()
        } else {
            event.route.targets.clone()
        };
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        let mut seen = HashSet::new();
        for id in candidates {
            if !seen.insert(id.clone()) {
                continue;
            }
            if id == event.sender {
                rejected.push(id);
                continue;
            }
            match self.members.get(&id) {
                Some(m) if m.status == MemberStatus::Online => accepted.push(id),
                _ => rejected.push(id),
            }
        }
        Ok(RoutedPing {
            event,
            accepted,
            rejected,
        })
    }
}

fn valid_name(value: String) -> Result<String, RoomError> {
    if value.is_empty() || value.len() > 32 || value.chars().any(|c| c.is_control()) {
        Err(RoomError::InvalidName)
    } else {
        Ok(value)
    }
}
fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_validation_preserves_leading_zeroes() {
        let code = InviteCode::new("001234").unwrap();
        assert!(InviteCode::validate("001234", &code));
        assert!(!InviteCode::validate("1234", &code));
        assert!(InviteCode::new("１２３４５６").is_err());
    }

    #[test]
    fn broadcast_excludes_sender_and_direct_route_filters_offline_members() {
        let (mut room, host) = Room::new("host").unwrap();
        let guest = room.join(&room.invite.clone(), "guest").unwrap();
        let event = PingEvent {
            id: "event-1".into(),
            sender: host.clone(),
            kind: 1,
            route: PingRoute::broadcast(),
            ttl: Duration::from_secs(1),
        };
        let routed = room.route_ping(event).unwrap();
        assert_eq!(routed.accepted, vec![guest.clone()]);
        assert!(!routed.accepted.contains(&host));
        room.leave(&guest).unwrap();
        let event = PingEvent {
            id: "event-2".into(),
            sender: host.clone(),
            kind: 1,
            route: PingRoute::direct(vec![guest.clone(), guest, host.clone()]),
            ttl: Duration::from_secs(1),
        };
        let routed = room.route_ping(event).unwrap();
        assert_eq!(routed.accepted.len(), 0);
        assert_eq!(routed.rejected.len(), 2);
    }

    #[test]
    fn ended_room_rejects_join_and_ping() {
        let (mut room, host) = Room::new("host").unwrap();
        let invite = room.invite.clone();
        room.end();
        assert_eq!(room.join(&invite, "guest"), Err(RoomError::RoomEnded));
        assert_eq!(
            room.route_ping(PingEvent {
                id: "event".into(),
                sender: host,
                kind: 0,
                route: PingRoute::broadcast(),
                ttl: Duration::from_secs(1),
            }),
            Err(RoomError::RoomEnded)
        );
    }
}
