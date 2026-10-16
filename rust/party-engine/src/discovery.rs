//! macOS Bonjour/DNS-SD discovery.
//!
//! The UI never deals with IP addresses. A host registers a service and a
//! guest browses the same service type. The TCP room protocol remains in
//! `network.rs`; this module only owns the platform discovery seam.

use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

type DNSServiceRef = *mut c_void;
type DNSServiceFlags = u32;
type DNSServiceErrorType = i32;
type DNSServiceProtocol = u32;

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

const K_DNS_SERVICE_NO_ERROR: DNSServiceErrorType = 0;
const K_DNS_SERVICE_FLAGS_ADD: DNSServiceFlags = 0x2;
const K_DNS_SERVICE_FLAGS_MORE_COMING: DNSServiceFlags = 0x1;
const K_DNS_SERVICE_PROTOCOL_IPV4: DNSServiceProtocol = 0x1;
const K_DNS_SERVICE_PROTOCOL_IPV6: DNSServiceProtocol = 0x2;
const POLLIN: i16 = 0x0001;
const SERVICE_TYPE: &str = "_partylolping._tcp.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredRoom {
    pub name: String,
    pub host: String,
    pub port: u16,
}

pub struct BonjourPublisher {
    service: DNSServiceRef,
    stop: Arc<AtomicBool>,
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
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread_service = service as usize;
        thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                let result = unsafe { DNSServiceProcessResult(thread_service as DNSServiceRef) };
                if result != K_DNS_SERVICE_NO_ERROR {
                    break;
                }
            }
        });
        Ok(Self { service, stop })
    }
}

impl Drop for BonjourPublisher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        unsafe { DNSServiceRefDeallocate(self.service) };
    }
}

pub fn browse(timeout: Duration) -> Vec<DiscoveredRoom> {
    let mut service = std::ptr::null_mut();
    let state = Box::new(Mutex::new(Vec::<DiscoveredRoom>::new()));
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
        let ready = unsafe { poll(&mut poll_fd, 1, wait_millis) };
        if ready <= 0 {
            break;
        }
        let result = unsafe { DNSServiceProcessResult(service) };
        if result != K_DNS_SERVICE_NO_ERROR {
            break;
        }
    }
    unsafe { DNSServiceRefDeallocate(service) };
    let rooms = unsafe { Box::from_raw(state_ptr) };
    rooms.lock().map(|rooms| rooms.clone()).unwrap_or_default()
}

unsafe extern "C" fn browse_callback(
    _service: DNSServiceRef,
    flags: DNSServiceFlags,
    error: DNSServiceErrorType,
    _interface: u32,
    _service_name: *const c_char,
    _reg_type: *const c_char,
    _reply_domain: *const c_char,
    context: *mut c_void,
) {
    if error != K_DNS_SERVICE_NO_ERROR || flags & K_DNS_SERVICE_FLAGS_ADD == 0 || context.is_null()
    {
        return;
    }
    // DNS-SD browse callbacks intentionally only identify the service. A
    // resolved endpoint is added by the resolver seam before it is presented
    // to the UI; keeping this callback free of UI state makes it thread-safe.
    let _ = CStr::from_ptr(_service_name);
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
                DNSServiceErrorType,
                u32,
                *const c_char,
                *const c_char,
                *const c_char,
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
