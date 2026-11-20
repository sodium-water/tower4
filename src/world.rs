use crate::graphics::{hash2, make_sphere_mesh};
use macroquad::prelude::*;

use std::sync::atomic::{AtomicU32, Ordering};
pub const WORLD_SIZE: i32 = 380;
pub static WORLD_SEED: AtomicU32 = AtomicU32::new(1);
pub fn set_world_seed(s: u32) { WORLD_SEED.store(s.max(1), Ordering::Relaxed); }
pub fn get_world_seed() -> u32 { WORLD_SEED.load(Ordering::Relaxed) }
pub const TOWERS: [(f32, f32); 4] = [(42.0, 42.0), (315.0, 58.0), (62.0, 318.0), (322.0, 322.0)];

pub fn random_outposts(seed: u32) -> [(i32, i32); 9] {
    let mut out = [(0, 0); 9];
    let mut attempt = 0u32;
    for i in 0..9 {
        loop {
            attempt += 1;
            let cx = 36 + (hash2(seed.wrapping_mul(17), attempt * 13 + i as u32) * 308.0) as i32;
            let cz = 36 + (hash2(attempt * 29 + i as u32, seed.wrapping_mul(31)) * 308.0) as i32;
            let mut ok = ((cx - 190).pow(2) + (cz - 190).pow(2)) > 26 * 26;
            for &(tx, tz) in &TOWERS {
                if ((cx as f32 - tx).powi(2) + (cz as f32 - tz).powi(2)) < 36.0 * 36.0 { ok = false; }
            }
            for j in 0..i {
                if ((cx - out[j].0).pow(2) + (cz - out[j].1).pow(2)) < 52 * 52 { ok = false; }
            }
            if ok || attempt > 4000 {
                out[i] = (cx, cz);
                break;
            }
        }
    }
    out
}

pub fn terrain_y(x: f32, z: f32) -> f32 {
    let s = get_world_seed();
    let p1 = hash2(s, 3) * 6.283;
    let p2 = hash2(s, 7) * 6.283;
    let p3 = hash2(s, 13) * 6.283;
    let h1 = (x * 0.042 + p1).sin() * (z * 0.042 + p2).cos() * 3.1;
    let h2 = ((x - z) * 0.024 + p3).sin() * 2.1;
    let mut flat = 1.0f32;
    for &(tx, tz) in &TOWERS {
        let d = ((x - tx).powi(2) + (z - tz).powi(2)).sqrt();
        flat = flat.min(((d - 6.0) / 8.0).clamp(0.0, 1.0));
    }
    let d_camp = ((x - 190.0).powi(2) + (z - 190.0).powi(2)).sqrt();
    flat = flat.min(((d_camp - 12.0) / 10.0).clamp(0.0, 1.0));
    (h1 + h2) * flat
}
pub fn walkable_y(x: f32, z: f32, outposts: &[(i32, i32); 9]) -> f32 {
    let mut base = terrain_y(x, z).max(terrain_y(x + 0.25, z + 0.25)).max(terrain_y(x - 0.25, z - 0.25));
    for &(tx, tz) in &TOWERS {
        let t_base = terrain_y(tx, tz);
        if ((tx - 2.85)..=(tx + 2.85)).contains(&x) && ((tz - 2.85)..=(tz + 4.15)).contains(&z) {
            if z <= tz + 2.88 || x >= tx + 1.42 { return t_base + 6.75; }
            if ((tx - 3.85)..=(tx + 1.8)).contains(&x) {
                base = base.max(t_base + ((x - (tx - 3.8)) / 5.2).clamp(0.0, 1.0) * 6.75);
            }
        } else if ((tx - 3.85)..=(tx - 2.85)).contains(&x) && ((tz + 2.85)..=(tz + 4.15)).contains(&z) {
            base = base.max(t_base + ((x - (tx - 3.8)) / 5.2).clamp(0.0, 1.0) * 6.75);
        }
    }
    base
}

