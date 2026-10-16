use crate::{BonjourPublisher, InviteCode, Room, RoomError};
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

pub struct LanHost {
    pub room: Arc<Mutex<Room>>,
    stop: Arc<AtomicBool>,
    listener: Option<TcpListener>,
    publisher: Option<BonjourPublisher>,
    peers: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>,
}

impl LanHost {
    pub fn start(host_name: &str, invite: &str, port: u16) -> Result<(Self, u16), RoomError> {
        let invite = InviteCode::new(invite)?;
        let (room, _) = Room::new_with_identity(crate::RoomId::generate(), invite, host_name)?;
        let room = Arc::new(Mutex::new(room));
        let listener = TcpListener::bind(("0.0.0.0", port)).map_err(|_| RoomError::NetworkIo)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| RoomError::NetworkIo)?;
        let actual_port = listener
            .local_addr()
            .map_err(|_| RoomError::NetworkIo)?
            .port();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_listener = listener.try_clone().map_err(|_| RoomError::NetworkIo)?;
        let thread_room = Arc::clone(&room);
        let thread_stop = Arc::clone(&stop);
        let peers = Arc::new(Mutex::new(HashMap::new()));
        let thread_peers = Arc::clone(&peers);
        thread::spawn(move || accept_loop(thread_listener, thread_room, thread_stop, thread_peers));
        let publisher = BonjourPublisher::publish(host_name, actual_port).ok();
        Ok((
            Self {
                room,
                stop,
                listener: Some(listener),
                publisher,
                peers,
            },
            actual_port,
        ))
    }

    pub fn discovery_available(&self) -> bool {
        self.publisher.is_some()
    }

    /// Send a ping originating from the host to every connected guest.
    ///
    /// The room reducer still owns validation/routing for the in-memory
    /// representation; this method is the network side of that same action.
    pub fn send_ping(&self, kind: u16) -> Result<(), RoomError> {
        let mut peers = self.peers.lock().map_err(|_| RoomError::NetworkIo)?;
        let message = format!("PING host {kind}\n");
        let mut stale = Vec::new();
        for (member, stream) in peers.iter() {
            let result = stream
                .lock()
                .map_err(|_| RoomError::NetworkIo)
                .and_then(|mut stream| {
                    stream
                        .write_all(message.as_bytes())
                        .map_err(|_| RoomError::NetworkIo)
                });
            if result.is_err() {
                stale.push(member.clone());
            }
        }
        for member in stale {
            peers.remove(&member);
        }
        Ok(())
    }
}

impl Drop for LanHost {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.listener.take();
    }
}

fn accept_loop(
    listener: TcpListener,
    room: Arc<Mutex<Room>>,
    stop: Arc<AtomicBool>,
    peers: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>,
) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                // Do not let a half-open or stalled discovery connection hold
                // a worker forever before it sends the JOIN line.
                // Some platforms inherit the listener's nonblocking mode on
                // accepted sockets; restore blocking mode for the protocol.
                if stream.set_nonblocking(false).is_err() {
                    continue;
                }
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
                let room = Arc::clone(&room);
                let peers = Arc::clone(&peers);
                thread::spawn(move || handle_connection(stream, room, peers));
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }
}

fn handle_connection(
    mut stream: TcpStream,
    room: Arc<Mutex<Room>>,
    peers: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>,
) {
    let request = {
        let mut line = String::new();
        let mut reader = BufReader::new(match stream.try_clone() {
            Ok(stream) => stream,
            Err(_) => return,
        });
        if reader.read_line(&mut line).is_err() {
            return;
        }
        line
    };
    let parts: Vec<&str> = request.trim_end().splitn(3, ' ').collect();
    if parts.len() != 3 || parts[0] != "JOIN" {
        let _ = stream.write_all(b"ERR invalid_request\n");
        return;
    }
    let result = room
        .lock()
        .map_err(|_| RoomError::RoomEnded)
        .and_then(|mut room| {
            let invite = InviteCode::new(parts[1])?;
            room.join(&invite, parts[2])
        });
    match result {
        Ok(member) => {
            let writer = match stream.try_clone() {
                Ok(writer) => Arc::new(Mutex::new(writer)),
                Err(_) => return,
            };
            if let Ok(mut peers) = peers.lock() {
                peers.insert(member.as_str().to_owned(), Arc::clone(&writer));
            }
            // Register the peer before acknowledging the JOIN. Otherwise a
            // host broadcast sent immediately after connect() can race the
            // insertion and be silently missed by the newly joined client.
            let _ = stream.write_all(format!("OK {}\n", member.as_str()).as_bytes());
            let mut reader = BufReader::new(stream);
            loop {
                let mut command = String::new();
                if reader.read_line(&mut command).is_err() || command.is_empty() {
                    break;
                }
                let command = command.trim_end();
                if let Some(kind) = command.strip_prefix("PING ") {
                    broadcast_ping(&peers, member.as_str(), kind);
                } else if let Some(length) = command.strip_prefix("FRAME ") {
                    let Ok(length) = length.parse::<usize>() else {
                        break;
                    };
                    if length > 4 * 1024 * 1024 {
                        break;
                    }
                    let mut frame = vec![0_u8; length];
                    if reader.read_exact(&mut frame).is_err() {
                        break;
                    }
                    broadcast_frame(&peers, member.as_str(), &frame);
                }
            }
            if let Ok(mut peers) = peers.lock() {
                peers.remove(member.as_str());
            }
        }
        Err(error) => {
            let _ = stream.write_all(format!("ERR {:?}\n", error).as_bytes());
        }
    }
}

