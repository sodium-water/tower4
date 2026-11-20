use macroquad::prelude::*;
use crate::audio::AudioEngine;

pub fn draw_intro(
    dt: f32,
    vol: f32,
    logo_tex: &Texture2D,
    audio: &AudioEngine,
    intro_t: &mut f32,
    intro_vo: &mut bool,
    intro_vo2: &mut bool,
    intro_vo3: &mut bool,
) -> bool {
    *intro_t += dt;
    let t = *intro_t;
    let (sw, sh) = (screen_width(), screen_height());
    let (mx, my) = mouse_position();
    let voice_start = 3.2f32;
    let t2 = voice_start + audio.radio_d1;
    let t3 = t2 + audio.radio_d2;
    let voice_end = voice_start + audio.radio_dur;
    let intro_end = voice_end + 1.8;

    if t >= voice_start && !*intro_vo { *intro_vo = true; audio.play_intro_radio1(vol); }
    if t >= t2 && !*intro_vo2 { *intro_vo2 = true; audio.play_intro_radio2(vol); }
    if t >= t3 && !*intro_vo3 { *intro_vo3 = true; audio.play_intro_radio3(vol); }

    if voice_start > t {
        draw_rectangle(0.0, 0.0, sw, sh, BLACK);
        let a = ((t / 0.8).min(1.0) * ((voice_start - t) / 0.6).min(1.0)).clamp(0.0, 1.0);
        let (cx, cy) = (sw * 0.5 + (mx - sw * 0.5) * 0.03, sh * 0.5 - 30.0 + (my - sh * 0.5) * 0.03);
        draw_texture_ex(logo_tex, cx - 56.0, cy - 110.0, Color::new(1.0, 1.0, 1.0, a), DrawTextureParams { dest_size: Some(vec2(112.0, 112.0)), ..Default::default() });
        draw_text("U.S. FOREST SERVICE  -  CASE FILE #04-B", cx - 165.0, cy + 28.0, 17.0, Color::new(0.72, 0.75, 0.80, a));
        draw_text("THE NIGHT SHIFT AT TOWER 4", cx - 188.0, cy + 62.0, 26.0, Color::new(0.95, 0.95, 0.95, a));
    } else {
        if is_key_down(KeyCode::N) {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.05, 0.35, 0.12, 0.28));
        }
        draw_rectangle(0.0, 0.0, sw, 34.0, Color::new(0.0, 0.0, 0.0, 0.68));
        draw_rectangle(0.0, sh - 114.0, sw, 114.0, Color::new(0.0, 0.0, 0.0, 0.82));
        if (get_time() * 2.5).sin() > 0.0 { draw_circle(42.0, 56.0, 6.0, RED); }
        draw_text("EVIDENCE TAPE #04-B  |  MOVE MOUSE TO PAN CAM  |  HOLD [N] NIGHT VISION", 58.0, 62.0, 17.0, Color::new(0.92, 0.84, 0.45, 0.92));
        // Live Radio Dispatch Waveform
        if voice_end > t {
            for b in 0..18 {
                let amp = ((t * 19.0 + b as f32 * 1.7).sin().abs()) * 14.0 + 3.0;
                draw_rectangle(sw - 220.0 + b as f32 * 9.0, 64.0 - amp * 0.5, 5.0, amp, GREEN);
            }
        }
        let (hdr, sub) = if t2 > t {
            ("[ INCIDENT BRIEFING: GEOMAGNETIC GRID OUTAGE ]", "\"Oct. 18, 1998. Severe geomagnetic storms have knocked out all 6 valley relay generators in Sector 4.\"")
        } else if t3 > t {
            ("[ PERSONNEL ALERT: RANGER R. MILLER - MISSING ]", "\"Lookout Ranger Miller went off-grid 2 hours ago near the timberline. Stay inside the watchtower catwalk.\"")
        } else if (voice_end + 0.15) > t {
            ("[ CONTAINMENT ALERT: ENTITIES 04-B & 04-C ]", "\"04-B mimics voices; 04-C stalks unseen & charges flashlight beams. Sync both breakers, recover 3 black boxes & call Eagle-1.\"")
        } else {
            ("[ ANOMALY 04-B DETECTED - SIGNAL LOST ]", "ALL LOOKOUT UNITS: RESTORE THE 6 GENERATORS AND CALL EXTRACTION FROM TOWER 4.")
        };
        draw_text(hdr, sw * 0.5 - (hdr.len() as f32 * 4.2), sh - 78.0, 16.0, if t >= t3 { RED } else { GOLD });
        draw_text(sub, sw * 0.5 - (sub.len() as f32 * 4.3), sh - 42.0, 19.0, WHITE);
    }

    draw_text("[SPACE / CLICK TO SKIP]  [F11: FULLSCREEN]", sw - 370.0, 24.0, 16.0, GRAY);
    t >= intro_end || is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) || is_mouse_button_pressed(MouseButton::Left)
}