pub fn tile_at(x: i32, z: i32, outposts: &[(i32, i32); 9]) -> char {
    if x <= 6 || z <= 6 || x >= WORLD_SIZE - 7 || z >= WORLD_SIZE - 7 { return 'M'; }
    for &(tx, tz) in &TOWERS {
        if (x - tx as i32).abs() <= 8 && (z - tz as i32).abs() <= 8 { return '.'; }
    }
    if (x - 190).abs() <= 14 && (z - 190).abs() <= 14 { return '.'; }
    for &(cx, cz) in outposts {
        let (x0, x1, z0, z1) = (cx - 3, cx + 3, cz - 3, cz + 3);
        if (x == x0 || x == x1) && (z0..=z1).contains(&z) { return '#'; }
        if (z == z0 || z == z1) && (x0..=x1).contains(&x) && x != cx { return '#'; }
        if (x - cx).abs() <= 6 && (z - cz).abs() <= 6 { return '.'; }
    }
    let s = get_world_seed(); let t_off1 = ((hash2(s, 21) - 0.5) * 40.0) as i32; let trail1 = 32 + t_off1 + ((x as f32 * 0.08 + hash2(s, 25) * 6.0).sin() * 14.0) as i32 + ((x - 32) / 2);
    let t_off2 = ((hash2(s, 31) - 0.5) * 40.0) as i32; let trail2 = 340 + t_off2 - ((x as f32 * 0.07 + hash2(s, 35) * 6.0).cos() * 14.0) as i32 - ((x - 32) / 2);
    if trail_dist(x as f32 + 0.5, z as f32 + 0.5, outposts) < 2.3 { return '.'; }
    let (gx, gz) = (x / 4, z / 4);
    let cell_r = hash2(gx as u32 * 73 + 11 + s, gz as u32 * 97 + 29 + s * 3);
    if cell_r > 0.14 {
        let tx = gx * 4 + 1 + (hash2(gx as u32 * 19, gz as u32 * 43) * 2.0) as i32;
        let tz = gz * 4 + 1 + (hash2(gx as u32 * 53, gz as u32 * 67) * 2.0) as i32;
        if x == tx && z == tz {
            return if cell_r > 0.82 { 'R' } else if cell_r > 0.58 { 'O' } else { 'P' };
        }
    }
    '.'
}
pub fn can_walk_to(x: f32, y: f32, z: f32, outposts: &[(i32, i32); 9], gens: &[(Vec3, f32, bool, bool)], chests: &[(Vec3, f32, bool, u8)]) -> bool {
    if x < 11.5 || z < 11.5 || x > (WORLD_SIZE as f32 - 11.5) || z > (WORLD_SIZE as f32 - 11.5) { return false; }
    if tile_at(x as i32, z as i32, outposts) != '.' { return false; }
    for tz_i in (z.floor() as i32 - 1)..=(z.floor() as i32 + 1) {
        for tx_i in (x.floor() as i32 - 1)..=(x.floor() as i32 + 1) {
            let tt = tile_at(tx_i, tz_i, outposts);
            if tt == 'P' || tt == 'O' || tt == 'R' {
                let d2 = (x - (tx_i as f32 + 0.5)).powi(2) + (z - (tz_i as f32 + 0.5)).powi(2);
                if 0.62 * 0.62 > d2 { return false; }
            }
        }
    }

    // Solid Generators & Iron Loot Chests
    for (gp, _, _, _) in gens {
        if (x - gp.x).abs() < 0.65 && (z - gp.z).abs() < 0.55 { return false; }
    }
    for (cp, _, _, _) in chests {
        if (x - cp.x).abs() < 0.55 && (z - cp.z).abs() < 0.42 { return false; }
    }

    // Solid Cabin Interior Furniture (Workbench & Bunk Bed)
    for &(ox, oz) in outposts {
        let (px, pz) = (ox as f32 + 0.5, oz as f32 + 0.5);
        if (x - (px - 2.1)).abs() < 0.58 && (z - (pz - 1.2)).abs() < 1.22 { return false; }
        if (x - (px + 1.9)).abs() < 1.02 && (z - (pz - 2.2)).abs() < 0.62 { return false; }
    }

    // Solid Abandoned Logging Camp Timber Piles (around 190, 190)
    for &(lx, lz) in &[(184.0, 185.0), (196.0, 186.0), (185.0, 196.0), (195.0, 195.0)] {
        if (x - lx).abs() < 1.8 && (z - lz).abs() < 0.95 { return false; }
    }

    // All 4 Climbable Watchtowers + Solid Radio Desk on Top
    for &(tx, tz) in &TOWERS {
        if (x - tx).abs() < 5.0 && (z - tz).abs() < 5.0 {
            let rel_y = y - terrain_y(tx, tz);
            if rel_y < 6.1 {
                for px in [tx - 1.8, tx + 1.55] { for pz in [tz - 1.8, tz + 1.55] {
                    if (x - (px + 0.12)).abs() < 0.32 && (z - (pz + 0.12)).abs() < 0.32 { return false; }
                }}
                if ((tx - 2.2)..=(tx + 1.8)).contains(&x) && ((tz + 2.0)..=(tz + 3.3)).contains(&z) {
                    if rel_y + 1.1 < ((x - (tx - 3.2)) / 4.8).clamp(0.0, 1.0) * 6.75 { return false; }
                }
            } else {
                if !((tx - 2.72)..=(tx + 2.72)).contains(&x) || !((tz - 2.72)..=(tz + 3.95)).contains(&z) { return false; }
                let in_cab = ((tx - 1.65)..=(tx + 1.65)).contains(&x) && ((tz - 1.65)..=(tz + 1.65)).contains(&z);
                let inside = ((tx - 1.35)..=(tx + 1.35)).contains(&x) && ((tz - 1.35)..=(tz + 1.35)).contains(&z);
                let door = ((tx - 0.65)..=(tx + 0.65)).contains(&x) && ((tz + 1.2)..=(tz + 1.8)).contains(&z);
                if in_cab && !inside && !door { return false; }
                // Solid Radio Extraction Desk
                if (x - tx).abs() < 0.82 && (z - (tz - 1.08)).abs() < 0.35 { return false; }
            }
        }
    }
    true
}

