//! Small, ownership-explicit C ABI for the room domain.
//!
//! Strings are UTF-8 byte slices, not nul-terminated strings.  The caller
//! owns every input and output buffer; the only Rust allocation exposed here
//! is the opaque `PartyEngineRoom`, released with `party_engine_room_destroy`.

use crate::{
    lan_join, InviteCode, LanHost, MemberId, PingEvent, PingRoute, Room, RoomError, RoomId,
};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::slice;

pub const PARTY_ENGINE_OK: i32 = 0;
pub const PARTY_ENGINE_ERR_NULL: i32 = -1;
pub const PARTY_ENGINE_ERR_UTF8: i32 = -2;
pub const PARTY_ENGINE_ERR_INVALID_ID: i32 = -3;
pub const PARTY_ENGINE_ERR_INVALID_INVITE: i32 = -4;
pub const PARTY_ENGINE_ERR_BUFFER_TOO_SMALL: i32 = -5;
pub const PARTY_ENGINE_ERR_ROOM_ENDED: i32 = -6;
pub const PARTY_ENGINE_ERR_MEMBER_NOT_FOUND: i32 = -7;
pub const PARTY_ENGINE_ERR_DUPLICATE_EVENT: i32 = -8;
pub const PARTY_ENGINE_ERR_SENDER_OFFLINE: i32 = -9;
pub const PARTY_ENGINE_ERR_INVALID_TARGET: i32 = -10;
pub const PARTY_ENGINE_ERR_CAPACITY: i32 = -11;
pub const PARTY_ENGINE_ERR_INVALID_NAME: i32 = -12;
pub const PARTY_ENGINE_ERR_INVALID_ADDRESS: i32 = -13;
pub const PARTY_ENGINE_ERR_CONNECTION_FAILED: i32 = -14;
pub const PARTY_ENGINE_ERR_NETWORK_IO: i32 = -15;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PartyEngineByteSlice {
    pub ptr: *const u8,
    pub len: usize,
}

pub struct PartyEngineRoom {
    room: Room,
}

pub struct PartyEngineLanHost {
    _host: LanHost,
}

fn error_code(error: RoomError) -> i32 {
    match error {
        RoomError::InvalidId => PARTY_ENGINE_ERR_INVALID_ID,
        RoomError::InvalidInvite => PARTY_ENGINE_ERR_INVALID_INVITE,
        RoomError::InvalidName => PARTY_ENGINE_ERR_INVALID_NAME,
        RoomError::RoomEnded => PARTY_ENGINE_ERR_ROOM_ENDED,
        RoomError::MemberNotFound => PARTY_ENGINE_ERR_MEMBER_NOT_FOUND,
        RoomError::DuplicateEvent => PARTY_ENGINE_ERR_DUPLICATE_EVENT,
        RoomError::SenderOffline => PARTY_ENGINE_ERR_SENDER_OFFLINE,
        RoomError::InvalidTarget => PARTY_ENGINE_ERR_INVALID_TARGET,
        RoomError::Capacity => PARTY_ENGINE_ERR_CAPACITY,
        RoomError::DuplicateMember => PARTY_ENGINE_ERR_MEMBER_NOT_FOUND,
        RoomError::InvalidAddress => PARTY_ENGINE_ERR_INVALID_ADDRESS,
        RoomError::ConnectionFailed => PARTY_ENGINE_ERR_CONNECTION_FAILED,
        RoomError::NetworkIo => PARTY_ENGINE_ERR_NETWORK_IO,
    }
}

unsafe fn input<'a>(value: PartyEngineByteSlice) -> Result<&'a str, i32> {
    if value.ptr.is_null() {
        return Err(PARTY_ENGINE_ERR_NULL);
    }
    let bytes = slice::from_raw_parts(value.ptr, value.len);
    std::str::from_utf8(bytes).map_err(|_| PARTY_ENGINE_ERR_UTF8)
}

