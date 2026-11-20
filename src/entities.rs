use macroquad::prelude::*;
use crate::world::{terrain_y, tile_at};

pub struct GazeStalker {
    pub pos: Vec3,
    pub yaw: f32,
    pub charging: bool,
    pub grace: f32,
}

impl GazeStalker {
    pub fn new() -> Self {
        Self { pos: vec3(345.0, terrain_y(345.0, 345.0), 345.0), yaw: 0.0, charging: false, grace: 4.0 }
    }
    pub fn reset(&mut self) {
        self.pos = vec3(345.0, terrain_y(345.0, 345.0), 345.0);
        self.grace = 4.0;
        self.charging = false;
    }
    pub fn update(&mut self, dt: f32, player_pos: Vec3, look_dir: Vec3, flash_on: bool, eclipse: bool, outposts: &[(i32, i32); 9]) -> bool {
        if self.grace > 0.0 {
            self.grace = (self.grace - dt).max(0.0);
            self.pos.y = terrain_y(self.pos.x, self.pos.z);
            return false;
        }
        let p_xz = vec2(player_pos.x, player_pos.z);
        let s_xz = vec2(self.pos.x, self.pos.z);
        let to_p = p_xz - s_xz;
        let dist = to_p.length().max(0.1);

        let mut in_safe = player_pos.y > terrain_y(player_pos.x, player_pos.z) + 3.6;
        for &(ox, oz) in outposts {
            if 3.4 > (player_pos.x - (ox as f32 + 0.5)).abs() && 3.4 > (player_pos.z - (oz as f32 + 0.5)).abs() {
                in_safe = true;
            }
        }
                if in_safe {
            let ang = (get_time() as f32) * 0.35;
            let orbit = vec2((p_xz.x + ang.cos() * 28.0).clamp(32.0, 348.0), (p_xz.y + ang.sin() * 28.0).clamp(32.0, 348.0));
            let o_dir = (orbit - s_xz).normalize_or_zero();
            self.yaw = o_dir.y.atan2(o_dir.x);
            let nx = (self.pos.x + o_dir.x * 4.5 * dt).clamp(24.0, 356.0);
            let nz = (self.pos.z + o_dir.y * 4.5 * dt).clamp(24.0, 356.0);
            self.pos = vec3(nx, terrain_y(nx, nz), nz);
            self.charging = false;
            return false;
        }

        let to_s3 = (self.pos + vec3(0.0, 1.35, 0.0) - player_pos).normalize_or_zero();
        let looked_at = flash_on && 24.0 >= dist && look_dir.dot(to_s3) > 0.91;
        self.charging = dist > 24.0 || looked_at;

        let dir = to_p.normalize_or_zero();
        self.yaw = dir.y.atan2(dir.x);
        let spd = if dist > 24.0 {
            if eclipse { 11.8 } else { 10.5 }
        } else if looked_at {
            if eclipse { 8.6 } else { 7.8 }
        } else if eclipse {
            1.10
        } else {
            0.70
        };

        let step = dir * spd * dt;
        let mut nx = (self.pos.x + step.x).clamp(22.0, 358.0);
        let mut nz = (self.pos.z + step.y).clamp(22.0, 358.0);
        let mut blocked = tile_at(nx as i32, nz as i32, outposts) == '#';
        for &(ox, oz) in outposts {
            if 3.8 > (nx - (ox as f32 + 0.5)).abs() && 3.8 > (nz - (oz as f32 + 0.5)).abs() { blocked = true; }
        }
        if blocked {
            let side = vec2(-dir.y, dir.x) * spd * dt;
            nx = (self.pos.x + side.x).clamp(22.0, 358.0);
            nz = (self.pos.z + side.y).clamp(22.0, 358.0);
        }
        self.pos = vec3(nx, terrain_y(nx, nz), nz);
        1.35 > (vec2(self.pos.x, self.pos.z) - p_xz).length()
    }

