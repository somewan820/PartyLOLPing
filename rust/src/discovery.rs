//! macOS Bonjour/DNS-SD discovery.
//!
//! A browse callback only identifies a service. Before a room is presented to
//! the UI we resolve that service to a host and port, so callers never have to
//! guess an address or connect to a stale demo endpoint.

use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

type DNSServiceRef = *mut c_void;
type DNSServiceFlags = u32;
type DNSServiceErrorType = i32;

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

const K_DNS_SERVICE_NO_ERROR: DNSServiceErrorType = 0;
const K_DNS_SERVICE_FLAGS_ADD: DNSServiceFlags = 0x2;
const POLLIN: i16 = 0x0001;
const SERVICE_TYPE: &str = "_partylolping._tcp.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredRoom {
    pub name: String,
    pub host: String,
    pub port: u16,
}

pub struct BonjourPublisher {
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

unsafe impl Send for BonjourPublisher {}
unsafe impl Sync for BonjourPublisher {}

impl BonjourPublisher {
    pub fn publish(name: &str, port: u16) -> Result<Self, String> {
        let name = CString::new(name).map_err(|_| "invalid service name".to_owned())?;
        let service_type = CString::new(SERVICE_TYPE).expect("static service type");
        let domain = CString::new("local.").expect("static domain");
        let mut service = std::ptr::null_mut();
        let error = unsafe {
            DNSServiceRegister(
                &mut service,
                0,
                0,
                name.as_ptr(),
                service_type.as_ptr(),
                domain.as_ptr(),
                std::ptr::null(),
                port.to_be(),
                0,
                std::ptr::null(),
                None,
                std::ptr::null_mut(),
            )
        };
        if error != K_DNS_SERVICE_NO_ERROR || service.is_null() {
            return Err(format!("Bonjour register failed ({error})"));
        }

        // DNSServiceProcessResult blocks until the service has an event, so
        // keep it off the UI thread. Polling the descriptor gives Drop a
        // bounded shutdown path and avoids deallocating a ref while another
        // thread is inside DNSServiceProcessResult.
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread_service = service as usize;
        let worker = thread::spawn(move || {
            let service = thread_service as DNSServiceRef;
            let socket = unsafe { DNSServiceRefSockFD(service) };
            let mut poll_fd = PollFd {
                fd: socket,
                events: POLLIN,
                revents: 0,
            };
            while !thread_stop.load(Ordering::Acquire) {
                if unsafe { poll(&mut poll_fd, 1, 100) } <= 0 {
                    continue;
                }
                if unsafe { DNSServiceProcessResult(service) } != K_DNS_SERVICE_NO_ERROR {
                    break;
                }
            }
            unsafe { DNSServiceRefDeallocate(service) };
        });
        Ok(Self {
            stop,
            worker: Some(worker),
        })
    }
}

impl Drop for BonjourPublisher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BrowseService {
    name: String,
    reg_type: String,
    domain: String,
    interface_index: u32,
}

#[derive(Default)]
struct BrowseState {
    services: Vec<BrowseService>,
}

#[derive(Default)]
struct ResolveState {
    room: Option<DiscoveredRoom>,
    display_name: String,
}

pub fn browse(timeout: Duration) -> Vec<DiscoveredRoom> {
    let mut service = std::ptr::null_mut();
    let state = Box::new(BrowseState::default());
    let state_ptr = Box::into_raw(state);
    let service_type = CString::new(SERVICE_TYPE).expect("static service type");
    let domain = CString::new("local.").expect("static domain");
    let error = unsafe {
        DNSServiceBrowse(
            &mut service,
            0,
            0,
            service_type.as_ptr(),
            domain.as_ptr(),
            Some(browse_callback),
            state_ptr.cast(),
        )
    };
    if error != K_DNS_SERVICE_NO_ERROR || service.is_null() {
        unsafe { drop(Box::from_raw(state_ptr)) };
        return Vec::new();
    }

    let socket = unsafe { DNSServiceRefSockFD(service) };
    if socket < 0 {
        unsafe { DNSServiceRefDeallocate(service) };
        unsafe { drop(Box::from_raw(state_ptr)) };
        return Vec::new();
    }

    let deadline = Instant::now() + timeout;
    let mut poll_fd = PollFd {
        fd: socket,
        events: POLLIN,
        revents: 0,
    };
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let wait_millis = remaining.as_millis().min(i32::MAX as u128) as i32;
        if unsafe { poll(&mut poll_fd, 1, wait_millis) } <= 0 {
            break;
        }
        if unsafe { DNSServiceProcessResult(service) } != K_DNS_SERVICE_NO_ERROR {
            break;
        }
    }
    unsafe { DNSServiceRefDeallocate(service) };