pub fn trail_dist(x: f32, z: f32, outposts: &[(i32, i32); 9]) -> f32 {
    let mut pts = Vec::with_capacity(14);
    pts.push(vec2(42.0, 42.0));
    for &(ox, oz) in outposts { pts.push(vec2(ox as f32 + 0.5, oz as f32 + 5.2)); }
    pts.push(vec2(190.0, 190.0));
    pts.push(vec2(315.0, 58.0));
    pts.push(vec2(62.0, 318.0));
    let p = vec2(x, z);
    let mut best = 999.0f32;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let ab = b - a;
        let len2 = ab.length_squared().max(0.01);
        let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
        let bend = (t * std::f32::consts::PI).sin() * 4.5 * ((a.x * 0.13).sin());
        let perp = vec2(-ab.y, ab.x).normalize_or_zero() * bend;
        best = best.min((p - (a + ab * t + perp)).length());
    }
    best
}
pub fn build_terrain_chunks(dirt: &Texture2D, outposts: &[(i32, i32); 9]) -> Vec<(Vec3, Mesh)> {
    let mut chunks = Vec::new();
    let mut cz = 0;
    while cz < WORLD_SIZE {
        let mut cx = 0;
        while cx < WORLD_SIZE {
            let (mut vertices, mut indices) = (Vec::with_capacity(1600), Vec::with_capacity(2400));
            for z in cz..(cz + 20) {
                for x in cx..(cx + 20) {
                    let (x0, z0, x1, z1) = (x as f32, z as f32, x as f32 + 1.0, z as f32 + 1.0);
                    let b = vertices.len() as u16;
                    let td = trail_dist(x0, z0, outposts);
                    let vcol = if 1.65 > td { Color::from_rgba(155, 130, 98, 255) } else if 2.35 > td { Color::from_rgba(110, 96, 78, 255) } else { WHITE };
                    vertices.push(Vertex::new(x0, terrain_y(x0, z0), z0, x0 * 0.125, z0 * 0.125, vcol));
                    vertices.push(Vertex::new(x1, terrain_y(x1, z0), z0, x1 * 0.125, z0 * 0.125, vcol));
                    vertices.push(Vertex::new(x1, terrain_y(x1, z1), z1, x1 * 0.125, z1 * 0.125, vcol));
                    vertices.push(Vertex::new(x0, terrain_y(x0, z1), z1, x0 * 0.125, z1 * 0.125, vcol));
                    indices.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
                }
            }
            chunks.push((vec3(cx as f32 + 10.0, 0.0, cz as f32 + 10.0), Mesh { vertices, indices, texture: Some(dirt.clone()) }));
            cx += 20;
        }
        cz += 20;
    }
    chunks
}
pub fn has_line_of_sight(a: Vec2, b: Vec2, outposts: &[(i32, i32); 9]) -> bool {
    let d = b - a;
    let len = d.length();
    let steps = (len / 0.45).ceil() as i32;
    for i in 1..steps {
        let p = a + d * (i as f32 / steps as f32);
        if tile_at(p.x as i32, p.y as i32, outposts) == '#' { return false; }
    }
    true
}

