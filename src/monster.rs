use crate::graphics::hash2;
use crate::world::{has_line_of_sight, terrain_y, tile_at};
use macroquad::prelude::*;

#[derive(PartialEq, Clone, Copy)]
pub enum AiPhase { Searching, Stalking, Chasing, Fleeing }

pub struct Mimic {
    pub pos: Vec3, pub yaw: f32, pub phase: AiPhase, pub timer: f32,
    pub grace_timer: f32, pub stun_exposure: f32, pub gait_phase: f32, pub frozen: bool,
    pub target_xz: Vec2, pub learned_hearing: f32, pub learned_flash_resist: f32,
    pub learned_ambush: f32, pub stealth_evades: u8, pub evade_cd: f32, pub learned_tech: f32, pub quad_blend: f32,
    pub persuasion_msg: Option<(String, Vec3, f32)>, pub persuade_cd: f32, pub disguised: bool, pub mimic_talking_t: f32, pub persuade_patience: f32, pub echo_cd: f32, pub distant_echo: bool, pub distant_scream: bool, pub flee_t: f32,
}

impl Mimic {
    pub fn new(_tex: &Texture2D) -> Self {
        Self {
            pos: vec3(190.0, 0.0, 210.0), yaw: 0.0, phase: AiPhase::Searching, timer: 4.0,
            grace_timer: 2.0, stun_exposure: 0.0, gait_phase: 0.0, frozen: false,
            target_xz: vec2(190.0, 190.0), learned_hearing: 0.0, learned_flash_resist: 0.0,
            learned_ambush: 0.0, stealth_evades: 0, evade_cd: 0.0, learned_tech: 0.0, quad_blend: 0.0,
            persuasion_msg: None, persuade_cd: 2.0, disguised: false, mimic_talking_t: 0.0, persuade_patience: 5.2, echo_cd: 11.0, distant_echo: false, distant_scream: false, flee_t: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32, player_pos: Vec3, look_dir: Vec3, flash_on: bool, noise: f32, fixed_gens: usize, has_radar: bool, flares: &[(Vec3, f32)], outposts: &[(i32, i32); 9]) -> (bool, bool, bool) {
        let prev_phase = self.phase; if self.flee_t > 0.0 { self.flee_t = (self.flee_t - dt).max(0.0); }
        self.distant_echo = false;
        self.distant_scream = false;
        self.echo_cd -= dt;
        if self.phase == AiPhase::Searching && self.echo_cd <= 0.0 && self.grace_timer <= 0.0 {
            self.echo_cd = 15.0 + ((get_time() * 7.0) as f32 % 8.0);
            if ((get_time() * 10.0) as u32) % 2 == 0 { self.distant_echo = true; } else { self.distant_scream = true; }
        }
        let p_xz = vec2(player_pos.x, player_pos.z);
        let m_xz = vec2(self.pos.x, self.pos.z);
        let dist = (p_xz - m_xz).length().max(0.01);
        let on_tower = player_pos.y > terrain_y(player_pos.x, player_pos.z) + 4.5;
        let near_flare = flares.iter().find(|&&(fp, ttl)| ttl > 0.0 && (vec2(fp.x, fp.z) - m_xz).length() < 11.0).map(|&(fp, _)| vec2(fp.x, fp.z));
        if let Some(f_xz) = near_flare {
            self.phase = AiPhase::Fleeing;
            self.disguised = false;
            self.flee_t = 6.5;
            self.grace_timer = 14.0;
            self.persuade_patience = 5.5;
            let away = (m_xz - f_xz).normalize_or_zero();
            self.target_xz = vec2((m_xz.x + away.x * 65.0).clamp(24.0, 356.0), (m_xz.y + away.y * 65.0).clamp(24.0, 356.0));
        } else if on_tower && self.phase != AiPhase::Fleeing {
            self.phase = AiPhase::Fleeing;
            self.disguised = false;
            self.flee_t = 4.5;
            self.grace_timer = 10.0;
            self.persuade_patience = 5.5;
            let away = (m_xz - p_xz).normalize_or_zero();
            self.target_xz = vec2((m_xz.x + away.x * 48.0).clamp(24.0, 356.0), (m_xz.y + away.y * 48.0).clamp(24.0, 356.0));
        }
        if self.phase == AiPhase::Fleeing {
            self.disguised = false;
            if self.flee_t <= 0.0 || (self.target_xz - m_xz).length() < 3.5 {
                self.phase = AiPhase::Searching;
                self.timer = 2.0;
            }
        } else {
            self.phase = AiPhase::Searching;
            self.disguised = false;
            let away = (m_xz - p_xz).normalize_or_zero();
            self.target_xz = vec2((m_xz.x + away.x * 38.0).clamp(24.0, 356.0), (m_xz.y + away.y * 38.0).clamp(24.0, 356.0));
        }
        let to_p = (p_xz - m_xz) / dist;
        let facing = vec2(self.yaw.cos(), self.yaw.sin());

        // Escalate intelligence as player repairs generators, avoids it, or uses tech
        let tech_lvl = (fixed_gens as f32) * 0.16 + if has_radar { 0.25 } else { 0.0 };
        self.learned_tech = self.learned_tech.max(tech_lvl.min(1.0));
        if noise > 0.72 { self.learned_hearing = (self.learned_hearing + dt * 0.45).min(10.0); }
        if !flash_on && noise < 0.25 { self.learned_ambush = (self.learned_ambush + dt * 0.22).min(1.0); }

        let sight_range = 24.0 + self.learned_tech * 12.0;
        let sees_player = dist < sight_range && facing.dot(to_p) > 0.28;
        let hear_range = 48.0 + self.learned_hearing * 3.5 + self.learned_tech * 20.0;

        if self.grace_timer > 0.0 { self.grace_timer -= dt; self.phase = AiPhase::Searching; }

        let to_m_3d = (vec3(self.pos.x, self.pos.y + 1.5, self.pos.z) - player_pos).normalize();
        if self.evade_cd > 0.0 { self.evade_cd = (self.evade_cd - dt).max(0.0); }
        let is_crouching = player_pos.y - terrain_y(player_pos.x, player_pos.z) < 1.35;
        let stealth_hidden = is_crouching && !flash_on && 3 > self.stealth_evades;
        if stealth_hidden && (self.phase == AiPhase::Chasing || self.phase == AiPhase::Stalking) && 28.0 > dist {
            if 0.0 >= self.evade_cd {
                self.stealth_evades += 1;
                self.evade_cd = 6.0;
            }
            self.phase = AiPhase::Searching;
            self.disguised = false;
            self.grace_timer = 4.5;
            self.timer = 0.0;
        }
        let lit = flash_on && dist < 16.0 && look_dir.dot(to_m_3d) > 0.88;
        self.frozen = false;

        // Tick down active persuasion lure
        if let Some((_, _, ref mut ttl)) = self.persuasion_msg {
            *ttl -= dt;
            if *ttl <= 0.0 { self.persuasion_msg = None; }
        }
        self.persuade_cd -= dt; if self.mimic_talking_t > 0.0 { self.mimic_talking_t -= dt; }

        match self.phase {
            AiPhase::Searching => { self.disguised = false;
                self.timer -= dt;
                if self.timer <= 0.0 || (self.target_xz - m_xz).length() < 4.5 {
                    let ang = (get_time() as f32) * 0.45 + self.pos.x * 0.05;
                    let wx = (p_xz.x + ang.cos() * 18.0).clamp(26.0, 354.0);
                    let wz = (p_xz.y + ang.sin() * 18.0).clamp(26.0, 354.0);
                    self.target_xz = vec2(wx, wz);
                    self.timer = 4.0;
                }
                if false {
                    self.timer = 6.0;
                    let a = (get_time() as f32) * 1.15 + self.pos.x * 0.1;
                    // Predictive flanking when player avoids it in stealth
                    let lead = if self.learned_ambush > 0.45 { vec2(look_dir.x, look_dir.z) * 16.0 } else { Vec2::ZERO };
                    let rad = (38.0 - self.learned_tech * 14.0).max(20.0);
                    self.target_xz = vec2((p_xz.x + lead.x + a.cos() * rad).clamp(22.0, 358.0), (p_xz.y + lead.y + a.sin() * rad).clamp(22.0, 358.0));
                }
                if self.grace_timer <= 0.0 && !on_tower {
                    if !stealth_hidden && (sees_player || (noise > 0.42 && dist < hear_range)) {
                        self.phase = AiPhase::Stalking;
                        self.timer = 11.0;
                        self.stun_exposure = 0.0;
                        self.persuade_patience = 5.2;
                        self.persuade_cd = 0.5;
                    }
                }
            }
            AiPhase::Stalking => {
                self.disguised = dist > 3.2 && self.grace_timer <= 0.0;
                if self.disguised && dist < 5.6 {
                    self.target_xz = m_xz;
                } else {
                    self.target_xz = p_xz;
                }
                self.timer -= dt;
                if dist < 26.0 { self.persuade_patience -= dt; }
                // Persuasion lure when stalking nearby in the dark
                if self.persuade_cd <= 0.0 && dist > 10.0 && dist < 34.0 && !lit {
                    self.persuade_cd = 22.0;
                    let lures = [
                        "\"OVER HERE... I FOUND A JERRYCAN BY THE TREES...\"",
                        "\"DON'T SHINE YOUR LIGHT, IT'S ME! COME QUICK...\"",
                        "\"HEY... HELP ME FIX THIS GENERATOR OVER HERE...\"",
                        "\"I SEE THE ENTITY... FOLLOW MY VOICE, HURRY...\"",
                    ];
                    let idx = ((get_time() * 10.0) as usize) % lures.len();
                    self.persuasion_msg = Some((lures[idx].to_string(), self.pos, 4.5));
                }
                let max_stun = if self.disguised { 1.1 } else { (2.4 - self.learned_flash_resist - self.learned_tech * 0.6).max(0.55) };
                if lit {
                    self.frozen = true;
                    self.stun_exposure += dt;
                    self.learned_flash_resist = (self.learned_flash_resist + dt * 0.30).min(1.8);
                    if self.stun_exposure > max_stun { self.phase = AiPhase::Chasing; self.timer = 4.8; }
                } else {
                    self.stun_exposure = (self.stun_exposure - dt * 0.35).max(0.0);
                }
                if !on_tower && near_flare.is_none() && self.persuade_patience <= 0.0 { self.persuade_patience = 5.5;
                    self.phase = AiPhase::Chasing; self.timer = 4.8;
                } else if self.timer <= 0.0 && dist > 18.0 {
                    self.phase = AiPhase::Searching; self.timer = 0.0;
                }
            }
            AiPhase::Fleeing => {} AiPhase::Chasing => { self.disguised = false;
                self.timer -= dt;
                if !stealth_hidden && (sees_player || noise > 0.50 || 6.5 > dist) {
                    self.target_xz = p_xz;
                    if noise > 0.65 || dist < 5.5 { self.timer = 4.2; }
                }
                if self.timer <= 0.0 && (self.target_xz - m_xz).length() < 3.0 {
                    self.phase = AiPhase::Searching; self.timer = 0.0;
                }
            }
        }
        // Smoothly transition between upright stalking (0.0) and quadrupedal gallop (1.0)
        let target_quad = if self.phase == AiPhase::Chasing || self.flee_t > 0.0 { 1.0 } else { 0.0 };
        self.quad_blend += (target_quad - self.quad_blend) * (5.5 * dt).min(1.0);

        // Smart 9-ray obstacle-avoidance pathfinding around cabins, rocks & walls
        let desired = (self.target_xz - m_xz).normalize_or_zero();
        let base_ang = desired.y.atan2(desired.x);
        let mut best_dir = desired;
        let mut best_score = -999.0f32;
        for step in -4..=4 {
            let ang = base_ang + (step as f32) * 0.42;
            let dir = vec2(ang.cos(), ang.sin());
            let mut clear = 4.0f32;
            for d in [1.0f32, 2.0, 3.0, 4.0] {
                let probe = m_xz + dir * d;
                if tile_at(probe.x as i32, probe.y as i32, outposts) == '#' { clear = d - 1.0; break; }
            }
            let score = clear * 2.2 + dir.dot(desired) * 1.8;
            if score > best_score { best_score = score; best_dir = dir; }
        }

        if 0.01 > best_dir.length_squared() { best_dir = desired; } if best_dir.length_squared() > 0.01 {
            let target_yaw = best_dir.y.atan2(best_dir.x);
            let diff = (target_yaw - self.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            self.yaw += diff * (8.0 * dt).min(1.0);
        }

        let speed = if self.frozen || (self.disguised && dist < 5.6 && self.flee_t <= 0.0) { 0.0 } else if self.flee_t > 0.0 { 8.2 } else {
            match self.phase {
                AiPhase::Searching => 2.95 + self.learned_tech * 0.45,
                AiPhase::Stalking => 3.75 + self.learned_tech * 0.55,
                AiPhase::Chasing => 6.55 + self.learned_tech * 0.30, AiPhase::Fleeing => 8.4,
            }
        };
        if speed > 0.0 {
            self.gait_phase += dt * speed * 3.0;
            let nx = self.pos.x + best_dir.x * speed * dt;
            let nz = self.pos.z + best_dir.y * speed * dt;
            if tile_at(nx as i32, self.pos.z as i32, outposts) != '#' { self.pos.x = nx; }
            if tile_at(self.pos.x as i32, nz as i32, outposts) != '#' { self.pos.z = nz; }
        }
        self.pos.y = terrain_y(self.pos.x, self.pos.z);
        (dist < 1.35 && self.grace_timer <= 0.0 && has_line_of_sight(m_xz, p_xz, outposts), prev_phase != AiPhase::Chasing && self.phase == AiPhase::Chasing, prev_phase == AiPhase::Chasing && self.phase == AiPhase::Searching)
    }

    pub fn draw(&self, tex: &Texture2D, player_tex: &Texture2D, grabbing: bool) {
        if self.disguised {
            let t = get_time() as f32;
            let moving_d = (self.target_xz - vec2(self.pos.x, self.pos.z)).length() > 0.35;
            let stride = if moving_d { (t * 7.5 + self.pos.x * 2.0 + self.pos.z * 2.0).sin() * 0.36 } else { 0.0 };
            let root = Mat4::from_translation(self.pos) * Mat4::from_rotation_y(-self.yaw + std::f32::consts::FRAC_PI_2);
            let limb = |pos: Vec3, ang: f32, sz: Vec3, col: Color| {
                unsafe { get_internal_gl().quad_gl.push_model_matrix(root * Mat4::from_translation(pos) * Mat4::from_rotation_x(ang)); }
                draw_cube(vec3(0.0, -sz.y * 0.5, 0.0), sz, Some(player_tex), col);
                unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
            };
            limb(vec3(-0.12, 0.84, 0.0), stride, vec3(0.14, 0.84, 0.16), Color::from_rgba(28, 36, 48, 255));
            limb(vec3( 0.12, 0.84, 0.0), -stride, vec3(0.14, 0.84, 0.16), Color::from_rgba(28, 36, 48, 255));
            limb(vec3(0.0, 1.44, 0.0), 0.0, vec3(0.42, 0.60, 0.24), Color::from_rgba(48, 68, 44, 255));
            limb(vec3(-0.28, 1.40, 0.0), -stride * 0.7, vec3(0.12, 0.54, 0.12), Color::from_rgba(48, 68, 44, 255));
            limb(vec3( 0.28, 1.40, 0.0), -1.25, vec3(0.12, 0.54, 0.12), Color::from_rgba(48, 68, 44, 255));
            limb(vec3(0.0, 1.74, 0.0), (t * 2.2).sin() * 0.06, vec3(0.24, 0.26, 0.24), Color::from_rgba(210, 175, 145, 255));
            limb(vec3(0.0, 1.82, 0.0), 0.0, vec3(0.28, 0.07, 0.28), Color::from_rgba(24, 26, 30, 255));
            limb(vec3(0.28, 0.96, 0.18), -1.25, vec3(0.06, 0.22, 0.06), Color::from_rgba(255, 248, 210, 255));
            return;
        }
        if self.disguised {
            let t = get_time() as f32;
            let stride = (t * 6.5 + self.pos.x * 2.0 + self.pos.z * 2.0).sin() * 0.35;
            let root = Mat4::from_translation(self.pos) * Mat4::from_rotation_y(-self.yaw + std::f32::consts::FRAC_PI_2);
            let limb = |pos: Vec3, ang: f32, sz: Vec3, col: Color| {
                unsafe { get_internal_gl().quad_gl.push_model_matrix(root * Mat4::from_translation(pos) * Mat4::from_rotation_x(ang)); }
                draw_cube(vec3(0.0, -sz.y * 0.5, 0.0), sz, Some(tex), col);
                unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
            };
            limb(vec3(-0.12, 0.84, 0.0), stride, vec3(0.14, 0.84, 0.16), Color::from_rgba(28, 36, 48, 255));
            limb(vec3( 0.12, 0.84, 0.0), -stride, vec3(0.14, 0.84, 0.16), Color::from_rgba(28, 36, 48, 255));
            limb(vec3(0.0, 1.44, 0.0), 0.0, vec3(0.42, 0.60, 0.24), Color::from_rgba(48, 68, 44, 255));
            limb(vec3(-0.28, 1.40, 0.0), -stride * 0.7, vec3(0.12, 0.54, 0.12), Color::from_rgba(48, 68, 44, 255));
            limb(vec3( 0.28, 1.40, 0.0), -1.15, vec3(0.12, 0.54, 0.12), Color::from_rgba(48, 68, 44, 255));
            limb(vec3(0.0, 1.74, 0.0), (t * 2.5).sin() * 0.08, vec3(0.24, 0.26, 0.24), Color::from_rgba(210, 175, 145, 255));
            return;
        }
        let t = get_time() as f32;
        let qb = self.quad_blend;
        let stride = self.gait_phase.sin() * (0.45 + qb * 0.45);
        let gallop = (self.gait_phase * 2.0).sin().abs() * 0.22 * qb;
        let hide = Color::from_rgba(24, 24, 26, 255);
        let bone = Color::from_rgba(218, 212, 198, 255);
        let gore = Color::from_rgba(128, 22, 18, 255);
        let eye_col = if self.phase == AiPhase::Chasing || grabbing { RED } else if self.phase == AiPhase::Stalking { ORANGE } else { Color::from_rgba(12, 12, 14, 255) };

        let hip_h = 1.08 - qb * 0.28 + gallop;
        let spine_pitch = 0.22 + qb * 1.18;
        let root = Mat4::from_translation(self.pos + vec3(0.0, hip_h, 0.0)) * Mat4::from_rotation_y(-self.yaw + std::f32::consts::FRAC_PI_2);

        let part = |m: Mat4, sz: Vec3, col: Color| {
            unsafe { get_internal_gl().quad_gl.push_model_matrix(m); }
            draw_cube(Vec3::ZERO, sz, Some(tex), col);
            unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
        };

        // Gaunt Torso + Exposed Ribcage & Jagged Spine Ridges
        let torso = root * Mat4::from_rotation_x(spine_pitch);
        part(torso * Mat4::from_translation(vec3(0.0, 0.52, 0.0)), vec3(0.44, 1.05, 0.28), hide);
        for r in 0..5 {
            let ry = 0.28 + (r as f32) * 0.14;
            part(torso * Mat4::from_translation(vec3(0.0, ry, 0.12)), vec3(0.46, 0.045, 0.08), bone);
            part(torso * Mat4::from_translation(vec3(0.0, ry + 0.05, -0.17)), vec3(0.07, 0.12, 0.12), bone);
        }

        // Reverse-Jointed Hind Legs
        for (sx, ph) in [(-0.18f32, stride), (0.18, -stride)] {
            let hip = root * Mat4::from_translation(vec3(sx, 0.0, -0.05)) * Mat4::from_rotation_x(ph * 0.85 - 0.28);
            part(hip * Mat4::from_translation(vec3(0.0, -0.28, 0.0)), vec3(0.15, 0.56, 0.17), hide);
            let knee = hip * Mat4::from_translation(vec3(0.0, -0.54, 0.0)) * Mat4::from_rotation_x(0.55 - ph * 0.65);
            part(knee * Mat4::from_translation(vec3(0.0, -0.28, 0.0)), vec3(0.11, 0.58, 0.13), hide);
            part(knee * Mat4::from_translation(vec3(0.0, -0.56, 0.08)), vec3(0.13, 0.08, 0.26), gore);
        }

        // Long Gaunt Forearms & Blood-Stained Claws
        let sh_base = torso * Mat4::from_translation(vec3(0.0, 0.96, 0.0));
        for (sx, ph) in [(-0.30f32, -stride), (0.30, stride)] {
            let arm_ang = if grabbing { -1.35 } else { -spine_pitch + ph * 0.95 - qb * 0.15 };
            let sh = sh_base * Mat4::from_translation(vec3(sx, 0.0, 0.0)) * Mat4::from_rotation_x(arm_ang);
            part(sh * Mat4::from_translation(vec3(0.0, -0.34, 0.0)), vec3(0.12, 0.68, 0.12), hide);
            let el = sh * Mat4::from_translation(vec3(0.0, -0.66, 0.0)) * Mat4::from_rotation_x(-0.25 - qb * 0.2);
            part(el * Mat4::from_translation(vec3(0.0, -0.34, 0.0)), vec3(0.10, 0.68, 0.11), gore);
            for c in [-0.04f32, 0.0, 0.04] {
                part(el * Mat4::from_translation(vec3(c, -0.74, 0.05)), vec3(0.022, 0.20, 0.025), bone);
            }
        }

        // Bleached Stag Skull, Hollow Eye Sockets, Jagged Jaw & Thorny Bloody Antlers
        let twitch = (t * 6.5).sin() * 0.08;
        let head = sh_base * Mat4::from_translation(vec3(0.0, 0.18, 0.12)) * Mat4::from_rotation_x(-spine_pitch * 0.72) * Mat4::from_rotation_z(twitch);
        part(head * Mat4::from_translation(vec3(0.0, 0.12, 0.06)), vec3(0.28, 0.26, 0.34), bone);
        part(head * Mat4::from_translation(vec3(0.0, 0.07, 0.28)), vec3(0.17, 0.14, 0.26), bone);
        part(head * Mat4::from_translation(vec3(0.0, -0.04, 0.22)), vec3(0.14, 0.06, 0.24), gore);
        part(head * Mat4::from_translation(vec3(-0.11, 0.13, 0.19)), vec3(0.05, 0.07, 0.07), eye_col);
        part(head * Mat4::from_translation(vec3( 0.11, 0.13, 0.19)), vec3(0.05, 0.07, 0.07), eye_col);

        for side in [-1.0f32, 1.0] {
            let ant = head * Mat4::from_translation(vec3(side * 0.11, 0.24, -0.02)) * Mat4::from_rotation_z(-side * 0.42) * Mat4::from_rotation_x(-0.22);
            part(ant * Mat4::from_translation(vec3(0.0, 0.26, 0.0)), vec3(0.045, 0.54, 0.045), gore);
            part(ant * Mat4::from_translation(vec3(side * 0.12, 0.46, 0.08)) * Mat4::from_rotation_z(-side * 0.35), vec3(0.038, 0.36, 0.038), bone);
            part(ant * Mat4::from_translation(vec3(-side * 0.06, 0.22, 0.11)) * Mat4::from_rotation_x(0.65), vec3(0.028, 0.22, 0.028), gore);
            part(ant * Mat4::from_translation(vec3(side * 0.08, 0.38, 0.12)) * Mat4::from_rotation_x(0.55), vec3(0.026, 0.20, 0.026), bone);
        }
    }
}