    pub fn draw(&self, bark: &Texture2D, metal: &Texture2D) {
        let t = get_time() as f32;
        let sw = (t * if self.charging { 20.0 } else { 5.5 }).sin() * if self.charging { 0.42 } else { 0.14 };
        let twitch = if self.charging { (t * 37.0).sin() * 0.24 } else { (t * 3.1).sin() * 0.08 };
        let root = Mat4::from_translation(self.pos) * Mat4::from_rotation_y(-self.yaw + std::f32::consts::FRAC_PI_2);
        unsafe { get_internal_gl().quad_gl.push_model_matrix(root); }
        let flesh = Color::from_rgba(28, 26, 30, 255);
        let bone = Color::from_rgba(188, 182, 168, 255);
        // 7-foot-tall emaciated digitigrade legs
        draw_cube(vec3(-0.15, 0.65, sw * 0.30), vec3(0.09, 1.30, 0.09), Some(bark), flesh);
        draw_cube(vec3( 0.15, 0.65, -sw * 0.30), vec3(0.09, 1.30, 0.09), Some(bark), flesh);
        // Gaunt ribcage & protruding spine vertebrae
        draw_cube(vec3(0.0, 1.66, 0.04), vec3(0.36, 0.88, 0.20), Some(bark), flesh);
        for r in 0..5 {
            let ry = 1.40 + (r as f32) * 0.12;
            draw_cube(vec3(0.0, ry, 0.14), vec3(0.38, 0.04, 0.05), Some(metal), bone);
            draw_cube(vec3(0.0, ry, -0.09), vec3(0.07, 0.07, 0.11), Some(metal), bone);
        }
        // Elongated spider-like arms + razor bone claws
        for side in [-1.0f32, 1.0] {
            let arm_z = 0.18 + side * sw * 0.35;
            draw_cube(vec3(side * 0.28, 1.38, arm_z), vec3(0.075, 1.28, 0.075), Some(bark), flesh);
            for cl in -1..=1 {
                draw_cube(vec3(side * 0.28 + (cl as f32) * 0.024, 0.68, arm_z + 0.06), vec3(0.014, 0.24, 0.025), Some(metal), bone);
            }
        }
        // Tilted fractured porcelain-bone mask head + unhinged gaping jaw + 4 sunken eyes
        let head_m = Mat4::from_translation(vec3(0.0, 2.26, 0.10)) * Mat4::from_rotation_z(twitch) * Mat4::from_rotation_x(if self.charging { -0.22 } else { 0.12 });
        unsafe { get_internal_gl().quad_gl.push_model_matrix(head_m); }
        draw_cube(vec3(0.0, 0.06, 0.0), vec3(0.25, 0.28, 0.24), Some(metal), bone);
        let jaw_drop = if self.charging { -0.18 } else { -0.09 };
        draw_cube(vec3(0.0, jaw_drop, 0.02), vec3(0.16, 0.14, 0.18), Some(bark), Color::from_rgba(14, 10, 12, 255));
        let eye_col = if self.charging { RED } else { Color::from_rgba(235, 245, 255, 255) };
        for (ex, ey) in [(-0.065f32, 0.09f32), (0.065, 0.09), (-0.045, 0.03), (0.045, 0.03)] {
            draw_cube(vec3(ex, ey, 0.125), vec3(0.032, 0.026, 0.02), Some(metal), eye_col);
        }
        unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
        unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
    }
}

pub fn draw_breaker_substation(pos: Vec3, active: bool, locked: bool, metal: &Texture2D) {
    let m = Mat4::from_translation(pos);
    unsafe { get_internal_gl().quad_gl.push_model_matrix(m); }
    draw_cube(vec3(0.0, 0.85, 0.0), vec3(1.15, 1.70, 0.65), Some(metal), Color::from_rgba(48, 56, 62, 255));
    draw_cube(vec3(0.0, 1.75, 0.0), vec3(1.24, 0.12, 0.74), Some(metal), Color::from_rgba(32, 36, 40, 255));
    draw_cube(vec3(0.0, 0.95, 0.34), vec3(0.72, 0.92, 0.06), Some(metal), Color::from_rgba(26, 30, 34, 255));
    let lever_y = if locked || active { 1.15 } else { 0.78 };
    draw_cube(vec3(0.22, lever_y, 0.40), vec3(0.24, 0.08, 0.14), Some(metal), GOLD);
    let beacon = if locked { GREEN } else if active { YELLOW } else { RED };
    draw_cube(vec3(0.0, 1.92, 0.0), vec3(0.22, 0.22, 0.22), Some(metal), beacon);
    unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
}

pub fn draw_black_box(pos: Vec3, metal: &Texture2D) {
    let m = Mat4::from_translation(pos);
    unsafe { get_internal_gl().quad_gl.push_model_matrix(m); }
    draw_cube(vec3(0.0, 0.16, 0.0), vec3(0.46, 0.30, 0.34), Some(metal), ORANGE);
    for sx in [-0.22f32, 0.22] {
        for sz in [-0.16f32, 0.16] {
            draw_cube(vec3(sx, 0.16, sz), vec3(0.07, 0.32, 0.07), Some(metal), Color::from_rgba(22, 24, 28, 255));
        }
    }
    draw_cube(vec3(0.0, 0.33, 0.0), vec3(0.24, 0.05, 0.06), Some(metal), LIGHTGRAY);
    draw_cube(vec3(0.0, 0.20, 0.175), vec3(0.22, 0.05, 0.02), Some(metal), GREEN);
    unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
}