unsafe fn copy_text(value: &str, output: *mut u8, capacity: usize, written: *mut usize) -> i32 {
    if written.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    *written = value.len();
    if output.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    if capacity < value.len() {
        return PARTY_ENGINE_ERR_BUFFER_TOO_SMALL;
    }
    std::ptr::copy_nonoverlapping(value.as_ptr(), output, value.len());
    PARTY_ENGINE_OK
}

/// Creates a room and writes its opaque handle to `out_room`.
#[no_mangle]
pub unsafe extern "C" fn party_engine_room_create(
    host_name: PartyEngineByteSlice,
    out_room: *mut *mut PartyEngineRoom,
) -> i32 {
    if out_room.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    *out_room = std::ptr::null_mut();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let name = input(host_name)?;
        let (room, _) = Room::new(name).map_err(error_code)?;
        *out_room = Box::into_raw(Box::new(PartyEngineRoom { room }));
        Ok(())
    }));
    match result {
        Ok(result) => result.map_or_else(|code| code, |_| PARTY_ENGINE_OK),
        Err(_) => PARTY_ENGINE_ERR_NULL,
    }
}

/// Creates a room with externally supplied identifiers while keeping the
/// native UI snapshot and the Rust source of truth on the same room.
#[no_mangle]
pub unsafe extern "C" fn party_engine_room_create_with_identity(
    room_id: PartyEngineByteSlice,
    invite: PartyEngineByteSlice,
    host_name: PartyEngineByteSlice,
    out_room: *mut *mut PartyEngineRoom,
) -> i32 {
    if out_room.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    *out_room = std::ptr::null_mut();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let room_id = RoomId::new(input(room_id)?).map_err(error_code)?;
        let invite = InviteCode::new(input(invite)?).map_err(error_code)?;
        let host_name = input(host_name)?;
        let (room, _) = Room::new_with_identity(room_id, invite, host_name).map_err(error_code)?;
        *out_room = Box::into_raw(Box::new(PartyEngineRoom { room }));
        Ok(())
    }));
    match result {
        Ok(result) => result.map_or_else(|code| code, |_| PARTY_ENGINE_OK),
        Err(_) => PARTY_ENGINE_ERR_NULL,
    }
}

