use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::os::raw::c_char;
use std::ptr;
use std::slice;
use std::time::Duration;

pub struct EngineSocket {
    stream: TcpStream,
}

pub struct EngineListener {
    listener: TcpListener,
}

fn timeout(milliseconds: u64) -> Option<Duration> {
    if milliseconds == 0 {
        None
    } else {
        Some(Duration::from_millis(milliseconds))
    }
}

/// Connects to a numeric host or DNS name. Bonjour resolution is handled by
/// the discovery module; Rust owns the TCP stream lifecycle.
#[no_mangle]
pub unsafe extern "C" fn party_engine_socket_connect(
    host: *const c_char,
    port: u16,
    timeout_ms: u64,
) -> *mut EngineSocket {
    if host.is_null() {
        return ptr::null_mut();
    }
    let host = match std::ffi::CStr::from_ptr(host).to_str() {
        Ok(value) if !value.is_empty() => value,
        _ => return ptr::null_mut(),
    };
    let address = match (host, port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut values| values.next())
    {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let stream = match timeout(timeout_ms) {
        Some(limit) => TcpStream::connect_timeout(&address, limit),
        None => TcpStream::connect(address),
    };
    match stream {
        Ok(stream) => Box::into_raw(Box::new(EngineSocket { stream })),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_socket_listen(port: u16) -> *mut EngineListener {
    match TcpListener::bind(("0.0.0.0", port)) {
        Ok(listener) => Box::into_raw(Box::new(EngineListener { listener })),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_socket_accept(
    listener: *mut EngineListener,
    timeout_ms: u64,
) -> *mut EngineSocket {
    if listener.is_null() {
        return ptr::null_mut();
    }
    let listener = &mut *listener;
    if let Some(limit) = timeout(timeout_ms) {
        let _ = listener.listener.set_nonblocking(true);
        let deadline = std::time::Instant::now() + limit;
        loop {
            match listener.listener.accept() {
                Ok((stream, _)) => {
                    // TcpListener is switched to non-blocking mode for the
                    // polling timeout above. On Darwin accepted sockets can
                    // inherit that flag, which would make the first read
                    // return WouldBlock even though the peer has not closed.
                    // Read timeouts are implemented by the stream, so restore
                    // blocking mode before handing ownership to the caller.
                    if stream.set_nonblocking(false).is_err() {
                        return ptr::null_mut();
                    }
                    return Box::into_raw(Box::new(EngineSocket { stream }));
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        return ptr::null_mut();
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(_) => return ptr::null_mut(),
            }
        }
    }
    match listener.listener.accept() {
        Ok((stream, _)) => Box::into_raw(Box::new(EngineSocket { stream })),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_socket_listener_port(listener: *mut EngineListener) -> u16 {
    if listener.is_null() {
        return 0;
    }
    (&*listener)
        .listener
        .local_addr()
        .map(|address| address.port())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_socket_read(
    socket: *mut EngineSocket,
    output: *mut u8,
    output_len: usize,
    timeout_ms: u64,
) -> isize {
    if socket.is_null() || output.is_null() || output_len == 0 {
        return -1;
    }
    let socket = &mut *socket;
    if socket.stream.set_read_timeout(timeout(timeout_ms)).is_err() {
        return -1;
    }
    let output = slice::from_raw_parts_mut(output, output_len);
    match socket.stream.read(output) {
        Ok(bytes) => bytes as isize,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_socket_write(
    socket: *mut EngineSocket,
    input: *const u8,
    input_len: usize,
    timeout_ms: u64,
) -> isize {
    if socket.is_null() || input.is_null() || input_len == 0 {
        return -1;
    }
    let socket = &mut *socket;
    if socket
        .stream
        .set_write_timeout(timeout(timeout_ms))
        .is_err()
    {
        return -1;
    }
    let input = slice::from_raw_parts(input, input_len);
    match socket.stream.write_all(input) {
        Ok(()) => input_len as isize,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_socket_close(socket: *mut EngineSocket) {
    if !socket.is_null() {
        drop(Box::from_raw(socket));
    }
}

#[no_mangle]
pub unsafe extern "C" fn party_engine_socket_listener_close(listener: *mut EngineListener) {
    if !listener.is_null() {
        drop(Box::from_raw(listener));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn tcp_lifecycle_connects_reads_writes_and_closes() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut input = [0_u8; 4];
            stream.read_exact(&mut input).unwrap();
            assert_eq!(&input, b"ping");
            stream.write_all(b"pong").unwrap();
        });

        let mut client = TcpStream::connect(address).unwrap();
        client.write_all(b"ping").unwrap();
        let mut output = [0_u8; 4];
        client.read_exact(&mut output).unwrap();
        assert_eq!(&output, b"pong");
        server.join().unwrap();
    }
}