pub fn draw_fp_hands(
    cam: Vec3, yaw: f32, pitch: f32, bob: f32, equip_t: f32, grip: f32, slot: usize,
    uv_charges: i32, flare_count: i32, adren_count: i32, has_map: bool, has_radar: bool,
    wood: &Texture2D, metal: &Texture2D,
) {
    let skin = Color::from_rgba(206, 166, 136, 255);
    let shade = Color::from_rgba(182, 142, 114, 255);
    let sleeve = Color::from_rgba(44, 62, 42, 255);
    let cuff = Color::from_rgba(32, 46, 30, 255);
    let sw_x = (bob * 0.5).cos() * 0.014;
    let sw_y = bob.sin().abs() * 0.015;
    let p_norm = (equip_t / 0.38).clamp(0.0, 1.0);
    let pocket_dip = (p_norm * std::f32::consts::PI).sin();
    let show_new_item = 0.19 > equip_t;

    let base = Mat4::from_translation(cam)
        * Mat4::from_rotation_y(-yaw + std::f32::consts::FRAC_PI_2)
        * Mat4::from_rotation_x(-pitch);
    unsafe { get_internal_gl().quad_gl.push_model_matrix(base); }

    let rx = 0.22 + sw_x + pocket_dip * 0.06;
    let ry = -0.22 - sw_y - pocket_dip * 0.24;
    let rz_off = -pocket_dip * 0.16;

    // Smooth Cylindrical Right Forearm, Cuff, Wrist & Rounded Palm
    let arm_m = Mat4::from_translation(vec3(rx + 0.05, ry - 0.10, 0.24 + rz_off)) * Mat4::from_rotation_x(1.22) * Mat4::from_rotation_z(0.18);
    unsafe { get_internal_gl().quad_gl.push_model_matrix(arm_m); }
    draw_cylinder(vec3(0.0, 0.0, 0.0), 0.036, 0.044, 0.24, Some(wood), sleeve);
    draw_cylinder(vec3(0.0, 0.24, 0.0), 0.035, 0.038, 0.04, Some(wood), cuff);
    draw_cylinder(vec3(0.0, 0.28, 0.0), 0.027, 0.030, 0.06, Some(wood), skin);
    unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }

    draw_sphere(vec3(rx, ry, 0.475 + rz_off), 0.034, Some(wood), skin);
    draw_sphere(vec3(rx - 0.004, ry + 0.002, 0.505 + rz_off), 0.031, Some(wood), skin);

    // Smooth Articulated Thumb & 4 Tapered Cylindrical Fingers
    let g = grip * (1.0 - pocket_dip * 0.85);
    let thumb_in = g * 0.018;
    draw_sphere(vec3(rx - 0.032 + thumb_in * 0.5, ry + 0.022 - g * 0.006, 0.49 + rz_off), 0.012, Some(wood), shade);
    draw_sphere(vec3(rx - 0.040 + thumb_in, ry + 0.032 - g * 0.012, 0.522 + rz_off), 0.010, Some(wood), skin);

    for f in 0..4 {
        let fy = ry + 0.021 - (f as f32) * 0.014;
        let curl_x = g * 0.024;
        let ext_z = (1.0 - g) * 0.028;
        draw_sphere(vec3(rx - 0.016 - curl_x * 0.4, fy, 0.532 + ext_z * 0.5 + rz_off), 0.0085, Some(wood), shade);
        draw_sphere(vec3(rx - 0.026 - curl_x, fy - g * 0.006, 0.554 + ext_z - g * 0.018 + rz_off), 0.0075, Some(wood), skin);
    }

    // Smooth Cylindrical Left Forearm & Hand
    let lx = -0.22 + sw_x * 0.6;
    let ly = if slot == 4 && (has_map || has_radar) { -0.19 - sw_y - pocket_dip * 0.14 } else { -0.26 - sw_y };
    let larm_m = Mat4::from_translation(vec3(lx - 0.05, ly - 0.10, 0.23)) * Mat4::from_rotation_x(1.22) * Mat4::from_rotation_z(-0.18);
    unsafe { get_internal_gl().quad_gl.push_model_matrix(larm_m); }
    draw_cylinder(vec3(0.0, 0.0, 0.0), 0.036, 0.044, 0.23, Some(wood), sleeve);
    draw_cylinder(vec3(0.0, 0.23, 0.0), 0.035, 0.038, 0.04, Some(wood), cuff);
    draw_cylinder(vec3(0.0, 0.27, 0.0), 0.027, 0.030, 0.06, Some(wood), skin);
    unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }

    draw_cube(vec3(lx, ly + 0.026, 0.41), vec3(0.032, 0.011, 0.032), Some(metal), BLACK);
    draw_sphere(vec3(lx, ly, 0.455), 0.033, Some(wood), skin);
    draw_sphere(vec3(lx + 0.032, ly + 0.020, 0.470), 0.011, Some(wood), shade);
    for f in 0..4 {
        let fy = ly + 0.020 - (f as f32) * 0.014;
        draw_sphere(vec3(lx + 0.020, fy, 0.495), 0.008, Some(wood), shade);
    }

    if show_new_item {
        match slot {
            0 => {
                draw_cube(vec3(rx - 0.015, ry + 0.06, 0.51 + rz_off), vec3(0.060, 0.140, 0.040), Some(metal), Color::from_rgba(32, 36, 42, 255));
                draw_cube(vec3(rx - 0.015, ry + 0.09, 0.487 + rz_off), vec3(0.040, 0.036, 0.008), Some(metal), GREEN);
                draw_cube(vec3(rx + 0.012, ry + 0.18, 0.51 + rz_off), vec3(0.010, 0.13, 0.010), Some(metal), BLACK);
            }
            1 if uv_charges > 0 => {
                let cam_blue = Color::from_rgba(38, 76, 118, 255);
                let cam_blk = Color::from_rgba(20, 22, 26, 255);
                draw_cube(vec3(rx - 0.015, ry + 0.045, 0.51 + rz_off), vec3(0.066, 0.062, 0.112), Some(metal), cam_blue);
                draw_cube(vec3(rx - 0.015, ry + 0.045, 0.450 + rz_off), vec3(0.070, 0.066, 0.014), Some(metal), cam_blk);
                draw_cube(vec3(rx - 0.015, ry + 0.045, 0.570 + rz_off), vec3(0.070, 0.066, 0.016), Some(metal), cam_blk);
                for sx in [-0.040f32, 0.010] {
                    for sy in [0.016f32, 0.074] {
                        draw_cube(vec3(rx + sx, ry + sy, 0.573 + rz_off), vec3(0.008, 0.008, 0.018), Some(metal), LIGHTGRAY);
                    }
                }
                draw_cube(vec3(rx - 0.015, ry + 0.045, 0.600 + rz_off), vec3(0.054, 0.054, 0.042), Some(metal), cam_blk);
                draw_cube(vec3(rx - 0.015, ry + 0.045, 0.622 + rz_off), vec3(0.048, 0.048, 0.006), Some(metal), LIGHTGRAY);
                draw_cube(vec3(rx - 0.015, ry + 0.045, 0.624 + rz_off), vec3(0.036, 0.036, 0.006), Some(metal), Color::from_rgba(145, 70, 255, 255));
            }
            2 if flare_count > 0 => {
                draw_cube(vec3(rx - 0.015, ry + 0.05, 0.54 + rz_off), vec3(0.034, 0.034, 0.21), Some(metal), RED);
                draw_cube(vec3(rx - 0.015, ry + 0.05, 0.650 + rz_off), vec3(0.038, 0.038, 0.03), Some(metal), LIGHTGRAY);
            }
            3 if adren_count > 0 => {
                draw_cube(vec3(rx - 0.015, ry + 0.04, 0.53 + rz_off), vec3(0.026, 0.026, 0.14), Some(metal), SKYBLUE);
                draw_cube(vec3(rx - 0.015, ry + 0.04, 0.44 + rz_off), vec3(0.015, 0.015, 0.05), Some(metal), WHITE);
            }
            4 if has_map || has_radar => {
                draw_cube(vec3(0.0, ly + 0.07, 0.49 + rz_off), vec3(0.38, 0.24, 0.008), Some(wood), Color::from_rgba(218, 212, 192, 255));
                draw_cube(vec3(0.0, ly + 0.07, 0.485 + rz_off), vec3(0.34, 0.20, 0.004), Some(wood), Color::from_rgba(52, 78, 62, 255));
                draw_cube(vec3(-0.06, ly + 0.09, 0.482 + rz_off), vec3(0.14, 0.008, 0.004), Some(wood), Color::from_rgba(165, 135, 95, 255));
                draw_cube(vec3(0.04, ly + 0.04, 0.482 + rz_off), vec3(0.008, 0.11, 0.004), Some(wood), Color::from_rgba(165, 135, 95, 255));
                if has_radar {
                    draw_cube(vec3(0.13, ly + 0.07, 0.47 + rz_off), vec3(0.11, 0.15, 0.026), Some(metal), Color::from_rgba(28, 32, 36, 255));
                    draw_cube(vec3(0.13, ly + 0.075, 0.456 + rz_off), vec3(0.088, 0.10, 0.006), Some(metal), Color::from_rgba(20, 215, 145, 255));
                }
            }
            _ => {}
        }
    }

    unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
}

pub fn is_safe_haven(pos: Vec3, outposts: &[(i32, i32); 9]) -> bool {
    if pos.y > terrain_y(pos.x, pos.z) + 3.6 { return true; }
    for &(ox, oz) in outposts {
        if 3.4 > (pos.x - (ox as f32 + 0.5)).abs() && 3.4 > (pos.z - (oz as f32 + 0.5)).abs() {
            return true;
        }
    }
    false
}