/// Destroys a room. Null is deliberately a safe no-op for cleanup paths.
#[no_mangle]
pub unsafe extern "C" fn party_engine_room_destroy(room: *mut PartyEngineRoom) {
    if !room.is_null() {
        drop(Box::from_raw(room));
    }
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_lan_host_create(
    host_name: PartyEngineByteSlice,
    invite: PartyEngineByteSlice,
    port: u16,
    output: *mut *mut PartyEngineLanHost,
    output_port: *mut u16,
    output_member: *mut u8,
    member_capacity: usize,
    member_written: *mut usize,
) -> i32 {
    if output.is_null() || output_port.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    *output = std::ptr::null_mut();
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), i32> {
        let host_name = input(host_name)?;
        let invite = input(invite)?;
        let (host, actual_port) = LanHost::start(host_name, invite, port).map_err(error_code)?;
        let member = host
            .room
            .lock()
            .map_err(|_| PARTY_ENGINE_ERR_ROOM_ENDED)?
            .members()
            .find(|member| matches!(member.role, crate::MemberRole::Host))
            .map(|member| member.id.as_str().to_owned())
            .ok_or(PARTY_ENGINE_ERR_MEMBER_NOT_FOUND)?;
        let status = copy_text(&member, output_member, member_capacity, member_written);
        if status != PARTY_ENGINE_OK {
            return Err(status);
        }
        *output_port = actual_port;
        *output = Box::into_raw(Box::new(PartyEngineLanHost { _host: host }));
        Ok(())
    }));
    match result {
        Ok(result) => result.map_or_else(|code| code, |_| PARTY_ENGINE_OK),
        Err(_) => PARTY_ENGINE_ERR_NULL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_lan_host_destroy(host: *mut PartyEngineLanHost) {
    if !host.is_null() {
        drop(Box::from_raw(host));
    }
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_lan_join(
    address: PartyEngineByteSlice,
    port: u16,
    invite: PartyEngineByteSlice,
    nickname: PartyEngineByteSlice,
    output_member: *mut u8,
    member_capacity: usize,
    member_written: *mut usize,
) -> i32 {
    if member_written.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<i32, i32> {
        let address = input(address)?;
        let invite = input(invite)?;
        let nickname = input(nickname)?;
        let member = lan_join(address, port, invite, nickname).map_err(error_code)?;
        Ok(copy_text(
            &member,
            output_member,
            member_capacity,
            member_written,
        ))
    }));
    result
        .unwrap_or(Err(PARTY_ENGINE_ERR_NULL))
        .map_or_else(|error| error, |status| status)
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_room_id(
    room: *const PartyEngineRoom,
    output: *mut u8,
    capacity: usize,
    written: *mut usize,
) -> i32 {
    if room.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    copy_text((*room).room.id.as_str(), output, capacity, written)
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_room_invite(
    room: *const PartyEngineRoom,
    output: *mut u8,
    capacity: usize,
    written: *mut usize,
) -> i32 {
    if room.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    copy_text((*room).room.invite.as_str(), output, capacity, written)
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_room_host_member(
    room: *const PartyEngineRoom,
    output: *mut u8,
    capacity: usize,
    written: *mut usize,
) -> i32 {
    if room.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    let host = (*room)
        .room
        .members()
        .find(|member| matches!(member.role, crate::MemberRole::Host));
    match host {
        Some(member) => copy_text(member.id.as_str(), output, capacity, written),
        None => PARTY_ENGINE_ERR_MEMBER_NOT_FOUND,
    }
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_room_join(
    room: *mut PartyEngineRoom,
    invite: PartyEngineByteSlice,
    name: PartyEngineByteSlice,
    output_member_id: *mut u8,
    capacity: usize,
    written: *mut usize,
) -> i32 {
    if room.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<i32, i32> {
        let invite = InviteCode::new(input(invite)?).map_err(error_code)?;
        let name = input(name)?;
        let member = (*room).room.join(&invite, name).map_err(error_code)?;
        Ok(copy_text(
            member.as_str(),
            output_member_id,
            capacity,
            written,
        ))
    }));
    result
        .unwrap_or(Err(PARTY_ENGINE_ERR_NULL))
        .map_or_else(|error| error, |status| status)
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_room_leave(
    room: *mut PartyEngineRoom,
    member_id: PartyEngineByteSlice,
) -> i32 {
    if room.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), i32> {
        let member = MemberId::new(input(member_id)?).map_err(error_code)?;
        (*room).room.leave(&member).map_err(error_code)
    }));
    result.map_or(PARTY_ENGINE_ERR_NULL, |result| {
        result.map_or_else(|e| e, |_| PARTY_ENGINE_OK)
    })
}