    let state = unsafe { Box::from_raw(state_ptr) };
    let services = state.services.clone();
    let mut rooms = Vec::with_capacity(services.len());
    for item in services {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        if let Some(room) = resolve_service(&item, remaining) {
            rooms.push(room);
        }
    }
    rooms
}

unsafe extern "C" fn browse_callback(
    _service: DNSServiceRef,
    flags: DNSServiceFlags,
    interface_index: u32,
    error: DNSServiceErrorType,
    service_name: *const c_char,
    reg_type: *const c_char,
    reply_domain: *const c_char,
    context: *mut c_void,
) {
    // DNS-SD's order is (flags, interface index, error code). Swapping the
    // latter two makes every normal Wi-Fi callback look like an error.
    if error != K_DNS_SERVICE_NO_ERROR
        || context.is_null()
        || service_name.is_null()
        || reg_type.is_null()
        || reply_domain.is_null()
    {
        return;
    }
    let state = &mut *(context as *mut BrowseState);
    let name = CStr::from_ptr(service_name).to_string_lossy().into_owned();
    let reg_type = CStr::from_ptr(reg_type).to_string_lossy().into_owned();
    let domain = CStr::from_ptr(reply_domain).to_string_lossy().into_owned();
    let item = BrowseService {
        name,
        reg_type,
        domain,
        interface_index,
    };
    if flags & K_DNS_SERVICE_FLAGS_ADD != 0 {
        if !state.services.contains(&item) {
            state.services.push(item);
        }
    } else {
        state.services.retain(|existing| existing != &item);
    }
}

fn resolve_service(service: &BrowseService, timeout: Duration) -> Option<DiscoveredRoom> {
    let name = CString::new(service.name.as_str()).ok()?;
    let reg_type = CString::new(service.reg_type.as_str()).ok()?;
    let domain = CString::new(service.domain.as_str()).ok()?;
    let mut resolver = std::ptr::null_mut();
    let state = Box::new(ResolveState {
        room: None,
        display_name: service.name.clone(),
    });
    let state_ptr = Box::into_raw(state);
    let error = unsafe {
        DNSServiceResolve(
            &mut resolver,
            0,
            service.interface_index,
            name.as_ptr(),
            reg_type.as_ptr(),
            domain.as_ptr(),
            Some(resolve_callback),
            state_ptr.cast(),
        )
    };
    if error != K_DNS_SERVICE_NO_ERROR || resolver.is_null() {
        unsafe { drop(Box::from_raw(state_ptr)) };
        return None;
    }
    let socket = unsafe { DNSServiceRefSockFD(resolver) };
    if socket < 0 {
        unsafe { DNSServiceRefDeallocate(resolver) };
        unsafe { drop(Box::from_raw(state_ptr)) };
        return None;
    }
    let deadline = Instant::now() + timeout;
    let mut poll_fd = PollFd {
        fd: socket,
        events: POLLIN,
        revents: 0,
    };
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let wait_millis = remaining.as_millis().min(i32::MAX as u128) as i32;
        if unsafe { poll(&mut poll_fd, 1, wait_millis) } <= 0 {
            break;
        }
        if unsafe { DNSServiceProcessResult(resolver) } != K_DNS_SERVICE_NO_ERROR {
            break;
        }
        let has_result = unsafe { (*(state_ptr as *const ResolveState)).room.is_some() };
        if has_result {
            break;
        }
    }
    unsafe { DNSServiceRefDeallocate(resolver) };
    unsafe { (*Box::from_raw(state_ptr)).room.take() }
}

unsafe extern "C" fn resolve_callback(
    _service: DNSServiceRef,
    _flags: DNSServiceFlags,
    _interface_index: u32,
    error: DNSServiceErrorType,
    _fullname: *const c_char,
    hosttarget: *const c_char,
    port: u16,
    _txt_len: u16,
    _txt_record: *const u8,
    context: *mut c_void,
) {
    if error != K_DNS_SERVICE_NO_ERROR || context.is_null() || hosttarget.is_null() || port == 0 {
        return;
    }
    let host = CStr::from_ptr(hosttarget)
        .to_string_lossy()
        .trim_end_matches('.')
        .to_owned();
    if host.is_empty() {
        return;
    }
    let state = &mut *(context as *mut ResolveState);
    state.room = Some(DiscoveredRoom {
        name: state.display_name.clone(),
        host,
        port: u16::from_be(port),
    });
}

