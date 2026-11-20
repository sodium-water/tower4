use crate::monster::AiPhase; use crate::world::{get_world_seed, set_world_seed};
use crate::voice::VoiceChat;
use macroquad::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::{connect, Message};

pub const RELAY_URL: &str = "wss://tower4-relay.onrender.com";

#[derive(Clone)]
pub struct RemotePlayer {
    pub name: String, pub pos: Vec3, pub target_pos: Vec3, pub yaw: f32, pub pitch: f32,
    pub flash_on: bool, pub crouch: bool, pub talking: bool, pub last_seen: Instant,
}

pub enum NetRole { Offline, Host, Client }

pub struct NetManager {
    pub role: NetRole, pub tx: Option<Sender<Vec<u8>>>, pub rx: Option<Receiver<Vec<u8>>>,
    pub peers: HashMap<String, RemotePlayer>, pub lobby_code: String, pub status: String,
    pub my_id: String, pub my_name: String, pub server_online: Arc<AtomicBool>,
}

impl NetManager {
    pub fn new() -> Self {
        let id = format!("U{}", (macroquad::time::get_time() * 100000.0) as u64 % 999999);
        let online = Arc::new(AtomicBool::new(false));
        let online_bg = online.clone();
        thread::spawn(move || {
            loop {
                if let Ok((mut ws, _)) = connect(RELAY_URL) {
                    online_bg.store(true, Ordering::Relaxed);
                    let _ = ws.close(None);
                } else {
                    online_bg.store(false, Ordering::Relaxed);
                }
                thread::sleep(Duration::from_secs(12));
            }
        });
        Self { role: NetRole::Offline, tx: None, rx: None, peers: HashMap::new(), lobby_code: "TOWER4".into(), status: "SOLO".into(), my_id: id, my_name: "RANGER".into(), server_online: online }
    }
    fn start_ws(&mut self, is_host: bool, code: &str, player_name: &str) {
        self.lobby_code = if code.trim().is_empty() { "TOWER4".into() } else { code.trim().to_uppercase() };
        self.my_name = if player_name.trim().is_empty() { "RANGER".into() } else { player_name.trim().to_uppercase() };
        self.peers.clear();
        self.role = if is_host { NetRole::Host } else { NetRole::Client };
        self.status = format!("CONNECTING [{}]...", self.lobby_code);
        let (out_tx, out_rx) = channel::<Vec<u8>>();
        let (in_tx, in_rx) = channel::<Vec<u8>>();
        let url = RELAY_URL.to_string();
        let join_msg = format!("J|{}|{}", self.lobby_code, self.my_id);
        let online_flag = self.server_online.clone();
        thread::spawn(move || {
            loop {
                if let Ok((mut ws, _)) = connect(&url) {
                    online_flag.store(true, Ordering::Relaxed);
                    let _ = ws.send(Message::Text(join_msg.clone().into()));
                    if let tungstenite::stream::MaybeTlsStream::Plain(s) = ws.get_ref() { let _ = s.set_nonblocking(true); }
                    if let tungstenite::stream::MaybeTlsStream::NativeTls(s) = ws.get_ref() { let _ = s.get_ref().set_nonblocking(true); }
                    loop {
                        while let Ok(pkt) = out_rx.try_recv() {
                            if ws.send(Message::Binary(pkt.into())).is_err() { break; }
                        }
                        match ws.read() {
                            Ok(Message::Binary(b)) => { let _ = in_tx.send(b.to_vec()); }
                            Ok(Message::Text(t)) => { let _ = in_tx.send(t.as_bytes().to_vec()); }
                            Err(tungstenite::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                            Err(_) => { online_flag.store(false, Ordering::Relaxed); break; }
                            _ => {}
                        }
                        thread::sleep(Duration::from_millis(8));
                    }
                } else {
                    online_flag.store(false, Ordering::Relaxed);
                }
                thread::sleep(Duration::from_secs(2));
            }
        });
        self.tx = Some(out_tx); self.rx = Some(in_rx);
    }
    pub fn host_with_name(&mut self, code: &str, name: &str) { self.start_ws(true, code, name); }
    pub fn join_with_name(&mut self, code: &str, name: &str) { self.start_ws(false, code, name); }
}
impl NetManager {
    #[allow(clippy::too_many_arguments)]
    pub fn sync(&mut self, dt: f32, radio_active: bool, pos: Vec3, yaw: f32, pitch: f32, flash: bool, crouch: bool, voice: &VoiceChat, master_vol: f32, outposts: &mut [(i32, i32); 9], gens: &mut Vec<(Vec3, f32, bool, bool)>, mimic_pos: &mut Vec3, mimic_yaw: &mut f32, mimic_phase: &mut AiPhase) -> (f32, Vec3, bool) {
        let (Some(tx), Some(rx)) = (&self.tx, &self.rx) else { return (0.0, Vec3::ZERO, false); };
        let mut ops_changed = false;
        let my_talk = *voice.ptt_active.lock().unwrap();
        while let Ok(buf) = rx.try_recv() {
            if buf.len() > 6 && buf.starts_with(b"V|") {
                let s = String::from_utf8_lossy(&buf[2..buf.len().min(40)]);
                let mut sp = s.splitn(3, '|');
                if let (Some(code), Some(pid)) = (sp.next(), sp.next()) {
                    if code == self.lobby_code && pid != self.my_id && buf.len() > 4 + code.len() + pid.len() {
                        let pcm = buf[4 + code.len() + pid.len()..].to_vec();
                        let dist = self.peers.get(pid).map(|p| (p.pos - pos).length()).unwrap_or(5.0);
                        let max_r = if radio_active { 320.0 } else { 36.0 }; let gain = if dist > max_r { 0.0 } else if radio_active { 1.15 * master_vol } else { (1.5 / (1.0 + dist * 0.14)).clamp(0.15, 1.4) * master_vol };
                        voice.record_peer_packet(&pcm); if let Ok(mut iq) = voice.in_queue.lock() { if iq.len() < 10 { iq.push_back((pcm, gain)); } }
                    }
                }
                continue;
            }
            let msg = String::from_utf8_lossy(&buf);
            let p: Vec<&str> = msg.split('|').collect();
            let (p0, p1, p2) = (p.first().copied().unwrap_or(""), p.get(1).copied().unwrap_or(""), p.get(2).copied().unwrap_or(""));
            if p.len() >= 12 && p0 == "P" && p1 == self.lobby_code && p2 != self.my_id {
                let tp = vec3(p.get(3).and_then(|s| s.parse().ok()).unwrap_or(44.5), p.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0), p.get(5).and_then(|s| s.parse().ok()).unwrap_or(45.5));
                let entry = self.peers.entry(p2.to_string()).or_insert(RemotePlayer { name: "RANGER".into(), pos: tp, target_pos: tp, yaw: 0.0, pitch: 0.0, flash_on: false, crouch: false, talking: false, last_seen: Instant::now() });
                entry.target_pos = tp; entry.yaw = p.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.0); entry.pitch = p.get(7).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                entry.flash_on = p.get(8).copied() == Some("1"); entry.crouch = p.get(9).copied() == Some("1"); entry.talking = p.get(10).copied() == Some("1");
                entry.name = p.get(11).copied().unwrap_or("RANGER").to_string(); entry.last_seen = Instant::now();
                self.status = format!("CONNECTED [{}]", self.lobby_code);
                if let (Some(gi), Some(gp)) = (p.get(12), p.get(13)) { if let (Ok(idx), Ok(prog)) = (gi.parse::<usize>(), gp.parse::<f32>()) { if idx < gens.len() && prog > gens[idx].1 { gens[idx].1 = prog; if gens[idx].1 >= 4.5 { gens[idx].3 = true; } } } }
            } else if p.len() >= 42 && p0 == "W" && p1 == self.lobby_code && matches!(self.role, NetRole::Client) {
                if let Some(ws_str) = p.get(43) {
                    if let Ok(ws) = ws_str.parse::<u32>() {
                        if ws > 0 && ws != get_world_seed() { set_world_seed(ws); ops_changed = true; }
                    }
                }
                for i in 0..9 {
                    let ox = p.get(2 + i * 2).and_then(|s| s.parse().ok()).unwrap_or(outposts[i].0);
                    let oz = p.get(3 + i * 2).and_then(|s| s.parse().ok()).unwrap_or(outposts[i].1);
                    if outposts[i] != (ox, oz) { outposts[i] = (ox, oz); ops_changed = true; }
                }
                for i in 0..gens.len().min(6) {
                    gens[i].1 = p.get(20 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(gens[i].1);
                    gens[i].2 = p.get(21 + i * 3).copied() == Some("1");
                    gens[i].3 = p.get(22 + i * 3).copied() == Some("1");
                }
                *mimic_pos = vec3(p.get(38).and_then(|s| s.parse().ok()).unwrap_or(280.0), p.get(39).and_then(|s| s.parse().ok()).unwrap_or(0.0), p.get(40).and_then(|s| s.parse().ok()).unwrap_or(280.0));
                *mimic_yaw = p.get(41).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                *mimic_phase = match p.get(42).copied().unwrap_or("0") { "1" => AiPhase::Stalking, "2" => AiPhase::Chasing, "3" => AiPhase::Fleeing, _ => AiPhase::Searching };
            }
        }
        for pl in self.peers.values_mut() { pl.pos = pl.pos.lerp(pl.target_pos, (14.0 * dt).min(1.0)); }
        self.peers.retain(|_, pl| pl.last_seen.elapsed().as_secs() < 6);
        if let Ok(mut oq) = voice.out_queue.lock() { while let Some(pcm) = oq.pop_front() { let mut pkt = format!("V|{}|{}|", self.lobby_code, self.my_id).into_bytes(); pkt.extend_from_slice(&pcm); let _ = tx.send(pkt); } }
        let mut rep = String::from("-1|0");
        for (i, (gp, prog, _, fixed)) in gens.iter().enumerate() { if !*fixed && (vec2(pos.x, pos.z) - vec2(gp.x, gp.z)).length() < 2.6 && *prog > 0.0 { rep = format!("{}|{:.2}", i, prog); } }
        let p_pkt = format!("P|{}|{}|{:.2}|{:.2}|{:.2}|{:.2}|{:.2}|{}|{}|{}|{}|{}", self.lobby_code, self.my_id, pos.x, pos.y, pos.z, yaw, pitch, flash as u8, crouch as u8, my_talk as u8, self.my_name, rep);
        let _ = tx.send(p_pkt.into_bytes());
        if matches!(self.role, NetRole::Host) {
            let ph = match *mimic_phase { AiPhase::Searching => "0", AiPhase::Stalking => "1", AiPhase::Chasing => "2", AiPhase::Fleeing => "3" };
            let ops_s: String = outposts.iter().map(|(x, z)| format!("{}|{}", x, z)).collect::<Vec<_>>().join("|");
            let gens_s: String = (0..6).map(|i| gens.get(i).map(|g| format!("{:.2}|{}|{}", g.1, g.2 as u8, g.3 as u8)).unwrap_or("0|0|0".into())).collect::<Vec<_>>().join("|");
            let w_pkt = format!("W|{}|{}|{}|{:.2}|{:.2}|{:.2}|{:.2}|{}|{}", self.lobby_code, ops_s, gens_s, mimic_pos.x, mimic_pos.y, mimic_pos.z, *mimic_yaw, ph, get_world_seed());
            let _ = tx.send(w_pkt.into_bytes());
        }
        let (mut peer_pos, mut max_n) = (Vec3::ZERO, 0.0f32);
        for pl in self.peers.values() { let n = if pl.talking { 0.85 } else if pl.crouch { 0.14 } else { 0.50 }; if n > max_n { max_n = n; peer_pos = pl.pos; } }
        (max_n, peer_pos, ops_changed)
    }
    pub fn draw_teammates(&self, wood: &Texture2D) {
        let t = get_time() as f32;
        for p in self.peers.values() {
            let sy = if p.crouch { 0.72 } else { 1.0 };
            let stride = (t * 8.0 + p.pos.x * 2.2 + p.pos.z * 2.2).sin() * 0.38;
            let root = Mat4::from_translation(p.pos) * Mat4::from_rotation_y(-p.yaw + std::f32::consts::FRAC_PI_2) * Mat4::from_scale(vec3(1.0, sy, 1.0));
            let limb = |pos: Vec3, ang: f32, sz: Vec3, col: Color| {
                unsafe { get_internal_gl().quad_gl.push_model_matrix(root * Mat4::from_translation(pos) * Mat4::from_rotation_x(ang)); }
                draw_cube(vec3(0.0, -sz.y * 0.5, 0.0), sz, Some(wood), col);
                unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
            };
            limb(vec3(-0.12, 0.84, 0.0), stride, vec3(0.14, 0.84, 0.16), Color::from_rgba(28, 36, 48, 255));
            limb(vec3( 0.12, 0.84, 0.0), -stride, vec3(0.14, 0.84, 0.16), Color::from_rgba(28, 36, 48, 255));
            limb(vec3(0.0, 1.44, 0.0), 0.0, vec3(0.42, 0.60, 0.24), Color::from_rgba(48, 68, 44, 255));
            limb(vec3(-0.28, 1.40, 0.0), -stride * 0.7, vec3(0.12, 0.54, 0.12), Color::from_rgba(48, 68, 44, 255));
            limb(vec3( 0.28, 1.40, 0.0), if p.flash_on { -1.25 } else { stride * 0.7 }, vec3(0.12, 0.54, 0.12), Color::from_rgba(48, 68, 44, 255));
            limb(vec3(0.0, 1.74, 0.0), -p.pitch * 0.5, vec3(0.24, 0.26, 0.24), Color::from_rgba(210, 175, 145, 255));
        }
    }
    pub fn first_peer_name(&self) -> String {
        self.peers.values().next().map(|p| p.name.clone()).unwrap_or_else(|| "RANGER_02".into())
    }
    pub fn draw_fake_nametag(&self, fake_pos: Vec3, fake_talking: bool, cam_pos: Vec3, cam_dir: Vec3) {
        let name = self.first_peer_name();
        let head = fake_pos + vec3(0.0, 1.98, 0.0);
        let to_h = head - cam_pos;
        let dist = to_h.length();
        if dist < 35.0 && cam_dir.dot(to_h / dist.max(0.01)) > 0.35 {
            let fwd = cam_dir.normalize();
            let right = fwd.cross(Vec3::Y).normalize();
            let up = right.cross(fwd);
            let z_c = to_h.dot(fwd);
            let fov_scale = 1.0 / (68.0f32.to_radians() * 0.5).tan();
            let aspect = screen_width() / screen_height().max(1.0);
            let sx = (to_h.dot(right) / (z_c * aspect)) * fov_scale;
            let sy = (to_h.dot(up) / z_c) * fov_scale;
            let sp = vec2((sx * 0.5 + 0.5) * screen_width(), (0.5 - sy * 0.5) * screen_height());
            let tag = if fake_talking { format!("{} [TALKING]", name) } else { name };
            draw_text(&tag, sp.x - (tag.len() as f32 * 4.5), sp.y, 18.0, if fake_talking { GREEN } else { WHITE });
        }
    }
    pub fn draw_nametags(&self, cam_pos: Vec3, cam_dir: Vec3) {
        for p in self.peers.values() {
            let head = p.pos + vec3(0.0, if p.crouch { 1.45 } else { 1.98 }, 0.0);
            let to_h = head - cam_pos;
            let dist = to_h.length();
            if dist < 35.0 && cam_dir.dot(to_h / dist.max(0.01)) > 0.35 {
                let fwd = cam_dir.normalize();
                let right = fwd.cross(Vec3::Y).normalize();
                let up = right.cross(fwd);
                let z_c = to_h.dot(fwd);
                let fov_scale = 1.0 / (68.0f32.to_radians() * 0.5).tan();
                let aspect = screen_width() / screen_height().max(1.0);
                let sx = (to_h.dot(right) / (z_c * aspect)) * fov_scale;
                let sy = (to_h.dot(up) / z_c) * fov_scale;
                let sp = vec2((sx * 0.5 + 0.5) * screen_width(), (0.5 - sy * 0.5) * screen_height());
                
                let tag = if p.talking { format!("{} [TALKING]", p.name) } else { p.name.clone() };
                draw_text(&tag, sp.x - (tag.len() as f32 * 4.5), sp.y, 18.0, if p.talking { GREEN } else { WHITE });
            }
        }
    }
}










