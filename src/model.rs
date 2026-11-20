use crate::graphics::hash2;
use macroquad::prelude::*;
use std::f32::consts::TAU;
use std::fs;

pub fn load_custom_or_default(custom_path: &str, default_path: &str, tex: Option<&Texture2D>, tint: Color) -> Mesh {
    load_custom_chunks(custom_path, default_path, tex, tint).into_iter().next().unwrap_or_else(|| load_obj_mesh(default_path, tex, tint))
}
pub fn load_custom_chunks(custom_path: &str, default_path: &str, tex: Option<&Texture2D>, tint: Color) -> Vec<Mesh> {
    let alt_path = format!("{}.obj", custom_path);
    let path = if std::path::Path::new(custom_path).exists() { custom_path } else if std::path::Path::new(&alt_path).exists() { alt_path.as_str() } else { default_path };
    let text = fs::read_to_string(path).unwrap_or_default();
    let (mut pos, mut uvs, mut tris) = (Vec::with_capacity(8192), Vec::with_capacity(8192), Vec::with_capacity(24000));
    for line in text.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() >= 4 && p[0] == "v" {
            pos.push(vec3(p[1].parse().unwrap_or(0.0), p[2].parse().unwrap_or(0.0), p[3].parse().unwrap_or(0.0)));
        } else if p.len() >= 3 && p[0] == "vt" {
            uvs.push(vec2(p[1].parse().unwrap_or(0.0), 1.0 - p[2].parse::<f32>().unwrap_or(0.0)));
        } else if p.len() >= 4 && p[0] == "f" {
            let f: Vec<(usize, usize)> = p[1..].iter().filter_map(|s| {
                let mut sp = s.split('/');
                let raw_v: isize = sp.next()?.parse().ok()?;
                let raw_t: isize = sp.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let vi = if 0 > raw_v { (pos.len() as isize + raw_v + 1) as usize } else { raw_v as usize };
                let ti = if 0 > raw_t { (uvs.len() as isize + raw_t + 1) as usize } else { raw_t as usize };
                Some((vi, ti))
            }).collect();
            for k in 1..f.len().saturating_sub(1) {
                let (v0, t0) = f[0];
                let (v1, t1) = f[k];
                let (v2, t2) = f[k + 1];
                let a = pos.get(v0.saturating_sub(1)).copied().unwrap_or(Vec3::ZERO);
                let b = pos.get(v1.saturating_sub(1)).copied().unwrap_or(Vec3::ZERO);
                let c = pos.get(v2.saturating_sub(1)).copied().unwrap_or(Vec3::ZERO);
                let ua = uvs.get(t0.saturating_sub(1)).copied().unwrap_or(vec2(a.x * 0.3, a.y * 0.3));
                let ub = uvs.get(t1.saturating_sub(1)).copied().unwrap_or(vec2(b.x * 0.3, b.y * 0.3));
                let uc = uvs.get(t2.saturating_sub(1)).copied().unwrap_or(vec2(c.x * 0.3, c.y * 0.3));
                tris.push(((a, ua), (b, ub), (c, uc)));
            }
        }
    }
    if tris.is_empty() { return vec![load_obj_mesh(default_path, tex, tint)]; }
    let (mut min_v, mut max_v) = (vec3(99999.0, 99999.0, 99999.0), vec3(-99999.0, -99999.0, -99999.0));
    for ((a, _), (b, _), (c, _)) in &tris {
        min_v = min_v.min(*a).min(*b).min(*c);
        max_v = max_v.max(*a).max(*b).max(*c);
    }
    let sz = (max_v - min_v).max(vec3(0.001, 0.001, 0.001));
    let is_custom = path != default_path;
    let target_h = if !is_custom { sz.y } else if custom_path.contains("cabin") { 3.8 } else if custom_path.contains("generator") { 0.95 } else { 0.85 };
    let sc = if is_custom { target_h / sz.y } else { 1.0 };
    let (cx, cz) = if is_custom { ((min_v.x + max_v.x) * 0.5, (min_v.z + max_v.z) * 0.5) } else { (0.0, 0.0) };
    let min_y = if is_custom { min_v.y } else { 0.0 };

    let mut meshes = Vec::new();
    for chunk in tris.chunks(10000) {
        let (mut verts, mut idx) = (Vec::with_capacity(chunk.len() * 3), Vec::with_capacity(chunk.len() * 3));
        for ((a, ua), (b, ub), (c, uc)) in chunk {
            let base = verts.len() as u16;
            for (pt, uv) in [(*a, *ua), (*b, *ub), (*c, *uc)] {
                let nx = (pt.x - cx) / (sz.x * 0.5).max(0.001);
                let ny = (pt.y - min_y) / sz.y;
                let nz = (pt.z - cz) / (sz.z * 0.5).max(0.001);
                let col: [u8; 4] = if is_custom && custom_path.contains("generator") {
                    let outer_frame = nx.abs() > 0.88 || nz.abs() > 0.91 || ny > 0.92 || 0.08 > ny;
                    let side_plug = nx.abs() > 0.68 && 0.88 >= nx.abs() && ny > 0.30 && 0.62 > ny && nz > 0.05;
                    if side_plug { Color::from_rgba(28, 76, 198, 255).into() }
                    else if outer_frame { Color::from_rgba(58, 62, 68, 255).into() }
                    else if ny >= 0.54 { Color::from_rgba(175, 158, 62, 255).into() }
                    else { Color::from_rgba(178, 30, 26, 255).into() }
                } else if is_custom && custom_path.contains("cabin") {
                    if ny > 0.68 { Color::from_rgba(64, 50, 40, 255).into() }
                    else if 0.08 > ny { Color::from_rgba(86, 58, 38, 255).into() }
                    else { Color::from_rgba(142, 88, 48, 255).into() }
                } else { tint.into() };
                verts.push(Vertex::new((pt.x - cx) * sc, (pt.y - min_y) * sc, (pt.z - cz) * sc, uv.x, uv.y, Color::from_rgba(col[0], col[1], col[2], col[3])));
            }
            idx.extend_from_slice(&[base, base + 1, base + 2]);
        }
        meshes.push(Mesh { vertices: verts, indices: idx, texture: tex.cloned() });
    }
    meshes
}
pub fn load_obj_mesh(path: &str, tex: Option<&Texture2D>, tint: Color) -> Mesh {
    let text = fs::read_to_string(path).unwrap_or_default();
    let (mut pos, mut uvs, mut tris) = (Vec::with_capacity(4096), Vec::with_capacity(4096), Vec::with_capacity(12000));
    for line in text.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() >= 4 && p[0] == "v" {
            pos.push(vec3(p[1].parse().unwrap_or(0.0), p[2].parse().unwrap_or(0.0), p[3].parse().unwrap_or(0.0)));
        } else if p.len() >= 3 && p[0] == "vt" {
            uvs.push(vec2(p[1].parse().unwrap_or(0.0), 1.0 - p[2].parse::<f32>().unwrap_or(0.0)));
        } else if p.len() >= 4 && p[0] == "f" {
            let f: Vec<(usize, usize)> = p[1..].iter().filter_map(|s| {
                let mut sp = s.split('/');
                let raw_v: isize = sp.next()?.parse().ok()?;
                let raw_t: isize = sp.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let vi = if 0 > raw_v { (pos.len() as isize + raw_v + 1) as usize } else { raw_v as usize };
                let ti = if 0 > raw_t { (uvs.len() as isize + raw_t + 1) as usize } else { raw_t as usize };
                Some((vi, ti))
            }).collect();
            for k in 1..f.len().saturating_sub(1) {
                let (v0, t0) = f[0];
                let (v1, t1) = f[k];
                let (v2, t2) = f[k + 1];
                let a = pos.get(v0.saturating_sub(1)).copied().unwrap_or(Vec3::ZERO);
                let b = pos.get(v1.saturating_sub(1)).copied().unwrap_or(Vec3::ZERO);
                let c = pos.get(v2.saturating_sub(1)).copied().unwrap_or(Vec3::ZERO);
                let ua = uvs.get(t0.saturating_sub(1)).copied().unwrap_or(vec2(a.x * 0.3, a.y * 0.3));
                let ub = uvs.get(t1.saturating_sub(1)).copied().unwrap_or(vec2(b.x * 0.3, b.y * 0.3));
                let uc = uvs.get(t2.saturating_sub(1)).copied().unwrap_or(vec2(c.x * 0.3, c.y * 0.3));
                tris.push(((a, ua), (b, ub), (c, uc)));
            }
        }
    }
    let max_tris = 21500usize;
    let step = if tris.len() > max_tris { tris.len() as f32 / max_tris as f32 } else { 1.0 };
    let count = tris.len().min(max_tris);
    let (mut verts, mut idx) = (Vec::with_capacity(count * 3), Vec::with_capacity(count * 3));
    for i in 0..count {
        let ti = ((i as f32 * step) as usize).min(tris.len() - 1);
        let ((a, ua), (b, ub), (c, uc)) = tris[ti];
        let base = verts.len() as u16;
        verts.push(Vertex::new(a.x, a.y, a.z, ua.x, ua.y, tint));
        verts.push(Vertex::new(b.x, b.y, b.z, ub.x, ub.y, tint));
        verts.push(Vertex::new(c.x, c.y, c.z, uc.x, uc.y, tint));
        idx.extend_from_slice(&[base, base + 1, base + 2]);
    }
    Mesh { vertices: verts, indices: idx, texture: tex.cloned() }
}
fn push_box(obj: &mut String, vi: &mut usize, x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32) {
    for (x, y, z) in [(x0,y0,z0),(x1,y0,z0),(x1,y0,z1),(x0,y0,z1),(x0,y1,z0),(x1,y1,z0),(x1,y1,z1),(x0,y1,z1)] {
        obj.push_str(&format!("v {:.3} {:.3} {:.3}\n", x, y, z));
    }
    let b = *vi;
    for (a, c, d, e) in [(0,1,2,3),(4,5,6,7),(0,1,5,4),(2,3,7,6),(1,2,6,5),(0,3,7,4)] {
        obj.push_str(&format!("f {} {} {} {}\n", b+a, b+c, b+d, b+e));
    }
    *vi += 8;
}

