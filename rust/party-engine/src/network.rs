use crate::{BonjourPublisher, InviteCode, Room, RoomError};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
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
        let listener = TcpListener::bind(("0.0.0.0", port)).map_err(|_| RoomError::InvalidId)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| RoomError::InvalidId)?;
        let actual_port = listener
            .local_addr()
            .map_err(|_| RoomError::InvalidId)?
            .port();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_listener = listener.try_clone().map_err(|_| RoomError::InvalidId)?;
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
            let _ = stream.write_all(format!("OK {}\n", member.as_str()).as_bytes());
            let writer = match stream.try_clone() {
                Ok(writer) => Arc::new(Mutex::new(writer)),
                Err(_) => return,
            };
            if let Ok(mut peers) = peers.lock() {
                peers.insert(member.as_str().to_owned(), Arc::clone(&writer));
            }
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
    if let Ok(peers) = peers.lock() {
        for (member, stream) in peers.iter() {
            if member == sender {
                continue;
            }
            if let Ok(mut stream) = stream.lock() {
                let _ = stream.write_all(message.as_bytes());
            }
        }
    }
}

fn broadcast_frame(
    peers: &Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>,
    sender: &str,
    frame: &[u8],
) {
    let header = format!("FRAME {}\n", frame.len());
    if let Ok(peers) = peers.lock() {
        for (member, stream) in peers.iter() {
            if member == sender {
                continue;
            }
            if let Ok(mut stream) = stream.lock() {
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(frame);
            }
        }
    }
}

pub struct LanClient {
    pub member_id: String,
    stream: TcpStream,
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
}

pub fn join(host: &str, port: u16, invite: &str, nickname: &str) -> Result<String, RoomError> {
    Ok(connect(host, port, invite, nickname)?.member_id)
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
    let address = (host.trim(), port)
        .to_socket_addrs()
        .map_err(|_| RoomError::InvalidAddress)?
        .next()
        .ok_or(RoomError::InvalidAddress)?;
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))
        .map_err(|_| RoomError::ConnectionFailed)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| RoomError::NetworkIo)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| RoomError::NetworkIo)?;
    stream
        .write_all(format!("JOIN {} {}\n", invite, nickname).as_bytes())
        .map_err(|_| RoomError::NetworkIo)?;
    let mut response = String::new();
    BufReader::new(stream.try_clone().map_err(|_| RoomError::NetworkIo)?)
        .read_line(&mut response)
        .map_err(|_| RoomError::NetworkIo)?;
    let parts: Vec<&str> = response.trim_end().splitn(2, ' ').collect();
    match parts.as_slice() {
        ["OK", member] => Ok(LanClient {
            member_id: (*member).to_owned(),
            stream,
        }),
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
}