pub fn cabin_slope_mat(x: f32, z: f32, outposts: &[(i32, i32); 9]) -> Mat4 {
    for &(cx, cz) in outposts {
        let (px, pz) = (cx as f32 + 0.5, cz as f32 + 0.5);
        if (x - px).abs() <= 4.2 && (z - pz).abs() <= 4.2 {
            let (yl, yr) = (terrain_y(px - 3.4, pz), terrain_y(px + 3.4, pz));
            let (yb, yf) = (terrain_y(px, pz - 3.4), terrain_y(px, pz + 3.4));
            let slope_n = vec3(yl - yr, 6.8, yb - yf).normalize();
            return Mat4::from_quat(Quat::from_rotation_arc(Vec3::Y, slope_n));
        }
    }
    Mat4::IDENTITY
}

pub fn draw_cabin_and_camp(outposts: &[(i32, i32); 9], cabin_mesh: &[Mesh], wood: &Texture2D, bark: &Texture2D, rock: &Texture2D, cam: Vec3) {
    let trim_col = Color::from_rgba(72, 52, 38, 255);
    for &(cx, cz) in outposts {
        let (px, pz) = (cx as f32 + 0.5, cz as f32 + 0.5);
        if (vec2(px, pz) - vec2(cam.x, cam.z)).length() > 65.0 { continue; }
        let gy = terrain_y(px, pz);
        let (yl, yr) = (terrain_y(px - 3.4, pz), terrain_y(px + 3.4, pz));
        let (yb, yf) = (terrain_y(px, pz - 3.4), terrain_y(px, pz + 3.4));
        let slope_n = vec3(yl - yr, 6.8, yb - yf).normalize();
        let cab_mat = Mat4::from_translation(vec3(px, gy, pz)) * Mat4::from_quat(Quat::from_rotation_arc(Vec3::Y, slope_n));
        unsafe { get_internal_gl().quad_gl.push_model_matrix(cab_mat); }
                for m in cabin_mesh { let mut cm = Mesh { vertices: m.vertices.clone(), indices: m.indices.clone(), texture: Some(wood.clone()) }; for v in &mut cm.vertices { v.uv = vec2(v.position.x * 0.45 + v.position.z * 0.45, v.position.y * 0.65); v.color = Color::from_rgba(148, 102, 64, 255).into(); } draw_mesh(&cm); }
        let log_col = Color::from_rgba(116, 74, 44, 255);
        let roof_col = Color::from_rgba(52, 42, 36, 255);
        let win_glow = Color::from_rgba(245, 185, 95, 255);
        // Horizontal stacked log corner notches & sill beams
        for ly in [0.45f32, 1.05, 1.65, 2.25, 2.85] {
            for sx in [-3.35f32, 3.35] {
                for sz in [-3.35f32, 3.35] {
                    draw_cube(vec3(sx, ly, sz), vec3(0.46, 0.22, 0.46), Some(bark), log_col);
                }
            }
        }
        // Overhanging Cedar-Shingle Gable Roof & Exposed Ridge Beam
        let r_left = Mat4::from_translation(vec3(-1.85, 3.72, 0.0)) * Mat4::from_rotation_z(0.46);
        unsafe { get_internal_gl().quad_gl.push_model_matrix(r_left); }
        draw_cube(Vec3::ZERO, vec3(4.35, 0.16, 8.2), Some(bark), roof_col);
        unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
        let r_right = Mat4::from_translation(vec3(1.85, 3.72, 0.0)) * Mat4::from_rotation_z(-0.46);
        unsafe { get_internal_gl().quad_gl.push_model_matrix(r_right); }
        draw_cube(Vec3::ZERO, vec3(4.35, 0.16, 8.2), Some(bark), roof_col);
        unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
        draw_cube(vec3(0.0, 4.52, 0.0), vec3(0.24, 0.24, 8.4), Some(wood), trim_col);
        // Covered Front Porch Deck, Posts & Railing
        draw_cube(vec3(0.0, 0.12, 4.15), vec3(6.8, 0.24, 1.75), Some(wood), trim_col);
        for px_p in [-3.1f32, -1.25, 1.25, 3.1] {
            draw_cube(vec3(px_p, 1.68, 4.85), vec3(0.18, 3.1, 0.18), Some(wood), log_col);
        }
        draw_cube(vec3(-2.18, 0.96, 4.85), vec3(1.85, 0.10, 0.12), Some(wood), trim_col);
        draw_cube(vec3( 2.18, 0.96, 4.85), vec3(1.85, 0.10, 0.12), Some(wood), trim_col);
        // Warm Glowing Amber Cabin Windows
        for sx in [-3.34f32, 3.34] {
            draw_cube(vec3(sx, 1.68, 0.0), vec3(0.06, 0.98, 1.18), Some(wood), win_glow);
        }
        draw_cube(vec3(0.0, -0.35, 0.0), vec3(6.9, 0.75, 6.9), Some(rock), Color::from_rgba(62, 58, 54, 255));
        draw_cube(vec3(2.35, 2.75, 2.15), vec3(0.82, 5.5, 0.82), Some(rock), Color::from_rgba(88, 92, 96, 255));
        for sx in [-3.2, 3.2] {
            draw_cube(vec3(sx, 1.68, 0.0), vec3(0.08, 1.14, 0.06), Some(bark), trim_col);
            draw_cube(vec3(sx, 1.68, 0.0), vec3(0.08, 0.06, 1.32), Some(bark), trim_col);
            draw_cube(vec3(sx, 1.12, 0.0), vec3(0.14, 0.06, 1.34), Some(bark), trim_col);
            draw_cube(vec3(sx, 2.24, 0.0), vec3(0.14, 0.06, 1.34), Some(bark), trim_col);
        }
        draw_cube(vec3(-2.1, 0.98, -1.2), vec3(0.85, 0.10, 2.2), Some(wood), trim_col);
        for lz in [-2.1, -0.3] { draw_cube(vec3(-2.1, 0.58, lz), vec3(0.75, 0.75, 0.12), Some(bark), trim_col); }
        draw_cube(vec3(1.9, 0.62, -2.2), vec3(1.8, 0.12, 0.95), Some(wood), trim_col);
        draw_cube(vec3(1.9, 1.62, -2.2), vec3(1.8, 0.12, 0.95), Some(wood), trim_col);
        draw_cube(vec3(0.0, 3.90, 0.0), vec3(0.18, 0.08, 0.18), Some(rock), Color::from_rgba(25, 25, 28, 255));
        draw_cube(vec3(0.0, 3.26, 0.0), vec3(0.025, 1.36, 0.025), Some(bark), Color::from_rgba(18, 18, 20, 255));
        draw_cube(vec3(0.0, 2.58, 0.0), vec3(0.07, 0.08, 0.07), Some(rock), Color::from_rgba(55, 58, 62, 255));
        let bulb_m = make_sphere_mesh(0.085, 8, 12, Some(wood), Color::from_rgba(255, 225, 145, 255));
        unsafe { get_internal_gl().quad_gl.push_model_matrix(Mat4::from_translation(vec3(0.0, 2.50, 0.0))); }
        draw_mesh(&bulb_m);
        unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
        let d_root = Mat4::from_translation(vec3(0.62, 0.28, 3.36)) * Mat4::from_rotation_y(1.38);
        unsafe { get_internal_gl().quad_gl.push_model_matrix(d_root); }
        draw_cube(vec3(-0.58, 1.08, 0.0), vec3(1.16, 2.16, 0.055), Some(wood), Color::from_rgba(105, 74, 48, 255));
        draw_cube(vec3(-0.58, 1.62, 0.0), vec3(0.86, 0.72, 0.075), Some(bark), Color::from_rgba(78, 54, 34, 255));
        draw_cube(vec3(-0.58, 0.62, 0.0), vec3(0.86, 0.78, 0.075), Some(bark), Color::from_rgba(78, 54, 34, 255));
        draw_cube(vec3(-0.14, 1.72, 0.0), vec3(0.24, 0.07, 0.085), Some(rock), Color::from_rgba(32, 34, 38, 255));
        draw_cube(vec3(-0.14, 0.48, 0.0), vec3(0.24, 0.07, 0.085), Some(rock), Color::from_rgba(32, 34, 38, 255));
        draw_cube(vec3(-1.02, 1.04, 0.0), vec3(0.07, 0.07, 0.12), Some(rock), Color::from_rgba(212, 175, 55, 255));
        unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
        unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
    }
    // Abandoned Logging Camp at (190, 190)
    if (vec2(190.0, 190.0) - vec2(cam.x, cam.z)).length() < 65.0 {
        for &(lx, lz) in &[(184.0, 185.0), (196.0, 186.0), (185.0, 196.0), (195.0, 195.0)] {
            let gy = terrain_y(lx, lz);
            for row in 0..3 {
                let count = 4 - row;
                for c in 0..count {
                    let oz = (c as f32 - (count as f32 - 1.0) * 0.5) * 0.34;
                    draw_cube(vec3(lx, gy + 0.18 + row as f32 * 0.30, lz + oz), vec3(3.2, 0.30, 0.30), Some(bark), Color::from_rgba(110, 86, 64, 255));
                }
            }
        }
    }
}