fn broadcast_ping(
    peers: &Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>,
    sender: &str,
    kind: &str,
) {
    let message = format!("PING {} {}\n", sender, kind);
    if let Ok(mut peers) = peers.lock() {
        let mut stale = Vec::new();
        for (member, stream) in peers.iter() {
            if member == sender {
                continue;
            }
            if let Ok(mut stream) = stream.lock() {
                if stream.write_all(message.as_bytes()).is_err() {
                    stale.push(member.clone());
                }
            } else {
                stale.push(member.clone());
            }
        }
        for member in stale {
            peers.remove(&member);
        }
    }
}

fn broadcast_frame(
    peers: &Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>,
    sender: &str,
    frame: &[u8],
) {
    let header = format!("FRAME {}\n", frame.len());
    if let Ok(mut peers) = peers.lock() {
        let mut stale = Vec::new();
        for (member, stream) in peers.iter() {
            if member == sender {
                continue;
            }
            if let Ok(mut stream) = stream.lock() {
                if stream
                    .write_all(header.as_bytes())
                    .and_then(|_| stream.write_all(frame))
                    .is_err()
                {
                    stale.push(member.clone());
                }
            } else {
                stale.push(member.clone());
            }
        }
        for member in stale {
            peers.remove(&member);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanMessage {
    Ping { sender: String, kind: u16 },
    Frame(Vec<u8>),
}

pub struct LanClient {
    pub member_id: String,
    stream: TcpStream,
    incoming: Arc<Mutex<VecDeque<LanMessage>>>,
    reader_stop: Arc<AtomicBool>,
    reader: Option<thread::JoinHandle<()>>,
}

impl LanClient {
    pub fn send_ping(&mut self, kind: u16) -> Result<(), RoomError> {
        self.stream
            .write_all(format!("PING {kind}\n").as_bytes())
            .map_err(|_| RoomError::NetworkIo)
    }

    pub fn send_frame(&mut self, frame: &[u8]) -> Result<(), RoomError> {
        if frame.len() > 4 * 1024 * 1024 {
            return Err(RoomError::NetworkIo);
        }
        self.stream
            .write_all(format!("FRAME {}\n", frame.len()).as_bytes())
            .and_then(|_| self.stream.write_all(frame))
            .map_err(|_| RoomError::NetworkIo)
    }

    /// Returns and clears messages received from the host. A dedicated reader
    /// keeps the TCP receive window drained while the UI is idle or busy.
    pub fn poll_messages(&self) -> Vec<LanMessage> {
        let Ok(mut incoming) = self.incoming.lock() else {
            return Vec::new();
        };
        incoming.drain(..).collect()
    }
}

impl Drop for LanClient {
    fn drop(&mut self) {
        self.reader_stop.store(true, Ordering::Release);
        let _ = self.stream.shutdown(Shutdown::Both);
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

pub fn join(host: &str, port: u16, invite: &str, nickname: &str) -> Result<String, RoomError> {
    Ok(connect(host, port, invite, nickname)?.member_id.clone())
}

pub fn connect(
    host: &str,
    port: u16,
    invite: &str,
    nickname: &str,
) -> Result<LanClient, RoomError> {
    if host.trim().is_empty() || port == 0 {
        return Err(RoomError::InvalidAddress);
    }
    let addresses: Vec<SocketAddr> = (host.trim(), port)
        .to_socket_addrs()
        .map_err(|_| RoomError::InvalidAddress)?
        .collect();
    if addresses.is_empty() {
        return Err(RoomError::InvalidAddress);
    }

    // A Bonjour hostname can resolve to both IPv6 and IPv4 addresses. Trying
    // only the first result made joins fail whenever that interface was
    // unreachable (common with VPNs, sleep/wake, and Wi-Fi handoff).
    let mut last_error = RoomError::ConnectionFailed;
    for address in addresses {
        match connect_once(address, invite, nickname) {
            Ok(client) => return Ok(client),
            Err(error @ RoomError::InvalidInvite)
            | Err(error @ RoomError::RoomEnded)
            | Err(error @ RoomError::Capacity)
            | Err(error @ RoomError::InvalidName) => return Err(error),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

fn connect_once(address: SocketAddr, invite: &str, nickname: &str) -> Result<LanClient, RoomError> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))
        .map_err(|_| RoomError::ConnectionFailed)?;
    stream.set_nodelay(true).map_err(|_| RoomError::NetworkIo)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| RoomError::NetworkIo)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| RoomError::NetworkIo)?;
    stream
        .write_all(format!("JOIN {} {}\n", invite, nickname).as_bytes())
        .map_err(|_| RoomError::NetworkIo)?;
    // Read only through the handshake newline. A BufReader on a cloned TCP
    // stream can read ahead and discard an immediate host broadcast before
    // the background receiver is started.
    let response = read_protocol_line(&mut stream, 4096).map_err(|_| RoomError::NetworkIo)?;
    let parts: Vec<&str> = response.trim_end().splitn(2, ' ').collect();
    match parts.as_slice() {
        ["OK", member] if !member.is_empty() => {
            let reader_stream = stream.try_clone().map_err(|_| RoomError::NetworkIo)?;
            let incoming = Arc::new(Mutex::new(VecDeque::new()));
            let reader_stop = Arc::new(AtomicBool::new(false));
            let thread_incoming = Arc::clone(&incoming);
            let thread_stop = Arc::clone(&reader_stop);
            let reader = thread::spawn(move || {
                drain_server_messages(reader_stream, thread_stop, thread_incoming)
            });
            Ok(LanClient {
                member_id: (*member).to_owned(),
                stream,
                incoming,
                reader_stop,
                reader: Some(reader),
            })
        }
        ["ERR", reason] => Err(match *reason {
            "RoomEnded" => RoomError::RoomEnded,
            "InvalidInvite" => RoomError::InvalidInvite,
            "Capacity" => RoomError::Capacity,
            "InvalidName" => RoomError::InvalidName,
            _ => RoomError::NetworkIo,
        }),
        _ => Err(RoomError::NetworkIo),
    }
}

fn read_protocol_line(stream: &mut TcpStream, max_length: usize) -> std::io::Result<String> {
    let mut bytes = Vec::with_capacity(64);
    while bytes.len() < max_length {
        let mut byte = [0_u8; 1];
        stream.read_exact(&mut byte)?;
        bytes.push(byte[0]);
        if byte[0] == b'\n' {
            return String::from_utf8(bytes)
                .map_err(|_| std::io::Error::new(ErrorKind::InvalidData, "invalid protocol"));
        }
    }
    Err(std::io::Error::new(
        ErrorKind::InvalidData,
        "protocol line too long",
    ))
}

fn drain_server_messages(
    stream: TcpStream,
    stop: Arc<AtomicBool>,
    incoming: Arc<Mutex<VecDeque<LanMessage>>>,
) {
    let mut reader = BufReader::new(stream);
    loop {
        if stop.load(Ordering::Acquire) {
            return;
        }
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return,
            Ok(_) => {
                let command = line.trim_end();
                if let Some(payload) = command.strip_prefix("PING ") {
                    let mut parts = payload.splitn(2, ' ');
                    let Some(sender) = parts.next().filter(|value| !value.is_empty()) else {
                        continue;
                    };
                    let Some(kind) = parts.next().and_then(|value| value.parse::<u16>().ok())
                    else {
                        continue;
                    };
                    enqueue_message(
                        &incoming,
                        LanMessage::Ping {
                            sender: sender.to_owned(),
                            kind,
                        },
                    );
                } else if let Some(length) = command.strip_prefix("FRAME ") {
                    let Ok(length) = length.parse::<usize>() else {
                        return;
                    };
                    if length > 4 * 1024 * 1024 {
                        return;
                    }
                    let mut frame = vec![0_u8; length];
                    if reader.read_exact(&mut frame).is_err() {
                        return;
                    }
                    enqueue_message(&incoming, LanMessage::Frame(frame));
                }
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return,
        }
    }
}

fn enqueue_message(incoming: &Arc<Mutex<VecDeque<LanMessage>>>, message: LanMessage) {
    if let Ok(mut incoming) = incoming.lock() {
        const MAX_QUEUED_MESSAGES: usize = 32;
        if incoming.len() >= MAX_QUEUED_MESSAGES {
            incoming.pop_front();
        }
        incoming.push_back(message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_accepts_join_over_tcp() {
        let (host, port) = LanHost::start("Host", "123456", 0).unwrap();
        let member = join("127.0.0.1", port, "123456", "Guest").unwrap();
        assert!(!member.is_empty());
        assert_eq!(host.room.lock().unwrap().members().count(), 2);
    }

    #[test]
    fn connect_can_fall_back_when_hostname_returns_multiple_addresses() {
        let (host, port) = LanHost::start("Host", "123456", 0).unwrap();
        let client = connect("localhost", port, "123456", "Guest").unwrap();
        assert!(!client.member_id.is_empty());
        assert_eq!(host.room.lock().unwrap().members().count(), 2);
    }

    #[test]
    fn client_drains_and_exposes_messages_from_host() {
        let (host, port) = LanHost::start("Host", "123456", 0).unwrap();
        let client = connect("127.0.0.1", port, "123456", "Guest").unwrap();
        host.send_ping(7).unwrap();
        for _ in 0..20 {
            let messages = client.poll_messages();
            if messages
                == vec![LanMessage::Ping {
                    sender: "host".to_owned(),
                    kind: 7,
                }]
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("host message was not delivered to the client");
    }
}
