#![windows_subsystem = "windows"]

mod audio; mod entities; mod intro; mod graphics; mod model; mod monster; mod net; mod voice; mod world;
use audio::*; use entities::*; use graphics::*; use macroquad::prelude::*; use model::*; use monster::*; use net::*; use voice::*; use world::*;

fn scale_rgba(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize, dst: &mut [u8]) {
    for y in 0..dh {
        for x in 0..dw {
            let sx = (x * sw) / dw;
            let sy = (y * sh) / dh;
            let si = (sy * sw + sx) * 4;
            let di = (y * dw + x) * 4;
            dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
}
fn load_app_icon() -> Option<macroquad::miniquad::conf::Icon> {
    let img = Image::from_file_with_format(include_bytes!("../assets/icon.png"), Some(ImageFormat::Png)).ok()?;
    let (mut s16, mut s32, mut s64) = ([0u8; 1024], [0u8; 4096], [0u8; 16384]);
    scale_rgba(&img.bytes, img.width as usize, img.height as usize, 16, 16, &mut s16);
    scale_rgba(&img.bytes, img.width as usize, img.height as usize, 32, 32, &mut s32);
    scale_rgba(&img.bytes, img.width as usize, img.height as usize, 64, 64, &mut s64);
    Some(macroquad::miniquad::conf::Icon { small: s16, medium: s32, big: s64 })
}
fn window_conf() -> Conf {
    Conf { window_title: "THE NIGHT SHIFT AT TOWER 4".into(), window_width: 1280, window_height: 720, window_resizable: true, icon: load_app_icon(), ..Default::default() }
}

enum Screen { Intro, Outro, UsernameSetup, Menu, HostSetup, Join, Settings, Lore, Playing }

fn ui_btn(label: &str, x: f32, y: f32, w: f32, h: f32) -> bool {
    let (mx, my) = mouse_position();
    let hov = mx >= x && mx <= x + w && my >= y && my <= y + h;
    draw_rectangle(x, y, w, h, if hov { Color::from_rgba(145, 22, 22, 235) } else { Color::from_rgba(14, 17, 22, 215) });
    draw_rectangle_lines(x, y, w, h, 2.0, if hov { RED } else { Color::from_rgba(70, 75, 85, 220) });
    draw_text(label, x + 20.0, y + h * 0.65, 22.0, if hov { WHITE } else { LIGHTGRAY });
    hov && is_mouse_button_pressed(MouseButton::Left)
}

fn profile_path() -> std::path::PathBuf { let base = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".into()); let dir = std::path::PathBuf::from(base).join(".tower4"); let _ = std::fs::create_dir_all(&dir); dir.join("player_profile.txt") }
#[macroquad::main(window_conf)]
async fn main() { std::panic::set_hook(Box::new(|info| { let _ = std::fs::write("crash.log", format!("{}\n{:?}", info, std::backtrace::Backtrace::force_capture())); }));
    
    let smooth_n = |x: f32, y: f32, s: u32| -> f32 {
        let (xi, yi) = (x.floor() as u32, y.floor() as u32);
        let (xf, yf) = (x - x.floor(), y - y.floor());
        let (u, v) = (xf * xf * (3.0 - 2.0 * xf), yf * yf * (3.0 - 2.0 * yf));
        let a = hash2(xi + s, yi + s); let b = hash2(xi + 1 + s, yi + s);
        let c = hash2(xi + s, yi + 1 + s); let d = hash2(xi + 1 + s, yi + 1 + s);
        (a * (1.0 - u) + b * u) * (1.0 - v) + (c * (1.0 - u) + d * u) * v
    };
    let fbm = |x: u16, y: u16, s: u32| -> f32 {
        let (fx, fy) = (x as f32, y as f32);
        smooth_n(fx * 0.04, fy * 0.04, s) * 0.52 + smooth_n(fx * 0.11, fy * 0.11, s + 19) * 0.31 + smooth_n(fx * 0.28, fy * 0.28, s + 47) * 0.17
    };
    let dirt = make_tex(128, 128, |x, y| {
        let (fx, fy) = (x as f32, y as f32);
        let n = smooth_n(fx * 0.025, fy * 0.025, 11) * 0.55 + smooth_n(fx * 0.08, fy * 0.08, 31) * 0.30 + smooth_n(fx * 0.22, fy * 0.22, 59) * 0.15;
        let soil = smooth_n(fx * 0.04, fy * 0.04, 91);
        let blade = hash2(x as u32 * 13, y as u32 * 29);
        let needle = ((fx * 0.35 + fy * 0.28 + n * 8.0).sin().abs()).powf(8.0) * soil;
        let r = (36.0 + n * 32.0 + soil * 28.0 + needle * 38.0 - blade * 10.0).clamp(18.0, 120.0) as u8;
        let g = (48.0 + n * 44.0 - soil * 14.0 + blade * 24.0 + needle * 14.0).clamp(24.0, 135.0) as u8;
        let b = (22.0 + n * 18.0 + soil * 8.0 - blade * 6.0).clamp(12.0, 85.0) as u8;
        [r, g, b, 255]
    });
    let foliage = make_tex(128, 128, |x, y| {
        let n = fbm(x, y, 61);
        let needle = hash2(x as u32 * 11, y as u32 * 17) * 0.18;
        [(24.0 + n * 26.0 + needle * 18.0) as u8, (54.0 + n * 48.0 + needle * 32.0) as u8, (22.0 + n * 22.0) as u8, 255]
    });
    let wood = make_tex(128, 128, |x, y| {
        let n = fbm(x, y, 23);
        let ring = ((y as f32 * 0.18 + n * 6.0).sin() * 0.5 + 0.5).powf(1.3);
        let plank = if y % 32 == 0 { 0.62 } else { 1.0 };
        [((76.0 + ring * 48.0) * plank) as u8, ((52.0 + ring * 34.0) * plank) as u8, ((34.0 + ring * 22.0) * plank) as u8, 255]
    });
    let bark = make_tex(128, 128, |x, y| {
        let n = fbm(x, y, 37);
        let ridge = ((x as f32 * 0.25 + n * 5.0).sin() * 0.5 + 0.5).powf(1.5);
        [(44.0 + ridge * 42.0) as u8, (34.0 + ridge * 30.0) as u8, (26.0 + ridge * 22.0) as u8, 255]
    });
    let rock = make_tex(128, 128, |x, y| {
        let n = fbm(x, y, 53);
        [(62.0 + n * 48.0) as u8, (66.0 + n * 50.0) as u8, (72.0 + n * 52.0) as u8, 255]
    });
    let metal = make_tex(128, 128, |x, y| {
        let n = fbm(x, y, 71);
        [(64.0 + n * 38.0) as u8, (70.0 + n * 40.0) as u8, (76.0 + n * 42.0) as u8, 255]
    });

    ensure_all_assets();
    
    let trees = [
        (load_obj_mesh("assets/pine_trunk.obj", Some(&bark), Color::from_rgba(110, 82, 62, 255)), load_obj_mesh("assets/pine_canopy.obj", Some(&foliage), Color::from_rgba(68, 112, 64, 255))),
        (load_obj_mesh("assets/oak_trunk.obj", Some(&bark), Color::from_rgba(95, 74, 58, 255)), load_obj_mesh("assets/oak_canopy.obj", Some(&foliage), Color::from_rgba(82, 122, 58, 255))),
        (load_obj_mesh("assets/redwood_trunk.obj", Some(&bark), Color::from_rgba(122, 72, 54, 255)), load_obj_mesh("assets/redwood_canopy.obj", Some(&foliage), Color::from_rgba(58, 98, 56, 255))),
    ];
    let tower_mesh = load_custom_or_default("assets/custom_watchtower.obj", "assets/watchtower.obj", Some(&wood), Color::from_rgba(120, 95, 72, 255));
    let cabin_meshes = load_custom_chunks("assets/cabin.obj", "assets/cabin.obj", Some(&wood), Color::from_rgba(145, 112, 82, 255));
    let gen_meshes = load_custom_chunks("assets/custom_generator.obj", "assets/generator.obj", Some(&metal), WHITE);
    let can_mesh = load_custom_or_default("assets/custom_jerrycan.obj", "assets/jerrycan.obj", Some(&metal), Color::from_rgba(185, 42, 36, 255));
    let chest_mesh = load_custom_or_default("assets/custom_chest.obj", "assets/chest.obj", Some(&wood), Color::from_rgba(130, 88, 48, 255));
    let moon_mesh = graphics::make_sphere_mesh(5.2, 14, 20, Some(&rock), Color::from_rgba(205, 222, 250, 255));

    
    let mat = create_horror_material();
    let mut outposts = random_outposts(1);
    let mut terrain_chunks = build_terrain_chunks(&dirt, &outposts);
    let make_gens = |ops: &[(i32, i32); 9]| -> Vec<(Vec3, f32, bool, bool)> {
        [0, 1, 3, 5, 7, 8].iter().map(|&idx| {
            let (ox, oz) = ops[idx];
            (vec3(ox as f32 + 0.5, terrain_y(ox as f32 + 0.5, oz as f32 + 0.5) + 0.28, oz as f32 + 0.5), 0.0, false, false)
        }).collect()
    };
    let make_cans = |ops: &[(i32, i32); 9]| -> Vec<(Vec3, bool)> {
        let mut v = Vec::new();
        for &(ox, oz) in ops {
            let (cx, cz) = (ox as f32 - 1.15, oz as f32 - 1.15);
            v.push((vec3(cx, walkable_y(cx, cz, ops), cz), true));
        }
        v.push((vec3(190.0, terrain_y(190.0, 190.0), 190.0), true));
        v.push((vec3(315.0, terrain_y(315.0, 58.0) + 6.75, 58.0), true));
        v
    };
    let make_chests = |ops: &[(i32, i32); 9]| -> Vec<(Vec3, f32, bool, u8)> {
        let hpi = std::f32::consts::FRAC_PI_2;
        let mut v = vec![
            (vec3(188.5, terrain_y(188.5, 192.0), 192.0), 0.0, false, 1),
            (vec3(62.0, terrain_y(62.0, 318.0) + 6.75, 318.5), 0.0, false, 2),
            (vec3(322.0, terrain_y(322.0, 322.0) + 6.75, 322.5), 0.0, false, 3),
        ];
        for (i, &(ox, oz)) in ops.iter().enumerate() {
            let gy = terrain_y(ox as f32 + 0.5, oz as f32 + 0.5) + 0.28;
            let loot = ((i % 3) + 1) as u8; // 100% guaranteed evading gear: 1=UV Camera, 2=2x Flares, 3=Adrenaline
            let (cx, cz, cyaw) = match i % 3 {
                0 => (ox as f32 + 2.2, oz as f32 + 1.5, -hpi), // right of door -> gold lock faces left
                1 => (ox as f32 - 2.0, oz as f32 + 1.5, hpi),  // left of door -> gold lock faces right
                _ => (ox as f32 + 0.5, oz as f32 - 2.2, 0.0),  // back wall -> gold lock faces door
            };
            v.push((vec3(cx, gy, cz), cyaw, false, loot));
        }
        v
    };
    let mut gens = make_gens(&outposts);
    let mut cans = make_cans(&outposts);
    let mut chests = make_chests(&outposts);
    let map_pickup = (vec3(outposts[0].0 as f32 - 1.6, terrain_y(outposts[0].0 as f32 + 0.5, outposts[0].1 as f32 + 0.5) + 1.15, outposts[0].1 as f32 - 0.7), true);
    let mut bats: Vec<(Vec3, bool)> = outposts.iter().map(|&(ox, oz)| {
        (vec3(ox as f32 - 2.1, terrain_y(ox as f32 + 0.5, oz as f32 + 0.5) + 1.04, oz as f32 - 1.2), true)
    }).collect();
    for &(tx, tz) in &[(42.3f32, 41.2f32), (315.3, 57.2), (62.3, 317.2), (322.3, 321.2)] {
        bats.push((vec3(tx, terrain_y(tx, tz) + 7.68, tz), true));
    }
    for &(fx, fz) in &[(95.0f32, 110.0f32), (145.0, 235.0), (225.0, 95.0), (265.0, 210.0), (120.0, 295.0), (295.0, 130.0), (190.0, 188.0), (75.0, 165.0)] {
        bats.push((vec3(fx, terrain_y(fx, fz) + 0.08, fz), true));
    }
    let towers = [(42.0f32, 42.0f32), (315.0, 58.0), (62.0, 318.0), (322.0, 322.0)];
    let tower_pos = vec3(42.0, terrain_y(42.0, 42.0), 42.0);
    let mut camps = [vec3(50.0, terrain_y(50.0, 50.0), 50.0), vec3(190.0, terrain_y(190.0, 186.0), 186.0)];
    let logo_tex = Texture2D::from_file_with_format(include_bytes!("../assets/icon.png"), Some(ImageFormat::Png));
    let mut mimic = Mimic::new(&bark);
    let mut stalker = GazeStalker::new();
    let mut breakers = [(vec3(54.0, terrain_y(54.0, 224.0), 224.0), false), (vec3(326.0, terrain_y(326.0, 152.0), 152.0), false)];
    let (mut breaker_timer, mut breakers_locked) = (0.0f32, false);
    let mut black_boxes = vec![
        (vec3(315.0, terrain_y(315.0, 58.0) + 6.75, 58.6), false),
        (vec3(62.0, terrain_y(62.0, 318.0) + 6.75, 318.6), false),
        (vec3(190.5, terrain_y(190.5, 190.5), 190.5), false),
    ];
    let (mut boxes_collected, mut eclipse_cd, mut eclipse_t, mut equip_t, mut hand_grip) = (0i32, 48.0f32, 0.0f32, 0.0f32, 1.0f32);
    let (mut radio_msg_t, mut radio_msg_idx) = (0.0f32, 0usize);
    let mut audio = AudioEngine::new(tower_pos).await;
    let voice = VoiceChat::new();
    let mut net = NetManager::new();
    let (_join_ip, mut lobby_code) = (String::new(), String::from("TOWER4"));
    let saved_name = std::fs::read_to_string(profile_path()).unwrap_or_default().trim().to_string();
    let mut player_name = saved_name.clone();
    let (mut screen, mut active_shift, mut won, mut intro_t, mut intro_vo, mut intro_vo2, mut intro_vo3, mut outro_t, mut outro_vo) = (Screen::Intro, false, false, 0.0f32, false, false, false, 0.0f32, false);
    let (mut vol, mut bright, mut sens, mut quality, mut is_fs) = (0.8f32, 1.0f32, 1.0f32, 2usize, false);
    let (mut pos, mut vel_y, mut yaw, mut pitch) = (vec3(44.5, walkable_y(44.5, 45.5, &outposts), 45.5), 0.0f32, -2.35f32, 0.05f32);
    let camcorder = true; let (mut uv_charges, mut uv_cd, mut uv_flash_t, mut flare_count, mut adren_count, mut adren_t, mut hotbar_slot, mut show_bag, mut drag_t, mut drag_dir) = (0i32, 0.0f32, 0.0f32, 0i32, 0i32, 0.0f32, 0usize, false, 0.0f32, Vec2::X); let mut active_flares: Vec<(Vec3, f32)> = Vec::new(); let mut dropped_items: Vec<(Vec3, u8)> = Vec::new(); let (mut eye_h, mut step_dist, mut bob, mut stam, mut bat, mut exhausted) = (1.65f32, 0.0f32, 0.0f32, 100.0f32, 100.0f32, false);
    let (mut on, mut noise, mut wake_t, mut scare_t, mut lost_notice_t, mut radio_prog) = (true, 0.0f32, 3.5f32, 0.0f32, 0.0f32, 0.0f32);
    let mut spare_bats = 0i32; let (mut has_journal, mut show_journal, mut has_map, mut show_map, mut has_radar, mut fuel_cans) = (false, false, true, false, false, 0i32);
    let (mut batteries, mut map_pickup) = (bats, map_pickup.0);
    loop {
        let dt = get_frame_time().min(0.05); if is_key_pressed(KeyCode::F11) || ((is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt)) && is_key_pressed(KeyCode::Enter)) { is_fs = !is_fs; set_fullscreen(is_fs); }
        let in_game = matches!(screen, Screen::Playing) && !show_map && !show_journal && !show_bag; audio.update(dt, vol, if matches!(screen, Screen::Playing) { 1.0 } else if matches!(screen, Screen::Intro) { if intro_t < 2.8 { -1.0 } else { 0.52 } } else { 0.0 }, pos, (vec2(pos.x, pos.z) - vec2(mimic.pos.x, mimic.pos.z)).length());
        set_cursor_grab(in_game); show_mouse(!in_game);
        if matches!(screen, Screen::Playing) {
            if is_key_pressed(KeyCode::Escape) { if show_map || show_journal || show_bag { show_map = false; show_journal = false; show_bag = false; } else { screen = Screen::Menu; } } if is_key_pressed(KeyCode::B) || is_key_pressed(KeyCode::Tab) { show_bag = !show_bag; show_map = false; show_journal = false; audio.play_click(vol); }
            if is_key_pressed(KeyCode::J) && has_journal { show_journal = !show_journal; show_map = false; audio.play_click(vol); }
            if is_key_pressed(KeyCode::M) && has_map { show_map = !show_map; show_journal = false; audio.play_click(vol); }
            if is_key_pressed(KeyCode::F) && bat > 1.0 { on = !on; audio.play_click(vol); }
        }
        let ptt = in_game && is_key_down(KeyCode::V); *voice.ptt_active.lock().unwrap() = ptt;
        let mic_n = *voice.mic_level.lock().unwrap();
        if in_game {
            wake_t += dt;
            if wake_t > 1.2 { let d = mouse_delta_position(); yaw -= d.x * 2.2 * sens; pitch = (pitch + d.y * 1.6 * sens).clamp(-1.35, 1.35); }
            let crouch = is_key_down(KeyCode::C) || is_key_down(KeyCode::LeftControl);
            let moving = is_key_down(KeyCode::W) || is_key_down(KeyCode::S) || is_key_down(KeyCode::A) || is_key_down(KeyCode::D);
            let sprint = is_key_down(KeyCode::LeftShift) && !crouch && moving && !exhausted && stam > 0.0;
            if sprint {
                stam = (stam - 26.0 * dt).max(0.0);
                if stam <= 0.0 { exhausted = true; }
            } else {
                stam = (stam + 16.0 * dt).min(100.0);
                if stam >= 25.0 { exhausted = false; }
            }
            eye_h += ((if crouch { 1.05 } else { 1.65 }) - eye_h) * (10.0 * dt).min(1.0);
            if adren_t > 0.0 { adren_t = (adren_t - dt).max(0.0); stam = 100.0; exhausted = false; } let spd = if wake_t < 1.5 { 0.0 } else if crouch { 2.1 } else if sprint { if adren_t > 0.0 { 8.4 } else { 6.8 } } else { 4.1 };
            let fwd = vec2(yaw.cos(), yaw.sin()); let right = vec2(-fwd.y, fwd.x); let mut mv = Vec2::ZERO;
            if is_key_down(KeyCode::W) { mv += fwd; } if is_key_down(KeyCode::S) { mv -= fwd; }
            if is_key_down(KeyCode::D) { mv += right; } if is_key_down(KeyCode::A) { mv -= right; }
            if mv.length_squared() > 0.0 {
                mv = mv.normalize() * spd * dt;
                let (nx, nz) = (pos.x + mv.x, pos.z + mv.y);
                if can_walk_to(nx + mv.x.signum() * 0.25, pos.y, pos.z, &outposts, &gens, &chests) && (walkable_y(nx, pos.z, &outposts) - pos.y) <= 0.75 { pos.x = nx; }
                if can_walk_to(pos.x, pos.y, nz + mv.y.signum() * 0.25, &outposts, &gens, &chests) && (walkable_y(pos.x, nz, &outposts) - pos.y) <= 0.75 { pos.z = nz; }
                step_dist += mv.length(); bob += dt * spd * 3.8;
                if step_dist > (if sprint { 2.6 } else { 1.85 }) { step_dist = 0.0; audio.play_step(vol, crouch, sprint); }
            }
            let target_n = if mv.length_squared() == 0.0 { 0.04 } else if crouch { 0.14 } else if sprint { 0.96 } else { 0.45f32 }.max(mic_n * 1.15);
            noise += (target_n - noise) * (6.0 * dt).min(1.0);
            if on { bat = (bat - 0.22 * dt).max(0.0); if 0.0 >= bat { on = false; } }
        }
        if in_game {
            for (k, idx) in [(KeyCode::Key1, 0), (KeyCode::Key2, 1), (KeyCode::Key3, 2), (KeyCode::Key4, 3), (KeyCode::Key5, 4)] { if is_key_pressed(k) && hotbar_slot != idx { hotbar_slot = idx; equip_t = 0.38; audio.play_click(vol * 1.15); } }
            let (_, mw) = mouse_wheel(); if mw > 0.1 { hotbar_slot = (hotbar_slot + 4) % 5; equip_t = 0.38; audio.play_click(vol * 1.15); } else if -0.1 > mw { hotbar_slot = (hotbar_slot + 1) % 5; equip_t = 0.38; audio.play_click(vol * 1.15); }
            if uv_cd > 0.0 { uv_cd = (uv_cd - dt).max(0.0); } if equip_t > 0.0 { equip_t = (equip_t - dt).max(0.0); } let holding_any = match hotbar_slot { 0 => true, 1 => uv_charges > 0, 2 => flare_count > 0, 3 => adren_count > 0, 4 => has_map || has_radar, _ => false }; let target_grip = if holding_any { 1.0f32 } else { 0.0f32 }; hand_grip += (target_grip - hand_grip) * (10.0 * dt).min(1.0);
            if radio_msg_t > 0.0 { radio_msg_t = (radio_msg_t - dt).max(0.0); }
            if breaker_timer > 0.0 && !breakers_locked {
                breaker_timer = (breaker_timer - dt).max(0.0);
                if 0.0 >= breaker_timer { breakers[0].1 = false; breakers[1].1 = false; }
            }
            if eclipse_t > 0.0 { eclipse_t = (eclipse_t - dt).max(0.0); }
            else {
                eclipse_cd -= dt;
                if 0.0 >= eclipse_cd { eclipse_t = 22.0; eclipse_cd = 78.0; mimic.phase = AiPhase::Chasing; mimic.disguised = false; audio.play_roar(vol * 0.8); }
            }
            if hotbar_slot == 0 && (is_mouse_button_pressed(MouseButton::Left) || is_key_pressed(KeyCode::T)) {
                radio_msg_idx = (radio_msg_idx + 1) % 4; radio_msg_t = 4.5; audio.play_click(vol);
            }
            if uv_flash_t > 0.0 { uv_flash_t = (uv_flash_t - dt).max(0.0); }
            for fl in &mut active_flares { fl.1 -= dt; } active_flares.retain(|fl| fl.1 > 0.0);
            if is_key_pressed(KeyCode::R) && spare_bats > 0 && 99.0 > bat {
                spare_bats -= 1; bat = (bat + 50.0).min(100.0); audio.play_click(vol);
            }
            if is_key_pressed(KeyCode::G) {
                let dp = vec3(pos.x + yaw.cos() * 0.9, walkable_y(pos.x + yaw.cos() * 0.9, pos.z + yaw.sin() * 0.9, &outposts) + 0.12, pos.z + yaw.sin() * 0.9);
                if fuel_cans > 0 { fuel_cans -= 1; dropped_items.push((dp, 0)); equip_t = 0.28; audio.play_click(vol); }
                else if hotbar_slot == 1 && uv_charges > 0 { uv_charges -= 1; dropped_items.push((dp, 1)); equip_t = 0.28; audio.play_click(vol); }
                else if hotbar_slot == 2 && flare_count > 0 { flare_count -= 1; dropped_items.push((dp, 2)); equip_t = 0.28; audio.play_click(vol); }
                else if hotbar_slot == 3 && adren_count > 0 { adren_count -= 1; dropped_items.push((dp, 3)); equip_t = 0.28; audio.play_click(vol); }
                else if spare_bats > 0 { spare_bats -= 1; dropped_items.push((dp, 5)); equip_t = 0.28; audio.play_click(vol); } else if hotbar_slot == 4 && has_radar { has_radar = false; dropped_items.push((dp, 4)); equip_t = 0.28; audio.play_click(vol); }
            }
            let q_press = is_key_pressed(KeyCode::Q);
            let lmb_use = is_mouse_button_pressed(MouseButton::Left) && hotbar_slot > 0;
            if (q_press || (lmb_use && hotbar_slot == 1)) && uv_charges > 0 && uv_cd <= 0.0 {
                uv_charges -= 1; uv_cd = 14.0; uv_flash_t = 0.40; audio.play_click(vol);
                let l_dir = vec3(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos()).normalize();
                let to_m = vec3(mimic.pos.x, mimic.pos.y + 1.2, mimic.pos.z) - pos;
                if to_m.length() < 22.0 && l_dir.dot(to_m.normalize_or_zero()) > 0.60 {
                    mimic.frozen = true; mimic.stun_exposure = 0.0; mimic.disguised = false; mimic.phase = AiPhase::Searching; mimic.grace_timer = 6.0;
                    let away = (vec2(mimic.pos.x, mimic.pos.z) - vec2(pos.x, pos.z)).normalize_or_zero();
                    mimic.target_xz = vec2((mimic.pos.x + away.x * 45.0).clamp(24.0, 356.0), (mimic.pos.z + away.y * 45.0).clamp(24.0, 356.0));
                }
            } else if lmb_use && hotbar_slot == 2 && flare_count > 0 {
                flare_count -= 1; active_flares.push((vec3(pos.x, walkable_y(pos.x, pos.z, &outposts) + 0.05, pos.z), 28.0)); audio.play_click(vol); if (vec2(mimic.pos.x, mimic.pos.z) - vec2(pos.x, pos.z)).length() < 16.0 { audio.play_roar(vol * 0.85); }
            } else if lmb_use && hotbar_slot == 3 && adren_count > 0 {
                adren_count -= 1; adren_t = 20.0; stam = 100.0; exhausted = false; audio.play_click(vol);
            }
        }
        let floor_y = walkable_y(pos.x, pos.z, &outposts);
        if pos.y - floor_y > 0.65 { vel_y -= 22.0 * dt; pos.y += vel_y * dt; if pos.y <= floor_y { pos.y = floor_y; vel_y = 0.0; } } else { pos.y = floor_y; vel_y = 0.0; }
        let (cam, look) = if drag_t > 0.0 {
            let t = get_time() as f32;
            let shk = vec3((t * 36.0).sin() * 0.12, (t * 44.0).cos() * 0.08, (t * 31.0).sin() * 0.12);
            (vec3(pos.x, pos.y + 0.48, pos.z) + shk, vec3(drag_dir.x, -0.15, drag_dir.y).normalize())
        } else if matches!(screen, Screen::Outro) {
            let p = (outro_t / 9.2).clamp(0.0, 1.0);
            let c_pos = vec3(42.0 + (p * 1.4).sin() * 12.0, terrain_y(42.0, 42.0) + 9.5 + p * 22.0, 42.0 + (p * 1.4).cos() * 14.0);
            let c_tgt = vec3(46.0, terrain_y(46.0, 46.0) + 2.5 - p * 1.5, 46.0);
            (c_pos, (c_tgt - c_pos).normalize())
        } else if matches!(screen, Screen::Intro) {
            let p = ((intro_t - 3.2).max(0.0) / (audio.radio_dur + 1.8)).clamp(0.0, 1.0);
            let (mx, my) = mouse_position(); let pan_x = (mx / screen_width() - 0.5) * 6.0; let pan_y = (0.5 - my / screen_height()) * 3.5; let c_pos = vec3(52.0 - p * 6.0 + pan_x, terrain_y(42.0, 42.0) + 2.2 + p * 6.5 + pan_y, 56.0 - p * 7.0);
            let c_tgt = vec3(42.0, terrain_y(42.0, 42.0) + 6.2 + p * 1.8, 42.0);
            (c_pos, (c_tgt - c_pos).normalize())
        } else if scare_t > 0.0 {
            let t = get_time() as f32;
            let shk = vec3((t * 48.0).sin() * 0.09, (t * 57.0).cos() * 0.09, (t * 43.0).sin() * 0.09);
            let m_fwd = vec3(mimic.yaw.cos(), 0.0, mimic.yaw.sin());
            let head = mimic.pos + vec3(0.0, 1.35, 0.0);
            (head + m_fwd * 0.92 + shk, (-m_fwd + vec3(0.0, -0.05, 0.0)).normalize())
        } else if matches!(screen, Screen::Playing) {
            let l = vec3(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos()).normalize();
            (vec3(pos.x, pos.y + eye_h + bob.sin().abs() * 0.045, pos.z), l)
        } else { let a = (get_time() as f32) * 0.14; (vec3(42.0 + a.cos() * 14.0, terrain_y(42.0, 42.0) + 9.0, 42.0 + a.sin() * 14.0), vec3(-a.cos(), -0.35, -a.sin()).normalize()) };
        if in_game {
            let (p_n, p_pos, ops_ch) = net.sync(dt, hotbar_slot == 0, pos, yaw, pitch, on, eye_h < 1.3, &voice, vol, &mut outposts, &mut gens, &mut mimic.pos, &mut mimic.yaw, &mut mimic.phase);
            if ops_ch { terrain_chunks = build_terrain_chunks(&dirt, &outposts); camps = [vec3(outposts[0].0 as f32 + 4.8, terrain_y(outposts[0].0 as f32 + 4.8, outposts[0].1 as f32 + 3.8), outposts[0].1 as f32 + 3.8), vec3(outposts[1].0 as f32 + 4.8, terrain_y(outposts[1].0 as f32 + 4.8, outposts[1].1 as f32 + 3.8), outposts[1].1 as f32 + 3.8)]; gens = make_gens(&outposts); cans = make_cans(&outposts); chests = make_chests(&outposts); has_journal = false; has_map = true; has_radar = false; fuel_cans = 0; spare_bats = 0; uv_charges = 0; flare_count = 0; adren_count = 0; active_flares.clear(); }
            let is_cli = !net.peers.is_empty() && matches!(net.role, NetRole::Client);
            let (caught, roared, lost) = if is_cli { ((vec2(pos.x, pos.z) - vec2(mimic.pos.x, mimic.pos.z)).length() < 1.25, false, false) }
                else if p_n > noise && p_pos != Vec3::ZERO { mimic.update(dt, p_pos, look, false, p_n, gens.iter().filter(|g| g.3).count(), has_radar, &active_flares, &outposts) }
                else { mimic.update(dt, cam, look, on, noise, gens.iter().filter(|g| g.3).count(), has_radar, &active_flares, &outposts) };
            if roared { audio.play_roar(vol); } if mimic.distant_scream { let d_sc = (mimic.pos - cam).length().max(15.0); let sc_v = (1.4 / (1.0 + d_sc * 0.11)).clamp(0.10, 0.35) * vol; audio.play_woman_scream(sc_v); }
            if mimic.distant_echo {
                let _d_ec = (mimic.pos - cam).length().max(8.0);
                let _ = voice.trigger_distant_voice_echo(0.28 * vol);
            }
            if lost { lost_notice_t = 4.0; } if 1.3 > eye_h && !on && 3 > mimic.stealth_evades && 30.0 > (mimic.pos - cam).length() { draw_text(&format!("STEALTH HIDDEN ({}/3 EVASIONS LEFT BEFORE ADAPTATION)", 3 - mimic.stealth_evades), screen_width() * 0.5 - 265.0, screen_height() - 155.0, 19.0, SKYBLUE); } else if 1.3 > eye_h && !on && mimic.stealth_evades >= 3 && 30.0 > (mimic.pos - cam).length() { draw_text("MIMIC ADAPTED TO CROUCH-STEALTH! USE UV CAMERA, FLARES OR ADRENALINE!", screen_width() * 0.5 - 310.0, screen_height() - 155.0, 19.0, RED); }
            let s_caught = stalker.update(dt, cam, look, on, eclipse_t > 0.0, &outposts);
            if (caught || s_caught) && scare_t <= 0.0 && drag_t <= 0.0 {
                audio.play_roar(vol); mimic.disguised = false;
                if ((get_time() * 100.0) as u32) % 2 == 0 {
                    scare_t = 2.4; audio.play_scare(vol);
                } else {
                    drag_t = 3.2;
                    let ang = (get_time() as f32) * 2.3;
                    drag_dir = vec2(ang.cos(), ang.sin()).normalize();
                }
            }
            if breaker_timer > 0.0 && !breakers_locked {
                    draw_text(&format!("BREAKER SYNC WINDOW: {:.1}S - THROW OPPOSITE RIDGE SWITCH!", breaker_timer), screen_width() * 0.5 - 270.0, 74.0, 20.0, YELLOW);
                }
                if radio_msg_t > 0.0 {
                    let calls = ["[RADIO - ALL VALLEY]: \"PULLING RIDGE BREAKER NOW - GET READY!\"", "[RADIO - ALL VALLEY]: \"SECOND ENTITY SPOTTED - DO NOT LOOK AT IT!\"", "[RADIO - ALL VALLEY]: \"RECOVERED A BLACK BOX - MOVING!\"", "[RADIO - ALL VALLEY]: \"ECLIPSE SURGE - GET INSIDE SHELTER!\""];
                    let cm = calls[radio_msg_idx];
                    draw_text(cm, screen_width() * 0.5 - (cm.len() as f32 * 4.4), 104.0, 19.0, GREEN);
                }
                if drag_t > 0.0 {
                drag_t -= dt;
                let nx = (pos.x + drag_dir.x * 14.5 * dt).clamp(24.0, 356.0);
                let nz = (pos.z + drag_dir.y * 14.5 * dt).clamp(24.0, 356.0);
                pos = vec3(nx, walkable_y(nx, nz, &outposts), nz);
                mimic.pos = vec3((pos.x + drag_dir.x * 2.2).clamp(24.0, 356.0), pos.y, (pos.z + drag_dir.y * 2.2).clamp(24.0, 356.0));
                if drag_t <= 0.0 {
                    mimic.phase = AiPhase::Fleeing; mimic.grace_timer = 13.0; mimic.flee_t = 5.5; mimic.persuade_patience = 5.5; let away = drag_dir * 48.0;
                    mimic.target_xz = vec2((pos.x + away.x).clamp(24.0, 356.0), (pos.z + away.y).clamp(24.0, 356.0));
                }
            }
            if scare_t > 0.0 && scare_t - dt <= 0.0 {
                wake_t = 0.0; eye_h = 0.25;
                pos = vec3(42.0, walkable_y(42.0, 42.3, &outposts), 42.3); yaw = -std::f32::consts::FRAC_PI_2;
                mimic.pos = vec3(190.0, 0.0, 210.0); mimic.stealth_evades = 0; mimic.grace_timer = 4.0; mimic.phase = AiPhase::Searching; stalker.reset(); boxes_collected = 0; for bb in &mut black_boxes { bb.1 = false; } breakers_locked = false; breaker_timer = 0.0; breakers[0].1 = false; breakers[1].1 = false; noise = 0.0;
            }
        }
        let anom_cycle = (get_time() as f32) % 62.0;
        let anom_pulse = if anom_cycle > 58.0 && anom_cycle < 58.75 { ((anom_cycle - 58.0) * 4.18).sin().max(0.0) } else { 0.0 };
        let flash_start = 3.2 + audio.radio_dur; let intro_flash = if matches!(screen, Screen::Intro) && intro_t > flash_start && intro_t < flash_start + 0.40 { 1.0f32 } else { 0.0 };
        let anom_pulse = anom_pulse.max(intro_flash);
        let ecl_dim = if eclipse_t > 0.0 { 0.22f32 } else { 1.0f32 }; clear_background(Color::from_rgba(((9.0 + anom_pulse * 95.0) * ecl_dim) as u8, (14.0 * ecl_dim) as u8, (24.0 * ecl_dim) as u8, 255));
        set_camera(&Camera3D { position: cam, target: cam + look, up: Vec3::Y, fovy: 68.0f32.to_radians(), z_near: 0.18, z_far: 185.0, ..Default::default() });
        gl_use_material(&mat);
        let mut lights = [Vec3::ZERO; 6];
        let mut sorted_camps = camps.clone();
        sorted_camps.sort_by(|a, b| (*a - cam).length_squared().partial_cmp(&(*b - cam).length_squared()).unwrap_or(std::cmp::Ordering::Equal));
        for i in 0..4 { if let Some(&cp) = sorted_camps.get(i) { lights[i] = vec3(cp.x, terrain_y(cp.x, cp.z) + 0.65, cp.z); } }
        if let Some(&(fp, _)) = active_flares.last() { lights[0] = fp + vec3(0.0, 0.65, 0.0); }
        lights[4] = camps[0] + vec3(0.0, 0.5, 0.0); lights[5] = camps[1] + vec3(0.0, 0.5, 0.0);
        mat.set_uniform("u_cam_pos", cam); mat.set_uniform("u_cam_dir", look); mat.set_uniform("u_on", if on && in_game { 1.0f32 } else { 0.0f32 });
        mat.set_uniform("u_bright", bright * if eclipse_t > 0.0 { 0.34 } else { 1.0 }); mat.set_uniform("u_time", get_time() as f32); mat.set_uniform("u_l0", lights[0]); mat.set_uniform("u_l1", lights[1]); mat.set_uniform("u_l2", lights[2]); mat.set_uniform("u_l3", lights[3]);
        
        for (cp, ch) in &terrain_chunks { if 122.0 > (vec2(cp.x, cp.z) - vec2(cam.x, cam.z)).length() { draw_mesh(ch); } }
        let r_d = match quality { 0 => 13, 1 => 16, _ => 19 };
        for z in ((cam.z as i32 - r_d).max(0))..((cam.z as i32 + r_d).min(WORLD_SIZE)) {
            for x in ((cam.x as i32 - r_d).max(0))..((cam.x as i32 + r_d).min(WORLD_SIZE)) { draw_forest_props(x, z, &outposts, &trees, &bark, &rock, quality >= 1); }
        }
        for &(tx, tz) in &towers { unsafe { get_internal_gl().quad_gl.push_model_matrix(Mat4::from_translation(vec3(tx, terrain_y(tx, tz), tz))); } draw_mesh(&tower_mesh); unsafe { get_internal_gl().quad_gl.pop_model_matrix(); } }
        draw_cabin_and_camp(&outposts, &cabin_meshes, &wood, &bark, &rock, cam);
        
        for (gp, _, _, fixed) in &gens { unsafe { get_internal_gl().quad_gl.push_model_matrix(Mat4::from_translation(*gp + if *fixed { vec3(((get_time() as f32) * 55.0).sin() * 0.008, ((get_time() as f32) * 70.0).cos() * 0.006, 0.0) } else { Vec3::ZERO }) * cabin_slope_mat(gp.x, gp.z, &outposts)); } for m in &gen_meshes { draw_mesh(m); } unsafe { get_internal_gl().quad_gl.pop_model_matrix(); } draw_cube(*gp + vec3(0.0, 0.9, 0.0), vec3(0.08, 0.08, 0.08), Some(&metal), if *fixed { GREEN } else { RED }); }
        for (fp, act) in &cans { if *act { unsafe { get_internal_gl().quad_gl.push_model_matrix(Mat4::from_translation(*fp) * cabin_slope_mat(fp.x, fp.z, &outposts)); } draw_mesh(&can_mesh); unsafe { get_internal_gl().quad_gl.pop_model_matrix(); } } }
        for (cp, cyaw, op, _) in &chests {
            if !*op {
                unsafe { get_internal_gl().quad_gl.push_model_matrix(Mat4::from_translation(*cp) * cabin_slope_mat(cp.x, cp.z, &outposts) * Mat4::from_rotation_y(*cyaw)); }
                draw_mesh(&chest_mesh);
                unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
                let iron = Color::from_rgba(38, 42, 48, 255);
                for bx in [-0.36f32, 0.0, 0.36] { draw_cube(*cp + vec3(bx, 0.29, 0.0), vec3(0.055, 0.58, 0.65), Some(&metal), iron); }
                for sx in [-0.51f32, 0.51] { draw_cube(*cp + vec3(sx, 0.22, 0.0), vec3(0.04, 0.08, 0.22), Some(&metal), iron); }
                draw_cube(*cp + vec3(0.0, 0.32, 0.32), vec3(0.14, 0.18, 0.04), Some(&metal), GOLD);
            }
        }
        for (bp, act) in &batteries {
            if *act {
                draw_cube(*bp, vec3(0.20, 0.14, 0.12), Some(&metal), Color::from_rgba(32, 34, 38, 255));
                draw_cube(*bp + vec3(0.0, 0.01, 0.0), vec3(0.205, 0.06, 0.125), Some(&metal), GOLD);
                draw_cube(*bp + vec3(-0.05, 0.085, 0.0), vec3(0.035, 0.035, 0.035), Some(&metal), LIGHTGRAY);
                draw_cube(*bp + vec3( 0.05, 0.085, 0.0), vec3(0.035, 0.035, 0.035), Some(&metal), RED);
            }
        }
        if !has_journal {
            let jp = tower_pos + vec3(0.35, 7.68, -1.05);
            draw_cube(jp, vec3(0.26, 0.045, 0.34), Some(&wood), Color::from_rgba(92, 48, 28, 255));
            draw_cube(jp + vec3(0.01, 0.0, 0.0), vec3(0.24, 0.035, 0.32), Some(&wood), Color::from_rgba(225, 215, 190, 255));
            draw_cube(jp + vec3(-0.12, 0.0, 0.0), vec3(0.03, 0.05, 0.345), Some(&metal), GOLD);
            draw_cube(jp + vec3(0.04, -0.01, 0.19), vec3(0.025, 0.01, 0.08), Some(&wood), RED);
        }
        if !has_radar {
            let rp = tower_pos + vec3(-0.35, 7.68, -1.05);
            draw_cube(rp, vec3(0.26, 0.09, 0.32), Some(&metal), Color::from_rgba(42, 54, 44, 255));
            draw_cube(rp + vec3(0.0, 0.048, 0.02), vec3(0.20, 0.01, 0.20), Some(&metal), Color::from_rgba(20, 185, 140, 255));
            draw_cube(rp + vec3(0.09, 0.04, -0.22), vec3(0.025, 0.025, 0.16), Some(&metal), BLACK);
            draw_cube(rp + vec3(-0.07, 0.05, -0.11), vec3(0.03, 0.02, 0.03), Some(&metal), RED);
            draw_cube(rp + vec3( 0.00, 0.05, -0.11), vec3(0.03, 0.02, 0.03), Some(&metal), GREEN);
        }
        if !has_map { draw_cube(map_pickup, vec3(0.35, 0.02, 0.28), Some(&wood), Color::from_rgba(190, 220, 180, 255)); }
        if in_game {
            mimic.draw(&bark, &wood, scare_t > 0.0);
            stalker.draw(&bark, &metal);
            for &(bp, act) in &breakers { draw_breaker_substation(bp, act, breakers_locked, &metal); }
            for &(bbp, taken) in &black_boxes { if !taken { draw_black_box(bbp, &metal); } }
            draw_fp_hands(cam, yaw, pitch, bob, equip_t, hand_grip, hotbar_slot, uv_charges, flare_count, adren_count, has_map, has_radar, &wood, &metal);
        }
        else if (matches!(screen, Screen::Intro) && intro_t > (3.2 + audio.radio_dur - 0.6)) || matches!(screen, Screen::Outro) {
            mimic.pos = vec3(47.5, terrain_y(47.5, 47.5), 47.5); mimic.yaw = 2.35; mimic.disguised = false;
            mimic.draw(&bark, &wood, false);
        }
        for &cp in &camps { if (cp - cam).length() < 42.0 { draw_campfire(cp, &bark, &rock); } }
        for &(dp, kind) in &dropped_items {
            let col = match kind { 0 => RED, 1 => SKYBLUE, 2 => ORANGE, 3 => VIOLET, _ => GREEN };
            draw_cube(dp, vec3(0.24, 0.16, 0.24), Some(&metal), col);
        } for &(fp, _) in &active_flares {
            let flk = ((get_time() as f32) * 26.0 + fp.x).sin().abs() * 0.08;
            draw_cube(fp, vec3(0.30, 0.07, 0.07), Some(&rock), RED);
            draw_cube(fp + vec3(0.14, 0.09 + flk, 0.0), vec3(0.12 + flk, 0.18 + flk * 1.5, 0.12 + flk), Some(&rock), Color::from_rgba(255, 65, 55, 255));
            draw_cube(fp + vec3(0.14, 0.12 + flk, 0.0), vec3(0.06, 0.12, 0.06), Some(&rock), WHITE);
        } gl_use_default_material();
        let moon_p = cam + vec3(-45.0, 58.0, -42.0);
        unsafe { get_internal_gl().quad_gl.push_model_matrix(Mat4::from_translation(moon_p)); } draw_mesh(&moon_mesh); unsafe { get_internal_gl().quad_gl.pop_model_matrix(); }
        if in_game { net.draw_teammates(&wood); }
        set_default_camera();
        if in_game { net.draw_nametags(cam, look); if mimic.disguised { net.draw_fake_nametag(mimic.pos, mimic.mimic_talking_t > 0.0, cam, look); } }
        match screen {
            Screen::Playing => {
                let fixed_count = gens.iter().filter(|g| g.3).count();
                let top_txt = if !has_journal {
                    "EXPLORE WATCHTOWER 4: FIND THE RANGER'S JOURNAL ON THE RADIO DESK".to_string()
                } else {
                    format!("GENS: {}/6 | BREAKERS: {} | BOXES: {}/3 | CANS: {}{}", fixed_count, if breakers_locked { "OK" } else { "NO" }, boxes_collected, fuel_cans, if eclipse_t > 0.0 { if is_safe_haven(pos, &outposts) { "  [ECLIPSE - SAFE]" } else { "  [ECLIPSE - SEEK SHELTER!]" } } else { "" })
                };
                draw_text(&top_txt, 30.0, 38.0, 21.0, if fixed_count == 6 && breakers_locked && boxes_collected >= 3 { GREEN } else { LIGHTGRAY });

                if let Some((lure_txt, lure_pos, ttl)) = mimic.persuasion_msg.clone() {
                    if ttl > 4.42 {
                        let d_lure = (lure_pos - cam).length().max(1.0);
                        let g_lure = (1.5 / (1.0 + d_lure * 0.12)) * vol;
                        if voice.trigger_mimic_voice_clone(g_lure) {
                            mimic.mimic_talking_t = 3.2;
                            let speaker = if net.peers.is_empty() { player_name.clone() } else { net.first_peer_name() };
                            mimic.persuasion_msg = Some((format!("[{}]: \"...\" (PLAYING BACK RECORDED VOICE)", speaker), lure_pos, ttl - 0.09));
                        }
                    }
                    let shown = mimic.persuasion_msg.as_ref().map(|m| m.0.as_str()).unwrap_or(&lure_txt);
                    draw_text(shown, screen_width() * 0.5 - (shown.len() as f32 * 4.6), screen_height() - 125.0, 20.0, Color::new(0.88, 0.92, 0.85, (ttl / 4.5).min(1.0)));
                }
                if anom_pulse > 0.15 {
                    draw_text("[ ANOMALY 04-B: ATMOSPHERIC BLOOD SURGE ]", screen_width() * 0.5 - 210.0, 68.0, 18.0, Color::new(0.95, 0.25, 0.25, anom_pulse));
                }
                if lost_notice_t > 0.0 {
                    lost_notice_t -= dt;
                    draw_text("[ THE MIMIC LOST YOUR TRAIL... ]", screen_width() * 0.5 - 190.0, 95.0, 24.0, Color::new(0.7, 0.85, 1.0, (lost_notice_t / 4.0).min(1.0)));
                }

                // Look-at interaction checks
                let jr_p = tower_pos + vec3(0.35, 7.68, -1.05);
                for bi in 0..2 {
                    let (bp, _act) = breakers[bi];
                    if !breakers_locked && 2.8 > (bp + vec3(0.0, 1.0, 0.0) - cam).length() && look.dot((bp + vec3(0.0, 1.0, 0.0) - cam).normalize()) > 0.95 {
                        draw_text(if bi == 0 { "PRESS [E] TO THROW WEST RIDGE BREAKER" } else { "PRESS [E] TO THROW EAST RIDGE BREAKER" }, screen_width() * 0.5 - 210.0, screen_height() * 0.5 + 40.0, 22.0, YELLOW);
                        if is_key_pressed(KeyCode::E) {
                            breakers[bi].1 = true; audio.play_click(vol);
                            if breakers[0].1 && breakers[1].1 { breakers_locked = true; breaker_timer = 0.0; }
                            else { breaker_timer = if net.peers.is_empty() { 55.0 } else { 12.0 }; }
                        }
                    }
                }
                for (bbp, taken) in &mut black_boxes {
                    if !*taken && 2.6 > (*bbp + vec3(0.0, 0.2, 0.0) - cam).length() && look.dot((*bbp + vec3(0.0, 0.2, 0.0) - cam).normalize()) > 0.95 {
                        draw_text("PRESS [E] TO RECOVER ANOMALY BLACK BOX", screen_width() * 0.5 - 215.0, screen_height() * 0.5 + 40.0, 22.0, ORANGE);
                        if is_key_pressed(KeyCode::E) { *taken = true; boxes_collected += 1; audio.play_click(vol); }
                    }
                }
                let mut picked_idx = None;
                for (di, &(dp, kind)) in dropped_items.iter().enumerate() {
                    if 2.5 > (dp - cam).length() && look.dot((dp - cam).normalize()) > 0.95 {
                        let lbl = match kind { 0 => "PRESS [E] TO PICK UP SHARED JERRYCAN", 1 => "PRESS [E] TO PICK UP SHARED UV CAMERA", 2 => "PRESS [E] TO PICK UP SHARED FLARE", 3 => "PRESS [E] TO PICK UP SHARED ADRENALINE", 5 => "PRESS [E] TO PICK UP SHARED BATTERY", _ => "PRESS [E] TO PICK UP SHARED RADAR" };
                        draw_text(lbl, screen_width() * 0.5 - 200.0, screen_height() * 0.5 + 40.0, 22.0, GOLD);
                        if is_key_pressed(KeyCode::E) {
                            picked_idx = Some(di); equip_t = 0.38; audio.play_click(vol);
                            match kind { 0 => fuel_cans += 1, 1 => uv_charges += 1, 2 => flare_count += 1, 3 => adren_count += 1, 5 => spare_bats += 1, _ => has_radar = true }
                        }
                    }
                }
                if let Some(di) = picked_idx { dropped_items.remove(di); }
                if !has_journal && 2.6 > (jr_p - cam).length() && look.dot((jr_p - cam).normalize()) > 0.968 {
                    draw_text("PRESS [E] TO READ RANGER'S JOURNAL", screen_width() * 0.5 - 195.0, screen_height() * 0.5 + 40.0, 22.0, GOLD);
                    if is_key_pressed(KeyCode::E) { has_journal = true; show_journal = true; audio.play_click(vol); }
                }

                let rd_p = tower_pos + vec3(-0.35, 7.68, -1.05);
                if !has_radar && (rd_p - cam).length() < 2.6 && look.dot((rd_p - cam).normalize()) > 0.968 && (rd_p - cam).length() <= (jr_p - cam).length() {
                    draw_text("PRESS [E] TO PICK UP MOTION RADAR", screen_width() * 0.5 - 190.0, screen_height() * 0.5 + 40.0, 22.0, SKYBLUE);
                    if is_key_pressed(KeyCode::E) { has_radar = true; audio.play_click(vol); }
                }

                if !has_map && (map_pickup - cam).length() < 2.5 && look.dot((map_pickup - cam).normalize()) > 0.965 {
                    draw_text("PRESS [E] TO TAKE SECTOR 4 TOPOGRAPHIC MAP", screen_width() * 0.5 - 230.0, screen_height() * 0.5 + 40.0, 22.0, GREEN);
                    if is_key_pressed(KeyCode::E) { has_map = true; show_map = true; audio.play_click(vol); }
                }

                for (bp, active) in &mut batteries {
                    if *active && (*bp - cam).length() < 2.4 && look.dot((*bp - cam).normalize()) > 0.965 {
                        draw_text("PRESS [E] TO STORE SPARE BATTERY IN BACKPACK", screen_width() * 0.5 - 220.0, screen_height() * 0.5 + 40.0, 22.0, YELLOW);
                        if is_key_pressed(KeyCode::E) { *active = false; spare_bats += 1; audio.play_click(vol); }
                    }
                }

                for (fp, active) in &mut cans {
                    if *active && (*fp - cam).length() < 2.5 && look.dot((*fp - cam).normalize()) > 0.965 {
                        draw_text("PRESS [E] TO COLLECT 20L FUEL JERRYCAN", screen_width() * 0.5 - 210.0, screen_height() * 0.5 + 40.0, 22.0, ORANGE);
                        if is_key_pressed(KeyCode::E) { *active = false; fuel_cans += 1; audio.play_click(vol); }
                    }
                }

                for (cp, _, opened, loot_kind) in &mut chests {
                    let c_mid = *cp + vec3(0.0, 0.35, 0.0);
                    if !*opened && (c_mid - cam).length() < 2.6 && look.dot((c_mid - cam).normalize()) > 0.965 {
                        draw_text("PRESS [E] TO UNLOCK SUPPLY CHEST", screen_width() * 0.5 - 180.0, screen_height() * 0.5 + 40.0, 22.0, GOLD);
                        if is_key_pressed(KeyCode::E) {
                            *opened = true; audio.play_click(vol);
                            match *loot_kind { 0 => fuel_cans += 1, 1 => uv_charges += 1, 2 => flare_count += 2, 3 => adren_count += 1, _ => spare_bats += 1 }
                        }
                    }
                }

                for (gp, prog, fueled, fixed) in &mut gens {
                    let g_mid = *gp + vec3(0.0, 0.45, 0.0);
                    if !*fixed && (g_mid - cam).length() < 2.7 && look.dot((g_mid - cam).normalize()) > 0.965 {
                        if !*fueled {
                            if fuel_cans > 0 {
                                draw_text("PRESS [E] TO POUR 20L FUEL CANISTER INTO GENERATOR", screen_width() * 0.5 - 250.0, screen_height() * 0.5 + 40.0, 22.0, ORANGE);
                                if is_key_pressed(KeyCode::E) { *fueled = true; fuel_cans -= 1; audio.play_click(vol); }
                            } else {
                                draw_text("GENERATOR EMPTY - REQUIRES 20L FUEL JERRYCAN", screen_width() * 0.5 - 230.0, screen_height() * 0.5 + 40.0, 22.0, RED);
                            }
                        } else {
                            draw_text(&format!("HOLD [E] TO REPAIR GENERATOR ({:.0}%)", (*prog / 4.5) * 100.0), screen_width() * 0.5 - 200.0, screen_height() * 0.5 + 40.0, 22.0, GREEN);
                            if is_key_down(KeyCode::E) {
                                *prog += dt; noise = 0.88;
                                if *prog >= 4.5 { *fixed = true; audio.play_click(vol); }
                            }
                        }
                    }
                }

                let radio_p = tower_pos + vec3(0.0, 7.65, -1.05);
                if fixed_count == 6 && (radio_p - cam).length() < 2.7 && look.dot((radio_p - cam).normalize()) > 0.89 {
                    draw_text(&format!("HOLD [E] ON RADIO CONSOLE TO CALL EXTRACTION ({:.0}%)", (radio_prog / 3.5) * 100.0), screen_width() * 0.5 - 250.0, screen_height() * 0.5 + 40.0, 22.0, GREEN);
                    if is_key_down(KeyCode::E) { radio_prog += dt; if radio_prog >= 3.5 { won = true; outro_t = 0.0; outro_vo = false; screen = Screen::Outro; } }
                }
                if show_journal && has_journal {
                    let (jw, jh, jx, jy) = (640.0, 440.0, screen_width() * 0.5 - 320.0, screen_height() * 0.5 - 220.0);
                    draw_rectangle(jx, jy, jw, jh, Color::from_rgba(24, 20, 16, 245));
                    draw_rectangle_lines(jx, jy, jw, jh, 2.5, Color::from_rgba(165, 135, 95, 255));
                    draw_text("RANGER FIELD JOURNAL - LOOKOUT TOWER 4  [J TO CLOSE]", jx + 24.0, jy + 36.0, 20.0, GOLD);
                    let j_lines = [
                        "SECTOR 4 SURVIVAL PROTOCOL (OCT 18, 1998):",
                        "1. ENTITY 04-B (MIMIC): Imitates voices. Crouch [C] + Light Off [F] hides you (3x max).",
                        "2. ENTITY 04-C (PALE STALKER): Full-on sprints until near; once close, DO NOT shine light on it!",
                        "3. ECLIPSE SURGES: Valley goes pitch-black; take shelter inside Cabins or Watchtowers.",
                        "4. EXTRACTION: Fix 6/6 Generators, sync West & East Ridge Breakers & get 3 Black Boxes.",
                        "5. Return to Watchtower 4 radio desk to call Eagle-1. [R] Reload Battery | [G] Share Gear.",
                    ];
                    for (idx, line) in j_lines.iter().enumerate() { draw_text(line, jx + 24.0, jy + 78.0 + idx as f32 * 32.0, 18.0, if idx == 7 { GREEN } else { LIGHTGRAY }); }
                }
                if show_map && has_map {
                    let (mw, mx, my) = (460.0, screen_width() * 0.5 - 230.0, screen_height() * 0.5 - 230.0);
                    draw_rectangle(mx - 18.0, my - 46.0, mw + 36.0, mw + 88.0, Color::from_rgba(10, 14, 16, 244));
                    draw_rectangle_lines(mx - 18.0, my - 46.0, mw + 36.0, mw + 88.0, 2.0, Color::from_rgba(90, 120, 105, 220));
                    draw_text("U.S.F.S. SECTOR 4 (380x380 BASIN)  [M TO CLOSE]", mx, my - 20.0, 18.0, Color::from_rgba(160, 210, 180, 255));
                    draw_rectangle(mx, my, mw, mw, Color::from_rgba(16, 22, 20, 255));
                    for (ci, &(ox, oz)) in outposts.iter().enumerate() {
                        let (cx, cz) = (mx + ((ox as f32 + 0.5) / 380.0) * mw, my + ((oz as f32 + 0.5) / 380.0) * mw);
                        draw_rectangle(cx - 4.0, cz - 4.0, 8.0, 8.0, Color::from_rgba(165, 125, 82, 255));
                        draw_text(&format!("C{}", ci + 1), cx + 6.0, cz + 4.0, 12.0, LIGHTGRAY);
                    }
                    for (bi, &(bp, _)) in breakers.iter().enumerate() {
                        let (bx, bz) = (mx + (bp.x / 380.0) * mw, my + (bp.z / 380.0) * mw);
                        let bcol = if breakers_locked { GREEN } else { YELLOW };
                        draw_circle(bx, bz, 5.0, bcol);
                        draw_text(if bi == 0 { "W-BRK" } else { "E-BRK" }, bx + 7.0, bz + 4.0, 12.0, bcol);
                    }
                    for &(bbp, taken) in &black_boxes {
                        if !taken {
                            let (bx, bz) = (mx + (bbp.x / 380.0) * mw, my + (bbp.z / 380.0) * mw);
                            draw_rectangle(bx - 4.0, bz - 4.0, 8.0, 8.0, ORANGE);
                            draw_text("BOX", bx + 6.0, bz + 4.0, 12.0, ORANGE);
                        }
                    }
                    for &(tx_w, tz_w) in &towers { draw_circle(mx + (tx_w / 380.0) * mw, my + (tz_w / 380.0) * mw, 5.5, SKYBLUE); }
                    for (idx, (gp, _, fueled, fixed)) in gens.iter().enumerate() { let _g_rot = (idx as f32) * 1.1;
                        let (gx, gz) = (mx + (gp.x / 380.0) * mw, my + (gp.z / 380.0) * mw);
                        let col = if *fixed { GREEN } else if *fueled { ORANGE } else { RED };
                        draw_rectangle(gx - 4.5, gz - 4.5, 9.0, 9.0, col); draw_text(&format!("G{}", idx + 1), gx + 7.0, gz + 4.0, 13.0, col);
                    }
                    let (px, pz) = (mx + (pos.x / 380.0) * mw, my + (pos.z / 380.0) * mw);
                    draw_circle(px, pz, 5.0, YELLOW); draw_line(px, pz, px + yaw.cos() * 16.0, pz + yaw.sin() * 16.0, 2.5, YELLOW);
                    if has_radar {
                        let t = get_time() as f32;
                        let sweep_r = (t * 145.0) % 440.0;
                        draw_circle_lines(px, pz, sweep_r, 1.5, Color::from_rgba(30, 235, 140, 120));
                        let (rx, rz) = (mx + (mimic.pos.x / 380.0) * mw, my + (mimic.pos.z / 380.0) * mw);
                        let pulse = 5.0 + (t * 5.0).sin().abs() * 3.0;
                        draw_circle_lines(rx, rz, pulse + 3.0, 1.5, Color::from_rgba(255, 65, 65, 170));
                        draw_circle(rx, rz, 5.2, RED);
                        draw_text("04-B MIMIC", rx + 8.0, rz + 4.0, 12.0, RED);
                        let (sx, sz) = (mx + (stalker.pos.x / 380.0) * mw, my + (stalker.pos.z / 380.0) * mw);
                        draw_circle_lines(sx, sz, pulse + 3.0, 1.5, Color::from_rgba(200, 110, 255, 170));
                        draw_circle(sx, sz, 5.2, VIOLET);
                        draw_text("04-C STALKER", sx + 8.0, sz + 4.0, 12.0, VIOLET);
                        draw_text("RADAR: LIVE TRACKING | BLUE=TOWERS | RED/ORANGE/GREEN=GENS", mx, my + mw + 24.0, 14.0, Color::from_rgba(30, 235, 140, 255));
                    } else { draw_text("RADAR: OFFLINE (CHECK TOWER 4 DESK)", mx, my + mw + 24.0, 15.0, ORANGE); }
                }
                if breaker_timer > 0.0 && !breakers_locked {
                    draw_text(&format!("BREAKER SYNC WINDOW: {:.1}S - THROW OPPOSITE RIDGE SWITCH!", breaker_timer), screen_width() * 0.5 - 270.0, 74.0, 20.0, YELLOW);
                }
                if radio_msg_t > 0.0 {
                    let calls = ["[RADIO - ALL VALLEY]: \"PULLING RIDGE BREAKER NOW - GET READY!\"", "[RADIO - ALL VALLEY]: \"SECOND ENTITY SPOTTED - DO NOT LOOK AT IT!\"", "[RADIO - ALL VALLEY]: \"RECOVERED A BLACK BOX - MOVING!\"", "[RADIO - ALL VALLEY]: \"ECLIPSE SURGE - GET INSIDE SHELTER!\""];
                    let cm = calls[radio_msg_idx];
                    draw_text(cm, screen_width() * 0.5 - (cm.len() as f32 * 4.4), 104.0, 19.0, GREEN);
                }
                if drag_t > 0.0 {
                    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.15, 0.0, 0.0, 0.35));
                    draw_text("SOMETHING IS DRAGGING YOU INTO THE WOODS...", screen_width() * 0.5 - 270.0, screen_height() - 150.0, 26.0, RED);
                }
                if scare_t > 0.0 {
                    scare_t -= dt;
                    let pulse = ((scare_t * 18.0).sin().abs()) * 0.42;
                    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.65, 0.02, 0.02, pulse));
                    draw_text("IT WAS NEVER YOUR FRIEND...", screen_width() * 0.5 - 230.0, screen_height() - 150.0, 34.0, RED);
                }
                if breaker_timer > 0.0 && !breakers_locked {
                    draw_text(&format!("BREAKER SYNC WINDOW: {:.1}S - THROW OPPOSITE RIDGE SWITCH!", breaker_timer), screen_width() * 0.5 - 270.0, 74.0, 20.0, YELLOW);
                }
                if radio_msg_t > 0.0 {
                    let calls = ["[RADIO - ALL VALLEY]: \"PULLING RIDGE BREAKER NOW - GET READY!\"", "[RADIO - ALL VALLEY]: \"SECOND ENTITY SPOTTED - DO NOT LOOK AT IT!\"", "[RADIO - ALL VALLEY]: \"RECOVERED A BLACK BOX - MOVING!\"", "[RADIO - ALL VALLEY]: \"ECLIPSE SURGE - GET INSIDE SHELTER!\""];
                    let cm = calls[radio_msg_idx];
                    draw_text(cm, screen_width() * 0.5 - (cm.len() as f32 * 4.4), 104.0, 19.0, GREEN);
                }
                draw_circle(screen_width() * 0.5, screen_height() * 0.5, 2.2, Color::from_rgba(235, 240, 245, 160));
                let (nx, ny, nw) = (screen_width() - 250.0, screen_height() - 42.0, 220.0);
                draw_text("NOISE LEVEL", nx, ny - 8.0, 18.0, LIGHTGRAY); draw_rectangle(nx, ny, nw, 16.0, Color::from_rgba(20, 22, 28, 200));
                draw_rectangle(nx, ny, nw * noise, 16.0, if noise > 0.6 { RED } else if noise > 0.3 { YELLOW } else { GREEN });
                let (sw, sh) = (screen_width(), screen_height());
                let slots = [
                    format!("1:RADIO [V]"),
                    if uv_cd > 0.0 { format!("2:UV ({:.0}S)", uv_cd) } else { format!("2:UV x{}", uv_charges) },
                    format!("3:FLARE x{}", flare_count),
                    if adren_t > 0.0 { format!("4:RUN ({:.0}S)", adren_t) } else { format!("4:ADREN x{}", adren_count) },
                    if has_radar { "5:RADAR".to_string() } else { "5:EMPTY".to_string() },
                ];
                draw_text("[B / TAB] BACKPACK  |  [G] DROP / SHARE  |  [1-5] HOTBAR", sw * 0.5 - 220.0, sh - 54.0, 14.0, LIGHTGRAY);
                let hb_x = sw * 0.5 - 335.0;
                for (i, lbl) in slots.iter().enumerate() {
                    let sx = hb_x + (i as f32) * 136.0;
                    draw_rectangle(sx, sh - 46.0, 130.0, 34.0, Color::from_rgba(12, 15, 22, 215));
                    draw_rectangle_lines(sx, sh - 46.0, 130.0, 34.0, 2.0, if hotbar_slot == i { GOLD } else { GRAY });
                    draw_text(lbl, sx + 8.0, sh - 24.0, 14.0, if hotbar_slot == i { WHITE } else { LIGHTGRAY });
                }
                if show_bag {
                    let (bx, by) = (sw * 0.5 - 250.0, sh * 0.5 - 170.0);
                    draw_rectangle(bx, by, 500.0, 320.0, Color::from_rgba(10, 12, 18, 245));
                    draw_rectangle_lines(bx, by, 500.0, 320.0, 2.0, GOLD);
                    draw_text("RANGER FIELD BACKPACK  [B / TAB TO CLOSE]", bx + 20.0, by + 32.0, 20.0, GOLD);
                    draw_text(&format!("20L FUEL JERRYCANS: {}   |   CIPHER KEYS: {}", fuel_cans, if has_journal { 1 } else { 0 }), bx + 20.0, by + 68.0, 17.0, ORANGE);
                    draw_text(&format!("[1] HALOGEN FLASHLIGHT: {:.0}% BATTERY", bat), bx + 20.0, by + 106.0, 17.0, WHITE);
                    draw_text(&format!("[2] UV STUN FLASH [Q]: {} CHARGES (STUNS 6S)", uv_charges), bx + 20.0, by + 138.0, 17.0, VIOLET);
                    draw_text(&format!("[3] MAGNESIUM FLARES: {} (REPELS ENTITY 28S)", flare_count), bx + 20.0, by + 170.0, 17.0, RED);
                    draw_text(&format!("[4] ADRENALINE SYRINGES: {} (20S SPRINT BOOST)", adren_count), bx + 20.0, by + 202.0, 17.0, SKYBLUE);
                    draw_text(&format!("[5] ENTITY MOTION RADAR: {}", if has_radar { "EQUIPPED (OPEN [M] MAP)" } else { "NOT FOUND (CHECK TOWER DESK)" }), bx + 20.0, by + 234.0, 17.0, GREEN);
                }
                draw_text("STAMINA", 30.0, screen_height() - 104.0, 16.0, LIGHTGRAY); draw_rectangle(30.0, screen_height() - 95.0, 180.0 * (stam / 100.0), 10.0, Color::from_rgba(95, 195, 235, 220));
                draw_text(if is_key_down(KeyCode::V) { "VOICE [V]: TRANSMITTING" } else { "VOICE [V]: HOLD TO TALK" }, 30.0, screen_height() - 72.0, 18.0, if is_key_down(KeyCode::V) { GREEN } else { GRAY });
                if camcorder {
                    let (sw, sh) = (screen_width(), screen_height());
                    let vc = Color::new(0.0, 0.0, 0.0, 0.38);
                    draw_rectangle(0.0, 0.0, sw, 26.0, vc); draw_rectangle(0.0, sh - 26.0, sw, 26.0, vc);
                    draw_rectangle_lines(18.0, 18.0, sw - 36.0, sh - 36.0, 1.5, Color::new(0.85, 0.90, 0.85, 0.25));
                    if (get_time() * 2.0).sin() > 0.0 { draw_circle(sw - 115.0, 44.0, 6.0, RED); }
                    draw_text("REC", sw - 100.0, 50.0, 22.0, Color::new(0.95, 0.95, 0.95, 0.85));
                    let mins = ((get_time() as u32) / 60) % 60;
                    draw_text(&format!("PM 11:{:02}", 14 + mins), sw - 220.0, sh - 108.0, 22.0, Color::new(0.92, 0.84, 0.45, 0.85));
                    draw_text("OCT. 18 1998", sw - 220.0, sh - 84.0, 22.0, Color::new(0.92, 0.84, 0.45, 0.85));
                }
                if uv_flash_t > 0.0 { draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.68, 0.55, 1.0, (uv_flash_t / 0.35) * 0.55)); }
                let uv_lbl = if uv_cd > 0.0 { format!("UV FLASH [Q]: {} (COOLDOWN {:.0}S)", uv_charges, uv_cd) } else { format!("UV FLASH [Q]: {} READY", uv_charges) };
                draw_text(&uv_lbl, 30.0, screen_height() - 134.0, 18.0, if uv_charges > 0 && uv_cd <= 0.0 { VIOLET } else { GRAY });
                draw_text(&format!("BATTERY ({} SPARE [R])", spare_bats), 30.0, screen_height() - 50.0, 18.0, LIGHTGRAY); draw_rectangle(30.0, screen_height() - 42.0, 180.0 * (bat / 100.0), 14.0, if bat < 25.0 { RED } else { Color::from_rgba(230, 210, 140, 230) });
                if wake_t < 3.2 { let p = (wake_t / 3.2).clamp(0.0, 1.0); let lid = (1.0 - p) * screen_height() * 0.52; draw_rectangle(0.0, 0.0, screen_width(), lid, BLACK); draw_rectangle(0.0, screen_height() - lid, screen_width(), lid, BLACK); draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, (1.0f32 - p).powf(1.5))); }
            }
            Screen::Intro => {
                if intro::draw_intro(dt, vol, &logo_tex, &audio, &mut intro_t, &mut intro_vo, &mut intro_vo2, &mut intro_vo3) {
                    audio.stop_intro_radio();
                    mimic.pos = vec3(190.0, 0.0, 210.0); mimic.stealth_evades = 0;
                    screen = if saved_name.is_empty() { Screen::UsernameSetup } else { Screen::Menu };
                }
            }
            Screen::Outro => {
                if intro::draw_outro(dt, vol, &player_name, &logo_tex, &audio, &mut outro_t, &mut outro_vo) {
                    audio.stop_outro_radio();
                    mimic.pos = vec3(190.0, 0.0, 210.0); mimic.stealth_evades = 0;
                    screen = Screen::Menu;
                }
            }
            Screen::UsernameSetup => {
                draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(6, 8, 12, 238));
                let (bx, by) = (screen_width() * 0.5 - 240.0, screen_height() * 0.5 - 120.0);
                draw_text("U.S. FOREST SERVICE - RANGER REGISTRATION", bx, by, 24.0, RED);
                draw_text("CHOOSE YOUR PERMANENT RANGER USERNAME:", bx, by + 42.0, 19.0, LIGHTGRAY);
                while let Some(ch) = get_char_pressed() { if (ch.is_ascii_alphanumeric() || ch == '_') && player_name.len() < 14 { player_name.push(ch.to_ascii_uppercase()); } }
                if is_key_pressed(KeyCode::Backspace) { player_name.pop(); }
                draw_rectangle(bx, by + 62.0, 480.0, 48.0, Color::from_rgba(20, 24, 30, 255));
                draw_rectangle_lines(bx, by + 62.0, 480.0, 48.0, 2.0, GREEN);
                draw_text(&format!("{}_", player_name), bx + 16.0, by + 94.0, 26.0, WHITE);
                if (ui_btn("SAVE & ENTER TOWER 4", bx, by + 130.0, 320.0, 48.0) || is_key_pressed(KeyCode::Enter)) && !player_name.trim().is_empty() {
                    let _ = std::fs::write(profile_path(), player_name.trim());
                    screen = Screen::Menu;
                }
            }
            Screen::Menu => {
                draw_text(&format!("RANGER: {}", player_name), 48.0, 42.0, 18.0, SKYBLUE);
                draw_rectangle(0.0, 0.0, 440.0, screen_height(), Color::from_rgba(6, 8, 12, 195));
                if won { draw_text("SHIFT SURVIVED - RESCUE EN ROUTE", 48.0, 75.0, 24.0, GREEN); }
                draw_text("THE NIGHT SHIFT AT", 48.0, 120.0, 26.0, Color::from_rgba(180, 185, 195, 255));
                draw_text("TOWER 4", 46.0, 178.0, 68.0, Color::from_rgba(210, 35, 35, 255)); draw_texture_ex(&logo_tex, 338.0, 118.0, WHITE, DrawTextureParams { dest_size: Some(vec2(58.0, 58.0)), ..Default::default() });
                if active_shift { if ui_btn("RESUME SHIFT", 48.0, 230.0, 320.0, 46.0) { screen = Screen::Playing; } }
                if ui_btn(if active_shift { "NEW RANDOM SHIFT" } else { "PLAY SHIFT" }, 48.0, if active_shift { 286.0 } else { 240.0 }, 320.0, 46.0) {
                    let new_seed = ((get_time() * 10000.0) as u32).wrapping_add(137); set_world_seed(new_seed); outposts = random_outposts(new_seed); terrain_chunks = build_terrain_chunks(&dirt, &outposts); camps = [vec3(outposts[0].0 as f32 + 4.8, terrain_y(outposts[0].0 as f32 + 4.8, outposts[0].1 as f32 + 3.8), outposts[0].1 as f32 + 3.8), vec3(outposts[1].0 as f32 + 4.8, terrain_y(outposts[1].0 as f32 + 4.8, outposts[1].1 as f32 + 3.8), outposts[1].1 as f32 + 3.8)]; gens = make_gens(&outposts); has_journal = false; has_map = true; has_radar = false; fuel_cans = 0; spare_bats = 0; uv_charges = 0; flare_count = 0; adren_count = 0; active_flares.clear(); batteries = [1, 3, 6, 7, 8].iter().map(|&idx| { let (ox, oz) = outposts[idx]; (vec3(ox as f32 - 2.1, terrain_y(ox as f32 + 0.5, oz as f32 + 0.5) + 1.04, oz as f32 - 1.2), true) }).collect();
                    cans = make_cans(&outposts); chests = make_chests(&outposts);
                    let (mx, mz) = outposts[0]; map_pickup = vec3(mx as f32 + 2.0, terrain_y(mx as f32 + 0.5, mz as f32 + 0.5) + 0.92, mz as f32 - 2.2);
                    screen = Screen::Playing; active_shift = true; wake_t = 0.0; eye_h = 0.25; pitch = -0.35;
                    pos = vec3(42.0, walkable_y(42.0, 42.3, &outposts), 42.3); yaw = -std::f32::consts::FRAC_PI_2; vel_y = 0.0; bat = 100.0; stam = 100.0;
                    radio_prog = 0.0;
                    mimic.pos = vec3(190.0, 0.0, 210.0); mimic.stealth_evades = 0; mimic.grace_timer = 4.0; mimic.phase = AiPhase::Searching; stalker.reset(); boxes_collected = 0; for bb in &mut black_boxes { bb.1 = false; } breakers_locked = false; breaker_timer = 0.0; breakers[0].1 = false; breakers[1].1 = false; noise = 0.0; won = false;
                }
                if ui_btn("HOST CO-OP LOBBY", 48.0, 342.0, 320.0, 46.0) { screen = Screen::HostSetup; }
                if ui_btn("JOIN CO-OP", 48.0, 398.0, 320.0, 46.0) { screen = Screen::Join; }
                if ui_btn("HOW TO PLAY & LORE", 48.0, 454.0, 320.0, 46.0) { screen = Screen::Lore; }
                if ui_btn("SETTINGS", 48.0, 510.0, 320.0, 46.0) { screen = Screen::Settings; }
                if ui_btn("WATCH ENDING", 48.0, 566.0, 320.0, 42.0) { outro_t = 0.0; outro_vo = false; screen = Screen::Outro; } if ui_btn("REPLAY INTRO", 48.0, 614.0, 320.0, 42.0) { intro_t = 0.0; intro_vo = false; intro_vo2 = false; intro_vo3 = false; screen = Screen::Intro; } if ui_btn("QUIT", 48.0, 662.0, 320.0, 42.0) { audio.shutdown(); break; }
            }
            Screen::HostSetup => {
                if is_key_pressed(KeyCode::Escape) { screen = Screen::Menu; }
                draw_rectangle(0.0, 0.0, 560.0, screen_height(), Color::from_rgba(6, 8, 12, 230));
                draw_text("HOST CO-OP LOBBY", 48.0, 110.0, 42.0, RED); let srv_ok = net.server_online.load(std::sync::atomic::Ordering::Relaxed);
                draw_text(if srv_ok { "SERVER STATUS: ONLINE - FRIENDS CAN JOIN BY CODE" } else { "SERVER STATUS: WAKING UP (WAIT ~30S)..." }, 48.0, 148.0, 18.0, if srv_ok { GREEN } else { ORANGE });
                draw_text("CUSTOM LOBBY CODE:", 48.0, 180.0, 19.0, LIGHTGRAY);
                while let Some(ch) = get_char_pressed() { if ch.is_ascii_alphanumeric() && lobby_code.len() < 12 { lobby_code.push(ch.to_ascii_uppercase()); } }
                if is_key_pressed(KeyCode::Backspace) { lobby_code.pop(); }
                draw_rectangle(48.0, 195.0, 420.0, 48.0, Color::from_rgba(20, 24, 30, 255)); draw_rectangle_lines(48.0, 195.0, 420.0, 48.0, 2.0, GREEN);
                draw_text(&format!("{}_", lobby_code), 64.0, 227.0, 26.0, WHITE);
                if ui_btn("START HOSTED SHIFT", 48.0, 265.0, 320.0, 50.0) || is_key_pressed(KeyCode::Enter) {
                    let new_seed = ((get_time() * 10000.0) as u32).wrapping_add(137); set_world_seed(new_seed); terrain_chunks = build_terrain_chunks(&dirt, &outposts); net.host_with_name(&lobby_code, &player_name); outposts = random_outposts(new_seed);
                    gens = make_gens(&outposts); batteries = [1, 3, 6, 7, 8].iter().map(|&idx| { let (ox, oz) = outposts[idx]; (vec3(ox as f32 - 2.1, terrain_y(ox as f32 + 0.5, oz as f32 + 0.5) + 1.04, oz as f32 - 1.2), true) }).collect(); cans = make_cans(&outposts); chests = make_chests(&outposts);
                    screen = Screen::Playing; active_shift = true; wake_t = 0.0; eye_h = 0.25; pitch = -0.35;
                    pos = vec3(42.0, walkable_y(42.0, 42.3, &outposts), 42.3); yaw = -std::f32::consts::FRAC_PI_2; vel_y = 0.0; bat = 100.0; stam = 100.0;
                    radio_prog = 0.0;
                    mimic.pos = vec3(190.0, 0.0, 210.0); mimic.stealth_evades = 0; mimic.grace_timer = 4.0; mimic.phase = AiPhase::Searching; stalker.reset(); boxes_collected = 0; for bb in &mut black_boxes { bb.1 = false; } breakers_locked = false; breaker_timer = 0.0; breakers[0].1 = false; breakers[1].1 = false; noise = 0.0; won = false;
                }
                if ui_btn("< BACK TO MENU", 48.0, screen_height() - 75.0, 280.0, 48.0) { screen = Screen::Menu; }
            }
            Screen::Join => {
                if is_key_pressed(KeyCode::Escape) { screen = Screen::Menu; }
                draw_rectangle(0.0, 0.0, 560.0, screen_height(), Color::from_rgba(6, 8, 12, 230));
                draw_text("JOIN CO-OP SHIFT", 48.0, 110.0, 42.0, RED);
                let srv_ok = net.server_online.load(std::sync::atomic::Ordering::Relaxed);
                draw_text(if srv_ok { "SERVER STATUS: ONLINE - READY TO JOIN" } else { "SERVER STATUS: WAKING UP (WAIT ~30S)..." }, 48.0, 148.0, 18.0, if srv_ok { GREEN } else { ORANGE });
                draw_text("ENTER LOBBY CODE:", 48.0, 180.0, 20.0, LIGHTGRAY);
                while let Some(ch) = get_char_pressed() { if ch.is_ascii_alphanumeric() && lobby_code.len() < 12 { lobby_code.push(ch.to_ascii_uppercase()); } }
                if is_key_pressed(KeyCode::Backspace) { lobby_code.pop(); }
                draw_rectangle(48.0, 195.0, 420.0, 48.0, Color::from_rgba(20, 24, 30, 255));
                draw_rectangle_lines(48.0, 195.0, 420.0, 48.0, 2.0, GREEN);
                draw_text(&format!("{}_", lobby_code), 64.0, 227.0, 26.0, WHITE);
                if ui_btn("DROP INTO LOBBY", 48.0, 265.0, 320.0, 50.0) || is_key_pressed(KeyCode::Enter) {
                    net.join_with_name(&lobby_code, &player_name); screen = Screen::Playing; active_shift = true; wake_t = 0.0; eye_h = 0.25;
                    pos = vec3(45.5, walkable_y(45.5, 45.5, &outposts), 45.5); vel_y = 0.0; bat = 100.0; stam = 100.0; won = false;
                }
                if ui_btn("< BACK TO MENU", 48.0, screen_height() - 75.0, 280.0, 48.0) { screen = Screen::Menu; }
            }
            Screen::Lore => {
                if is_key_pressed(KeyCode::Escape) { screen = Screen::Menu; }
                draw_rectangle(0.0, 0.0, 740.0, screen_height(), Color::from_rgba(6, 8, 12, 230));
                draw_text("CASE FILE: TOWER 4 (OCTOBER 1998)", 48.0, 65.0, 32.0, RED);
                let lines = ["Climb Watchtower 4 to read the Ranger's Journal [J] & take the Motion Radar.", "Search the 9 Outpost Cabins for the Sector Map [M] & 20L Fuel Jerrycans.", "Fuel & repair all 6 Relay Generators across the 380x380 valley, then extract.", "WASD: Walk | SHIFT: Sprint | C: Crouch | F: Light | J: Journal | M: Map | V: Voice"];
                for (i, line) in lines.iter().enumerate() { draw_text(line, 48.0, 120.0 + i as f32 * 30.0, 18.5, LIGHTGRAY); }
                if ui_btn("< BACK TO MENU", 48.0, screen_height() - 75.0, 280.0, 48.0) { screen = Screen::Menu; }
            }
            Screen::Settings => {
                if is_key_pressed(KeyCode::Escape) { *voice.test_loopback.lock().unwrap() = false; screen = Screen::Menu; }
                draw_rectangle(0.0, 0.0, 520.0, screen_height(), Color::from_rgba(6, 8, 12, 225));
                draw_text("SETTINGS", 48.0, 85.0, 44.0, RED);
                if ui_btn(&format!("AUDIO VOLUME: {:.0}%", vol * 100.0), 48.0, 140.0, 380.0, 46.0) { vol = if vol >= 1.0 { 0.0 } else { (vol + 0.25).min(1.0) }; audio.play_click(vol); }
                if ui_btn(&format!("BRIGHTNESS: {:.0}%", bright * 100.0), 48.0, 200.0, 380.0, 46.0) { bright = if bright >= 1.8 { 0.6 } else { bright + 0.3 }; }
                if ui_btn(&format!("GRAPHICS QUALITY: {}", ["LOW", "MEDIUM", "HIGH"][quality]), 48.0, 260.0, 380.0, 46.0) { quality = (quality + 1) % 3; }
                if ui_btn(&format!("MOUSE SENSITIVITY: {:.1}x", sens), 48.0, 320.0, 380.0, 46.0) { sens = if sens >= 2.0 { 0.5 } else { sens + 0.5 }; }
                let mut lb = voice.test_loopback.lock().unwrap(); if is_key_down(KeyCode::V) { *lb = true; }
                let testing = *lb;
                if ui_btn(if testing { "MIC LOOPBACK TEST: ON" } else { "MIC LOOPBACK TEST: OFF (OR HOLD V)" }, 48.0, 380.0, 420.0, 46.0) { *lb = !*lb; audio.play_click(vol); }
                drop(lb);
                let lvl = (*voice.mic_level.lock().unwrap() * 2.5).clamp(0.0, 1.0);
                draw_rectangle(48.0, 455.0, 420.0 * lvl, 16.0, if lvl > 0.7 { RED } else { GREEN });
                if ui_btn("< BACK TO MENU", 48.0, screen_height() - 75.0, 280.0, 48.0) { *voice.test_loopback.lock().unwrap() = false; screen = Screen::Menu; }
            }
        }
        next_frame().await;
    }
}































