pub fn draw_campfire(raw_pos: Vec3, bark: &Texture2D, rock: &Texture2D) {
    let pos = vec3(raw_pos.x, terrain_y(raw_pos.x, raw_pos.z), raw_pos.z);
    let t = get_time() as f32;
    for i in 0..8 {
        let a = (i as f32 / 8.0) * std::f32::consts::TAU;
        draw_cube(pos + vec3(a.cos() * 0.52, 0.07, a.sin() * 0.52), vec3(0.22, 0.14, 0.20), Some(rock), Color::from_rgba(68, 72, 78, 255));
    }
    for ang in [0.4f32, 1.2, 2.0, 2.8] {
        let m = Mat4::from_translation(pos + vec3(0.0, 0.14, 0.0)) * Mat4::from_rotation_y(ang) * Mat4::from_rotation_z(0.28);
        unsafe { get_internal_gl().quad_gl.push_model_matrix(m); }
        draw_cube(Vec3::ZERO, vec3(0.74, 0.10, 0.10), Some(bark), Color::from_rgba(38, 28, 22, 255));
        unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
    }
    let mut verts = Vec::new();
    let mut idxs = Vec::new();
    for (k, (rad, h, col)) in [(0.34f32, 0.72f32, Color::from_rgba(255, 85, 18, 235)), (0.21, 0.54, Color::from_rgba(255, 195, 50, 245)), (0.11, 0.34, Color::from_rgba(255, 248, 200, 255))].iter().enumerate() {
        let sway_x = (t * (11.0 + k as f32 * 3.0) + pos.x).sin() * 0.07;
        let sway_z = (t * (9.5 + k as f32 * 2.5) + pos.z).cos() * 0.07;
        let b = verts.len() as u16;
        verts.push(Vertex::new(pos.x - rad, pos.y + 0.12, pos.z - rad, 0.0, 0.0, *col));
        verts.push(Vertex::new(pos.x + rad, pos.y + 0.12, pos.z - rad, 1.0, 0.0, *col));
        verts.push(Vertex::new(pos.x + rad, pos.y + 0.12, pos.z + rad, 1.0, 1.0, *col));
        verts.push(Vertex::new(pos.x - rad, pos.y + 0.12, pos.z + rad, 0.0, 1.0, *col));
        verts.push(Vertex::new(pos.x + sway_x, pos.y + 0.12 + h, pos.z + sway_z, 0.5, 0.5, *col));
        idxs.extend_from_slice(&[b, b+1, b+4, b+1, b+2, b+4, b+2, b+3, b+4, b+3, b, b+4]);
    }
    draw_mesh(&Mesh { vertices: verts, indices: idxs, texture: Some(rock.clone()) });
}
pub fn draw_forest_props(x: i32, z: i32, outposts: &[(i32, i32); 9], trees: &[(Mesh, Mesh); 3], bark: &Texture2D, rock: &Texture2D, detail: bool) {
    let t = tile_at(x, z, outposts);
    let (px, pz) = (x as f32 + 0.5, z as f32 + 0.5);
    let gy = terrain_y(px, pz);
    let seed = hash2(x as u32, z as u32);
    if t == 'P' || t == 'O' || t == 'R' {
        let idx = if t == 'P' { 0 } else if t == 'O' { 1 } else { 2 };
        let sc = 0.88 + seed * 0.48;
        let m = Mat4::from_translation(vec3(px, gy, pz)) * Mat4::from_rotation_y(seed * 6.28) * Mat4::from_scale(vec3(sc, sc, sc));
        unsafe { get_internal_gl().quad_gl.push_model_matrix(m); }
        draw_mesh(&trees[idx].0);
        draw_mesh(&trees[idx].1);
        unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
    } else if detail && t == '.' && seed > 0.965 && trail_dist(px, pz, outposts) > 2.6 {
        if seed > 0.985 {
            draw_cube(vec3(px + 0.12, gy + 0.14, pz - 0.1), vec3(0.46, 0.28, 0.46), Some(rock), Color::from_rgba(135, 140, 135, 255));
        } else {
            draw_cube(vec3(px, gy + 0.20, pz), vec3(0.52, 0.40, 0.52), Some(bark), Color::from_rgba(22, 50, 24, 255));
        }
    }
}