pub fn draw_outro(
    dt: f32,
    vol: f32,
    player_name: &str,
    logo_tex: &Texture2D,
    audio: &AudioEngine,
    outro_t: &mut f32,
    outro_vo: &mut bool,
) -> bool {
    *outro_t += dt;
    let t = *outro_t;
    let (sw, sh) = (screen_width(), screen_height());
    if t >= 0.4 && !*outro_vo { *outro_vo = true; audio.play_outro_radio(vol); }

    if 9.2 > t {
        draw_rectangle(0.0, 0.0, sw, 36.0, Color::new(0.0, 0.0, 0.0, 0.72));
        draw_rectangle(0.0, sh - 108.0, sw, 108.0, Color::new(0.0, 0.0, 0.0, 0.80));
        if (get_time() * 4.0).sin() > 0.0 { draw_circle(42.0, 56.0, 6.0, GREEN); }
        draw_text("U.S.F.S. AIR RESCUE  |  CALLSIGN: EAGLE-1  |  EXTRACTION IN PROGRESS", 58.0, 62.0, 18.0, GREEN);
        let (hdr, sub) = if 4.5 > t {
            ("[ BEACON LOCKED - TOWER 4 CATWALK ]", "\"Rescue Eagle-1 to Tower 4... we have your relay beacon on visual.\"")
        } else {
            ("[ ALTITUDE ASCENDING - SECTOR 4 PERIMETER ]", "\"Lifting off now. Do NOT look back at the tree line.\"")
        };
        draw_text(hdr, sw * 0.5 - (hdr.len() as f32 * 4.2), sh - 74.0, 16.0, GOLD);
        draw_text(sub, sw * 0.5 - (sub.len() as f32 * 4.3), sh - 40.0, 19.0, WHITE);
    } else {
        draw_rectangle(0.0, 0.0, sw, sh, BLACK);
        let (cx, cy) = (sw * 0.5, sh * 0.5 - 50.0);
        draw_texture_ex(logo_tex, cx - 48.0, cy - 115.0, WHITE, DrawTextureParams { dest_size: Some(vec2(96.0, 96.0)), ..Default::default() });
        draw_text("CASE FILE #04-B  -  SHIFT DEBRIEF", cx - 155.0, cy + 12.0, 20.0, GOLD);
        draw_text("EXTRACTION SUCCESSFUL", cx - 150.0, cy + 48.0, 26.0, GREEN);
        let r_line = format!("SURVIVING LOOKOUT: {}   |   VALLEY RELAYS: 6 / 6 ONLINE", player_name);
        draw_text(&r_line, cx - (r_line.len() as f32 * 4.3), cy + 88.0, 18.0, LIGHTGRAY);
        draw_text("CONTAINMENT STATUS: ENTITY 04-B REMAINS AT LARGE IN SECTOR 4", cx - 260.0, cy + 120.0, 17.0, RED);
    }

    draw_text("[SPACE / CLICK TO CONTINUE]", sw - 265.0, 25.0, 16.0, GRAY);
    t >= 14.0 || is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) || is_mouse_button_pressed(MouseButton::Left)
}