/// Routes a ping. `targets` is empty for broadcast. For a direct route,
/// `statuses[i]` is 1 when the corresponding target is accepted and 0 when
/// rejected. Duplicate targets are reported with the same status; the counts
/// are the domain's de-duplicated counts.
#[no_mangle]
pub unsafe extern "C" fn party_engine_room_ping(
    room: *mut PartyEngineRoom,
    event_id: PartyEngineByteSlice,
    sender_id: PartyEngineByteSlice,
    kind: u16,
    targets: *const PartyEngineByteSlice,
    target_count: usize,
    statuses: *mut u8,
    statuses_len: usize,
    accepted_count: *mut usize,
    rejected_count: *mut usize,
) -> i32 {
    if room.is_null() || accepted_count.is_null() || rejected_count.is_null() {
        return PARTY_ENGINE_ERR_NULL;
    }
    if target_count > 0 && (targets.is_null() || statuses.is_null() || statuses_len < target_count)
    {
        return PARTY_ENGINE_ERR_NULL;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), i32> {
        let sender = MemberId::new(input(sender_id)?).map_err(error_code)?;
        let event = input(event_id)?.to_owned();
        let target_slices = if target_count == 0 {
            &[][..]
        } else {
            slice::from_raw_parts(targets, target_count)
        };
        let mut target_ids = Vec::with_capacity(target_count);
        for target in target_slices {
            target_ids.push(MemberId::new(input(*target)?).map_err(error_code)?);
        }
        let routed = (*room)
            .room
            .route_ping(PingEvent {
                id: event,
                sender,
                kind,
                route: PingRoute::direct(target_ids.clone()),
                ttl: std::time::Duration::from_secs(1),
            })
            .map_err(error_code)?;
        *accepted_count = routed.accepted.len();
        *rejected_count = routed.rejected.len();
        for (index, target) in target_ids.iter().enumerate() {
            statuses
                .add(index)
                .write(u8::from(routed.accepted.contains(target)));
        }
        Ok(())
    }));
    result
        .unwrap_or(Err(PARTY_ENGINE_ERR_NULL))
        .map_or_else(|e| e, |_| PARTY_ENGINE_OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(value: &str) -> PartyEngineByteSlice {
        PartyEngineByteSlice {
            ptr: value.as_ptr(),
            len: value.len(),
        }
    }

    #[test]
    fn rejects_null_create_inputs_and_invalid_utf8() {
        let mut room = std::ptr::null_mut();
        assert_eq!(
            unsafe {
                party_engine_room_create(
                    PartyEngineByteSlice {
                        ptr: std::ptr::null(),
                        len: 1,
                    },
                    &mut room,
                )
            },
            PARTY_ENGINE_ERR_NULL
        );
        assert!(room.is_null());
        let bad = [0xff];
        assert_eq!(
            unsafe {
                party_engine_room_create(
                    PartyEngineByteSlice {
                        ptr: bad.as_ptr(),
                        len: 1,
                    },
                    &mut room,
                )
            },
            PARTY_ENGINE_ERR_UTF8
        );
    }

    #[test]
    fn output_buffers_are_bounded_and_null_safe() {
        let mut room = std::ptr::null_mut();
        assert_eq!(
            unsafe { party_engine_room_create(bytes("host"), &mut room) },
            PARTY_ENGINE_OK
        );
        let mut required = 0;
        assert_eq!(
            unsafe { party_engine_room_id(room, std::ptr::null_mut(), 0, &mut required) },
            PARTY_ENGINE_ERR_NULL
        );
        assert!(required > 0);
        let mut short = [0; 1];
        assert_eq!(
            unsafe {
                party_engine_room_invite(room, short.as_mut_ptr(), short.len(), &mut required)
            },
            PARTY_ENGINE_ERR_BUFFER_TOO_SMALL
        );
        unsafe { party_engine_room_destroy(room) };
        unsafe { party_engine_room_destroy(std::ptr::null_mut()) };
    }

    #[test]
    fn rejects_null_and_short_ping_buffers() {
        let mut room = std::ptr::null_mut();
        assert_eq!(
            unsafe { party_engine_room_create(bytes("host"), &mut room) },
            PARTY_ENGINE_OK
        );
        let mut accepted = 0;
        let mut rejected = 0;
        assert_eq!(
            unsafe {
                party_engine_room_ping(
                    room,
                    bytes("event"),
                    bytes("bad"),
                    1,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    0,
                    &mut accepted,
                    &mut rejected,
                )
            },
            PARTY_ENGINE_ERR_MEMBER_NOT_FOUND
        );
        assert_eq!(
            unsafe {
                party_engine_room_ping(
                    room,
                    bytes("event"),
                    bytes("bad"),
                    1,
                    std::ptr::null(),
                    1,
                    std::ptr::null_mut(),
                    0,
                    &mut accepted,
                    &mut rejected,
                )
            },
            PARTY_ENGINE_ERR_NULL
        );
        unsafe { party_engine_room_destroy(room) };
    }
}