fn push_cone(obj: &mut String, vc: &mut usize, cx: f32, cy: f32, cz: f32, rb: f32, rt: f32, h: f32, segs: usize, jag: f32, droop: f32, seed: u32) {
    let base = *vc;
    for ring in 0..=1 {
        let (y, r) = if ring == 0 { (cy, rb) } else { (cy + h, rt) };
        for s in 0..segs {
            let a = (s as f32 / segs as f32) * TAU;
            let w = if ring == 0 { 1.0 + (hash2(s as u32, seed) - 0.45) * jag } else { 1.0 };
            let d = if ring == 0 { -hash2(seed, s as u32) * droop } else { 0.0 };
            obj.push_str(&format!("v {:.3} {:.3} {:.3}\n", cx + a.cos() * r * w, y + d, cz + a.sin() * r * w));
            *vc += 1;
        }
    }
    for s in 0..segs {
        let n = (s + 1) % segs;
        obj.push_str(&format!("f {} {} {} {}\n", base + s, base + n, base + segs + n, base + segs + s));
    }
}

pub fn ensure_all_assets() {
    // 1. Tall Drooping Spruce/Pine (Separate Trunk + Canopy)
    let (mut pt, mut ptc, mut pc, mut pcc) = (String::new(), 1, String::new(), 1);
    push_cone(&mut pt, &mut ptc, 0.0, -0.35, 0.0, 0.34, 0.18, 1.1, 12, 0.25, 0.0, 1);
    push_cone(&mut pt, &mut ptc, 0.0, 0.65, 0.0, 0.18, 0.04, 5.8, 10, 0.08, 0.0, 2);
    for t in 0..10 {
        let tf = t as f32;
        push_cone(&mut pc, &mut pcc, (hash2(t, 5)-0.5)*0.14, 1.35 + tf*0.52, (hash2(t, 9)-0.5)*0.14, (1.68 - tf*0.15).max(0.24), 0.04, 1.15, 16, 0.52, 0.32, t+10);
    }
    let _ = fs::write("assets/pine_trunk.obj", pt);
    let _ = fs::write("assets/pine_canopy.obj", pc);

    // 2. Gnarled Forked Oak (Separate Trunk/Limbs + Leafy Canopy)
    let (mut ot, mut otc, mut oc, mut occ) = (String::new(), 1, String::new(), 1);
    push_cone(&mut ot, &mut otc, 0.0, -0.35, 0.0, 0.52, 0.24, 3.4, 12, 0.22, 0.0, 21);
    for (bx, bz, r_can) in [(-1.35, 0.65, 1.55), (1.45, -0.55, 1.45), (0.25, 1.35, 1.40), (-0.45, -1.25, 1.45), (0.0, 0.0, 1.75)] {
        push_cone(&mut ot, &mut otc, bx * 0.14, 1.85, bz * 0.14, 0.20, 0.09, 1.45, 8, 0.12, 0.0, 22);
        push_cone(&mut oc, &mut occ, bx, 2.95, bz, r_can, r_can * 0.45, 1.15, 14, 0.38, 0.18, 25);
        push_cone(&mut oc, &mut occ, bx, 3.95, bz, r_can * 0.72, 0.15, 0.85, 12, 0.32, 0.10, 28);
    }
    let _ = fs::write("assets/oak_trunk.obj", ot);
    let _ = fs::write("assets/oak_canopy.obj", oc);

    // 3. Towering Ancient Redwood (Separate Fluted Trunk + High Boughs)
    let (mut rt, mut rtc, mut rc, mut rcc) = (String::new(), 1, String::new(), 1);
    push_cone(&mut rt, &mut rtc, 0.0, -0.4, 0.0, 0.76, 0.38, 3.3, 14, 0.22, 0.0, 31);
    push_cone(&mut rt, &mut rtc, 0.0, 2.8, 0.0, 0.38, 0.14, 6.4, 12, 0.10, 0.0, 32);
    for t in 0..8 {
        let tf = t as f32;
        push_cone(&mut rc, &mut rcc, (hash2(t, 2)-0.5)*0.20, 4.1 + tf*0.68, (hash2(t, 4)-0.5)*0.20, (2.10 - tf*0.20).max(0.40), 0.08, 1.25, 16, 0.48, 0.30, t+40);
    }
    let _ = fs::write("assets/redwood_trunk.obj", rt);
    let _ = fs::write("assets/redwood_canopy.obj", rc);

    // 4. Wide-Deck Climbable Watchtower
    let (mut wt, mut wvi) = (String::new(), 1);
    for sx in [-2.2, 1.95] { for sz in [-2.2, 1.95] { push_box(&mut wt, &mut wvi, sx, -0.5, sz, sx + 0.25, 6.5, sz + 0.25); } }
    for y in [2.1, 4.3] {
        push_box(&mut wt, &mut wvi, -2.2, y, -2.2, 2.2, y+0.18, -2.02);
        push_box(&mut wt, &mut wvi, -2.2, y, 2.02, 2.2, y+0.18, 2.2);
        push_box(&mut wt, &mut wvi, -2.2, y, -2.2, -2.02, y+0.18, 2.2);
        push_box(&mut wt, &mut wvi, 2.02, y, -2.2, 2.2, y+0.18, 2.2);
    }
    for step in 0..16 {
        let t = step as f32 / 16.0;
        push_box(&mut wt, &mut wvi, -3.8 + t * 5.2, t * 6.5, 2.85, -3.42 + t * 5.2, t * 6.5 + 0.25, 4.10);
    }
    push_box(&mut wt, &mut wvi, -2.85, 6.5, -2.85, 2.85, 6.75, 2.85);
    push_box(&mut wt, &mut wvi, 1.4, 6.5, 2.85, 2.85, 6.75, 4.15);
    push_box(&mut wt, &mut wvi, 2.60, -0.5, 3.90, 2.83, 6.5, 4.13);
    for rx in [-2.80, 2.65] { for rz in [-2.80, 2.65] { push_box(&mut wt, &mut wvi, rx, 6.75, rz, rx + 0.15, 7.75, rz + 0.15); } }
    push_box(&mut wt, &mut wvi, -2.85, 7.6, -2.85, 2.85, 7.75, -2.70);
    push_box(&mut wt, &mut wvi, -2.85, 7.6, -2.85, -2.70, 7.75, 2.85);
    push_box(&mut wt, &mut wvi, 2.70, 7.6, -2.85, 2.85, 7.75, 2.85);
    for cx in [-1.5, 1.32] { for cz in [-1.5, 1.32] { push_box(&mut wt, &mut wvi, cx, 6.75, cz, cx + 0.18, 9.1, cz + 0.18); } }
    push_box(&mut wt, &mut wvi, -1.5, 6.75, -1.5, 1.5, 7.75, -1.36);
    push_box(&mut wt, &mut wvi, -1.5, 6.75, -1.5, -1.36, 7.75, 1.5);
    push_box(&mut wt, &mut wvi, 1.36, 6.75, -1.5, 1.5, 7.75, 1.5);
    push_box(&mut wt, &mut wvi, -1.5, 6.75, 1.36, -0.65, 9.05, 1.5);
    push_box(&mut wt, &mut wvi, 0.65, 6.75, 1.36, 1.5, 9.05, 1.5);
    push_box(&mut wt, &mut wvi, -0.7, 6.75, -1.3, 0.7, 7.65, -0.85);
    push_box(&mut wt, &mut wvi, -1.95, 9.05, -1.95, 1.95, 9.3, 1.95);
    push_box(&mut wt, &mut wvi, -1.35, 9.3, -1.35, 1.35, 9.6, 1.35);
    let _ = fs::write("assets/watchtower.obj", wt);

    // 5. Detailed 20L Stamped Military Jerrycan
    let (mut jc, mut jvi) = (String::new(), 1);
    push_box(&mut jc, &mut jvi, -0.15, 0.0, -0.23, 0.15, 0.45, 0.23);
    push_box(&mut jc, &mut jvi, -0.17, 0.08, -0.16, 0.17, 0.37, 0.16);
    push_box(&mut jc, &mut jvi, -0.10, 0.45, -0.16, -0.06, 0.54, 0.16);
    push_box(&mut jc, &mut jvi, -0.02, 0.45, -0.16, 0.02, 0.54, 0.16);
    push_box(&mut jc, &mut jvi, 0.06, 0.45, -0.16, 0.10, 0.54, 0.16);
    push_box(&mut jc, &mut jvi, -0.10, 0.52, -0.16, 0.10, 0.55, 0.16);
    push_box(&mut jc, &mut jvi, -0.05, 0.45, 0.12, 0.05, 0.56, 0.22);
    let _ = fs::write("assets/jerrycan.obj", jc);

    // 6. Detailed Iron-Banded Barrel-Top Supply Chest
    let (mut ch, mut cvi) = (String::new(), 1);
    push_box(&mut ch, &mut cvi, -0.48, 0.0, -0.30, 0.48, 0.36, 0.30);
    push_box(&mut ch, &mut cvi, -0.50, 0.34, -0.32, 0.50, 0.40, 0.32);
    push_box(&mut ch, &mut cvi, -0.46, 0.40, -0.27, 0.46, 0.48, 0.27);
    push_box(&mut ch, &mut cvi, -0.40, 0.48, -0.20, 0.40, 0.55, 0.20);
    push_box(&mut ch, &mut cvi, -0.30, 0.55, -0.12, 0.30, 0.60, 0.12);
    for bx in [-0.44, -0.18, 0.18, 0.44] { push_box(&mut ch, &mut cvi, bx - 0.025, 0.0, -0.32, bx + 0.025, 0.57, 0.32); }
    for sx in [-0.52, 0.48] { push_box(&mut ch, &mut cvi, sx, 0.18, -0.10, sx + 0.04, 0.24, 0.10); }
    push_box(&mut ch, &mut cvi, -0.07, 0.22, 0.29, 0.07, 0.42, 0.34);
    let _ = fs::write("assets/chest.obj", ch);

    // 7. Industrial V-Twin Field Generator with Roll Cage & Fuel Tank
    let (mut gn, mut gvi) = (String::new(), 1);
    push_box(&mut gn, &mut gvi, -0.55, 0.0, -0.38, 0.55, 0.08, 0.38);
    for cx in [-0.54, 0.50] { for cz in [-0.37, 0.33] { push_box(&mut gn, &mut gvi, cx, 0.08, cz, cx + 0.04, 0.78, cz + 0.04); } }
    push_box(&mut gn, &mut gvi, -0.55, 0.74, -0.38, 0.55, 0.78, -0.34);
    push_box(&mut gn, &mut gvi, -0.55, 0.74, 0.34, 0.55, 0.78, 0.38);
    push_box(&mut gn, &mut gvi, -0.55, 0.74, -0.38, -0.51, 0.78, 0.38);
    push_box(&mut gn, &mut gvi, 0.51, 0.74, -0.38, 0.55, 0.78, 0.38);
    push_box(&mut gn, &mut gvi, -0.38, 0.12, -0.26, 0.15, 0.54, 0.26);
    push_box(&mut gn, &mut gvi, 0.15, 0.16, -0.22, 0.46, 0.50, 0.22);
    push_box(&mut gn, &mut gvi, -0.42, 0.56, -0.26, 0.36, 0.74, 0.26);
    push_box(&mut gn, &mut gvi, -0.48, 0.38, 0.24, -0.18, 0.66, 0.35);
    push_box(&mut gn, &mut gvi, 0.22, 0.44, -0.34, 0.44, 0.62, -0.22);
    let _ = fs::write("assets/generator.obj", gn);
}