#[link(name = "System")]
extern "C" {
    fn DNSServiceRegister(
        sd_ref: *mut DNSServiceRef,
        flags: DNSServiceFlags,
        interface_index: u32,
        name: *const c_char,
        reg_type: *const c_char,
        domain: *const c_char,
        host: *const c_char,
        port: u16,
        txt_len: u16,
        txt_record: *const u8,
        callback: Option<
            unsafe extern "C" fn(
                DNSServiceRef,
                DNSServiceFlags,
                DNSServiceErrorType,
                *const c_char,
                *const c_char,
                *const c_char,
                *mut c_void,
            ),
        >,
        context: *mut c_void,
    ) -> DNSServiceErrorType;
    fn DNSServiceBrowse(
        sd_ref: *mut DNSServiceRef,
        flags: DNSServiceFlags,
        interface_index: u32,
        reg_type: *const c_char,
        domain: *const c_char,
        callback: Option<
            unsafe extern "C" fn(
                DNSServiceRef,
                DNSServiceFlags,
                u32,
                DNSServiceErrorType,
                *const c_char,
                *const c_char,
                *const c_char,
                *mut c_void,
            ),
        >,
        context: *mut c_void,
    ) -> DNSServiceErrorType;
    fn DNSServiceResolve(
        sd_ref: *mut DNSServiceRef,
        flags: DNSServiceFlags,
        interface_index: u32,
        name: *const c_char,
        reg_type: *const c_char,
        domain: *const c_char,
        callback: Option<
            unsafe extern "C" fn(
                DNSServiceRef,
                DNSServiceFlags,
                u32,
                DNSServiceErrorType,
                *const c_char,
                *const c_char,
                u16,
                u16,
                *const u8,
                *mut c_void,
            ),
        >,
        context: *mut c_void,
    ) -> DNSServiceErrorType;
    fn DNSServiceRefSockFD(sd_ref: DNSServiceRef) -> i32;
    fn DNSServiceProcessResult(sd_ref: DNSServiceRef) -> DNSServiceErrorType;
    fn DNSServiceRefDeallocate(sd_ref: DNSServiceRef);
    fn poll(fds: *mut PollFd, count: usize, timeout: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn browse_callback_keeps_added_service_on_normal_interface() {
        let mut state = BrowseState::default();
        let name = CString::new("Host").unwrap();
        let reg_type = CString::new(SERVICE_TYPE).unwrap();
        let domain = CString::new("local.").unwrap();
        unsafe {
            browse_callback(
                ptr::null_mut(),
                K_DNS_SERVICE_FLAGS_ADD,
                1,
                K_DNS_SERVICE_NO_ERROR,
                name.as_ptr(),
                reg_type.as_ptr(),
                domain.as_ptr(),
                (&mut state as *mut BrowseState).cast(),
            );
        }
        assert_eq!(state.services.len(), 1);
        assert_eq!(state.services[0].name, "Host");
        assert_eq!(state.services[0].interface_index, 1);
    }

    #[test]
    fn browse_callback_removes_service_without_leaking_stale_rooms() {
        let service = BrowseService {
            name: "Host".into(),
            reg_type: SERVICE_TYPE.into(),
            domain: "local.".into(),
            interface_index: 1,
        };
        let mut state = BrowseState {
            services: vec![service],
        };
        let name = CString::new("Host").unwrap();
        let reg_type = CString::new(SERVICE_TYPE).unwrap();
        let domain = CString::new("local.").unwrap();
        unsafe {
            browse_callback(
                ptr::null_mut(),
                0,
                1,
                K_DNS_SERVICE_NO_ERROR,
                name.as_ptr(),
                reg_type.as_ptr(),
                domain.as_ptr(),
                (&mut state as *mut BrowseState).cast(),
            );
        }
        assert!(state.services.is_empty());
    }

    #[test]
    fn resolve_callback_preserves_service_name_and_network_port() {
        let mut state = ResolveState {
            room: None,
            display_name: "我的房间".into(),
        };
        let host = CString::new("host.local.").unwrap();
        unsafe {
            resolve_callback(
                ptr::null_mut(),
                0,
                1,
                K_DNS_SERVICE_NO_ERROR,
                ptr::null(),
                host.as_ptr(),
                45678_u16.to_be(),
                0,
                ptr::null(),
                (&mut state as *mut ResolveState).cast(),
            );
        }
        let room = state.room.unwrap();
        assert_eq!(room.name, "我的房间");
        assert_eq!(room.host, "host.local");
        assert_eq!(room.port, 45678);
    }
}
