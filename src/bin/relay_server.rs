use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::Instant;

fn main() {
    let sock = UdpSocket::bind("0.0.0.0:7777").expect("Failed to bind UDP 7777");
    println!("TOWER 4 GLOBAL LOBBY RELAY ONLINE ON UDP 7777");
    let mut rooms: HashMap<String, HashMap<SocketAddr, Instant>> = HashMap::new();
    let mut buf = [0u8; 1500];
    loop {
        if let Ok((len, src)) = sock.recv_from(&mut buf) {
            if len > 4 && buf[1] == b'|' {
                if let Some(sep) = buf[2..len].iter().position(|&b| b == b'|') {
                    let code = String::from_utf8_lossy(&buf[2..2 + sep]).to_uppercase();
                    let room = rooms.entry(code).or_default();
                    room.insert(src, Instant::now());
                    room.retain(|_, t| t.elapsed().as_secs() < 10);
                    for &peer in room.keys() {
                        if peer != src { let _ = sock.send_to(&buf[..len], peer); }
                    }
                }
            }
        }
    }
}
