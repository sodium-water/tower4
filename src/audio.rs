use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};
use macroquad::prelude::Vec3;

fn build_wav(sr: u32, samples: &[f32]) -> Vec<u8> {
    let bytes = (samples.len() * 2) as u32;
    let mut w = Vec::with_capacity(44 + bytes as usize);
    w.extend_from_slice(b"RIFF"); w.extend_from_slice(&(36 + bytes).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt "); w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&sr.to_le_bytes()); w.extend_from_slice(&(sr * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes()); w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data"); w.extend_from_slice(&bytes.to_le_bytes());
    for &s in samples { w.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()); }
    w
}

pub struct AudioEngine {
    wind: Sound, gen_hum: Sound, menu_music: Sound, intro_radio1: Sound, intro_radio2: Sound, intro_radio3: Sound, outro_radio: Sound, click: Sound, scare: Sound, roar: Sound, woman_scream: Sound, beat: Sound,
    steps: [Sound; 4], step_idx: usize, gen_pos: Vec3, beat_timer: f32, pub radio_d1: f32, pub radio_d2: f32, pub radio_dur: f32,
}

impl AudioEngine {
    pub async fn new(gen_pos: Vec3) -> Self {
        let sr = 22050u32;
        let mut seed = 13579u32;
        let mut rnd = || { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); ((seed >> 9) as f32 / 4194304.0) - 1.0 };

        let mut wind_s = Vec::with_capacity((sr * 4) as usize);
        let mut lp = 0.0f32;
        for i in 0..(sr * 4) {
            let w = rnd(); lp += 0.025 * (w - lp);
            wind_s.push(lp * 0.45 * (0.65 + 0.35 * ((i as f32 / sr as f32) * 1.57).sin()) );
        }

        let mut gen_s = Vec::with_capacity((sr * 2) as usize);
        for i in 0..(sr * 2) {
            let t = i as f32 / sr as f32;
            gen_s.push(((t * 55.0 * 6.2831).sin() * 0.6 + (t * 82.5 * 6.2831).sin() * 0.3 + rnd() * 0.05) * 0.55);
        }

        let click_s: Vec<f32> = (0..(sr / 22)).map(|i| {
            let env = (1.0 - i as f32 / (sr / 22) as f32).powi(3);
            (((i as f32 / sr as f32) * 1800.0 * 6.2831).sin() * 0.5 + rnd() * 0.5) * env * 0.8
        }).collect();

        let roar_len = (sr as f32 * 1.2) as usize;
        let mut roar_s = Vec::with_capacity(roar_len);
        let mut r_lp = 0.0f32;
        for i in 0..roar_len {
            let t = i as f32 / sr as f32;
            let env = (t * 8.0).min(1.0) * (1.0 - t / 1.2).powf(0.8);
            let growl = ((t * (78.0 - t * 22.0) * 6.2831).sin() * 2.5 + (t * 119.0 * 6.2831).sin() * 1.5).tanh();
            let n = rnd(); r_lp += 0.18 * (n - r_lp);
            roar_s.push((growl * 0.65 + r_lp * 0.8) * env * 0.92);
        }

        let beat_len = (sr as f32 * 0.45) as usize;
        let beat_s: Vec<f32> = (0..beat_len).map(|i| {
            let t = i as f32 / sr as f32;
            let b1 = (-t * 24.0).exp() * (t * 52.0 * 6.2831).sin();
            let b2 = if t > 0.16 { (-(t - 0.16) * 28.0).exp() * ((t - 0.16) * 46.0 * 6.2831).sin() * 0.85 } else { 0.0 };
            (b1 + b2).clamp(-1.0, 1.0) * 0.95
        }).collect();

        let sc_len = (sr as f32 * 1.4) as usize;
        let mut sc_s = Vec::with_capacity(sc_len);
        let mut s_lp = 0.0f32;
        for i in 0..sc_len {
            let t = i as f32 / sr as f32; let p = t / 1.4;
            let sub = (t * 38.0 * 6.2831).sin() * (-p * 4.5).exp() * 1.4;
            let brass = ((t * 110.0 * 6.2831).sin() + (t * 116.5 * 6.2831).sin() + (t * 155.6 * 6.2831).sin()).tanh() * 0.45 * (1.0 - p).powf(0.7);
            let n = rnd(); s_lp += 0.35 * (n - s_lp);
            sc_s.push((sub + brass + (n - s_lp) * (-p * 3.0).exp() * 0.75).clamp(-0.98, 0.98));
        }

        let mut steps_v = Vec::new();
        for v in 0..4 {
            let len = (sr as f32 * 0.16) as usize;
            let (mut buf, mut lp1, mut lp2) = (Vec::with_capacity(len), 0.0f32, 0.0f32);
            let pitch_v = 52.0 + (v as f32) * 3.5;
            for i in 0..len {
                let p = i as f32 / len as f32; let t = i as f32 / sr as f32; let n = rnd();
                lp1 += 0.028 * (n - lp1); lp2 += 0.075 * (lp1 - lp2);
                let env = (p * 18.0).min(1.0) * (-p * 13.0).exp();
                let thump = (t * pitch_v * 6.2831).sin() * (-p * 24.0).exp() * 0.55;
                buf.push(((lp2 * 2.8 + thump) * env).clamp(-0.95, 0.95));
            }
            steps_v.push(load_sound_from_bytes(&build_wav(sr, &buf)).await.unwrap());
        }

        let wind = load_sound_from_bytes(&build_wav(sr, &wind_s)).await.unwrap();
        let gen_hum = load_sound_from_bytes(&build_wav(sr, &gen_s)).await.unwrap();
        let click = load_sound_from_bytes(&build_wav(sr, &click_s)).await.unwrap();
        let scare = load_sound_from_bytes(&build_wav(sr, &sc_s)).await.unwrap();
        let roar = load_sound_from_bytes(&build_wav(sr, &roar_s)).await.unwrap();
        let ws_len = (sr as f32 * 2.4) as usize;
        let mut ws_s = vec![0.0f32; ws_len];
        let mut phase = 0.0f32;
        let (mut f1_y1, mut f1_y2, mut f2_y1, mut f2_y2, mut f3_y1, mut f3_y2, mut air_lp) = (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for i in 0..ws_len {
            let t = i as f32 / sr as f32;
            if t < 1.65 {
                let p = t / 1.65;
                let env = (t * 18.0).min(1.0) * (1.0 - p).powf(0.85);
                let jitter = (t * 9.5 * 6.2831).sin() * 14.0 + (t * 17.3 * 6.2831).sin() * 6.0;
                let f0 = if p < 0.18 { 480.0 + (p / 0.18) * 390.0 } else { 870.0 - ((p - 0.18) / 0.82).powf(1.3) * 360.0 } + jitter;
                phase = (phase + f0 / sr as f32) % 1.0;
                let glottal = (1.0 - 2.0 * phase) * 0.65 + (phase * 6.2831).sin() * 0.45;
                let n = rnd(); air_lp += 0.22 * (n - air_lp);
                let src = glottal * 0.82 + (n - air_lp) * 0.32 * env;
                // Open-mouth "AAAH" vocal tract formants (F1=880Hz, F2=1450Hz, F3=2850Hz)
                let (c1, r1) = (2.0 * 0.965 * (2.0 * std::f32::consts::PI * 880.0 / sr as f32).cos(), 0.965 * 0.965);
                let y1 = 0.14 * src + c1 * f1_y1 - r1 * f1_y2; f1_y2 = f1_y1; f1_y1 = y1;
                let (c2, r2) = (2.0 * 0.955 * (2.0 * std::f32::consts::PI * 1450.0 / sr as f32).cos(), 0.955 * 0.955);
                let y2 = 0.11 * src + c2 * f2_y1 - r2 * f2_y2; f2_y2 = f2_y1; f2_y1 = y2;
                let (c3, r3) = (2.0 * 0.945 * (2.0 * std::f32::consts::PI * 2850.0 / sr as f32).cos(), 0.945 * 0.945);
                let y3 = 0.06 * src + c3 * f3_y1 - r3 * f3_y2; f3_y2 = f3_y1; f3_y1 = y3;
                let raw_sig = ((y1 + y2 + y3) * 1.15 * env).clamp(-0.85, 0.85); f3_y2 += 0.24 * (raw_sig - f3_y2); let sig = f3_y2;
                ws_s[i] += sig;
                let d1 = i + (sr as f32 * 0.28) as usize;
                let d2 = i + (sr as f32 * 0.56) as usize;
                if d1 < ws_len { ws_s[d1] += sig * 0.36; }
                if d2 < ws_len { ws_s[d2] += sig * 0.18; }
            }
        }
        let woman_scream = load_sound_from_bytes(&build_wav(sr, &ws_s)).await.unwrap();
        let beat = load_sound_from_bytes(&build_wav(sr, &beat_s)).await.unwrap();
        let mm_len = (sr as f32 * 32.0) as usize;
        let mut mm_s = vec![0.0f32; mm_len];
        let chords: [[f32; 4]; 4] = [
            [110.00, 220.00, 261.63, 329.63], // Am
            [87.31,  174.61, 220.00, 329.63], // Fmaj7
            [73.42,  146.83, 174.61, 261.63], // Dm7
            [82.41,  164.81, 207.65, 293.66], // E7
        ];
        let arp_notes: [[f32; 8]; 4] = [
            [440.00, 523.25, 659.25, 698.46, 659.25, 523.25, 493.88, 440.00],
            [349.23, 440.00, 523.25, 659.25, 523.25, 440.00, 392.00, 349.23],
            [293.66, 349.23, 440.00, 523.25, 440.00, 349.23, 329.63, 293.66],
            [329.63, 415.30, 493.88, 587.33, 659.25, 587.33, 493.88, 415.30],
        ];
        let lead_notes: [f32; 8] = [659.25, 698.46, 659.25, 587.33, 523.25, 493.88, 415.30, 440.00];
        let mut pad_lp = 0.0f32;
        for i in 0..mm_len {
            let t = i as f32 / sr as f32;
            let bar = ((t / 4.0) as usize) % 4;
            let bar_t = t % 4.0;
            let step = ((t / 0.5) as usize) % 8;
            let step_t = t % 0.5;
            let drift = 1.0 + (t * 0.35 * 6.2831).sin() * 0.003;

            // 1. Sub-bass & Warm Detuned Horror String Pad
            let ch = chords[bar];
            let sub = (t * ch[0] * 0.5 * 6.2831).sin() * 0.26;
            let raw_pad = (t * ch[1] * drift * 6.2831).sin()
                        + (t * ch[2] * (2.0 - drift) * 6.2831).sin() * 0.85
                        + (t * ch[3] * drift * 6.2831).sin() * 0.75;
            pad_lp += 0.045 * (raw_pad - pad_lp);
            let swell = 0.65 + 0.35 * ((bar_t / 4.0) * std::f32::consts::PI).sin();
            let pad = pad_lp * swell * 0.22;

            // 2. Haunting 8th-note Piano / Music-Box Arpeggio (enters at bar 2 / 8s)
            let mut arp = 0.0f32;
            if t >= 8.0 {
                let nf = arp_notes[bar][step] * drift;
                let a_env = (step_t * 28.0).min(1.0) * (-step_t * 4.2).exp();
                arp = ((t * nf * 6.2831).sin() * 0.65 + (t * nf * 2.0 * 6.2831).sin() * 0.24 * (-step_t * 8.0).exp()) * a_env * 0.28;
            }

            // 3. Climax Dark Violin/Synth Lead Melody (24s - 32s)
            let mut lead = 0.0f32;
            if t >= 24.0 {
                let l_idx = (((t - 24.0) / 1.0) as usize) % 8;
                let l_t = (t - 24.0) % 1.0;
                let lf = lead_notes[l_idx] * (1.0 + (t * 5.2 * 6.2831).sin() * 0.005);
                let l_env = (l_t * 6.0).min(1.0) * (1.0 - l_t * 0.45);
                lead = ((t * lf * 6.2831).sin() * 0.5 + (t * lf * 2.0 * 6.2831).sin() * 0.25 + (t * lf * 0.5 * 6.2831).sin() * 0.25) * l_env * 0.24;
            }

            // 4. Slow Cinematic Timpani / Heartbeat Pulse (every 2.0s)
            let beat_t = t % 2.0;
            let kick = if beat_t < 0.35 { (beat_t * (58.0 - beat_t * 45.0) * 6.2831).sin() * (-beat_t * 11.0).exp() * 0.28 } else { 0.0 };

            let dry = sub + pad + arp + lead + kick;
            mm_s[i] += dry;
            let echo = (arp + lead) * 0.38;
            let d1 = i + (sr as f32 * 0.375) as usize;
            let d2 = i + (sr as f32 * 0.750) as usize;
            if d1 < mm_len { mm_s[d1] += echo; }
            if d2 < mm_len { mm_s[d2] += echo * 0.45; }
        }
        for s in &mut mm_s { *s = (*s * 1.15).clamp(-0.92, 0.92); }
        let menu_music = load_sound_from_bytes(&build_wav(sr, &mm_s)).await.unwrap();
        play_sound(&menu_music, PlaySoundParams { looped: true, volume: 0.0 });
        let r1_bytes = std::fs::read("assets/intro_1.wav").unwrap_or_else(|_| build_wav(sr, &click_s));
        let r2_bytes = std::fs::read("assets/intro_2.wav").unwrap_or_else(|_| build_wav(sr, &click_s));
        let r3_bytes = std::fs::read("assets/intro_3.wav").unwrap_or_else(|_| build_wav(sr, &click_s));
        let radio_d1 = ((r1_bytes.len().saturating_sub(44)) as f32 / 44100.0).max(3.0);
        let radio_d2 = ((r2_bytes.len().saturating_sub(44)) as f32 / 44100.0).max(3.0);
        let radio_d3 = ((r3_bytes.len().saturating_sub(44)) as f32 / 44100.0).max(3.5);
        let radio_dur = radio_d1 + radio_d2 + radio_d3;
        let intro_radio1 = match load_sound_from_bytes(&r1_bytes).await { Ok(s) => s, Err(_) => load_sound_from_bytes(&build_wav(sr, &click_s)).await.unwrap() };
        let intro_radio2 = match load_sound_from_bytes(&r2_bytes).await { Ok(s) => s, Err(_) => load_sound_from_bytes(&build_wav(sr, &click_s)).await.unwrap() };
        let intro_radio3 = match load_sound_from_bytes(&r3_bytes).await { Ok(s) => s, Err(_) => load_sound_from_bytes(&build_wav(sr, &click_s)).await.unwrap() };
        let out_bytes = std::fs::read("assets/outro_radio.wav").unwrap_or_else(|_| build_wav(sr, &click_s));
        let outro_radio = match load_sound_from_bytes(&out_bytes).await { Ok(s) => s, Err(_) => load_sound_from_bytes(&build_wav(sr, &click_s)).await.unwrap() };
        play_sound(&wind, PlaySoundParams { looped: true, volume: 0.0 });
        play_sound(&gen_hum, PlaySoundParams { looped: true, volume: 0.0 });

        Self { wind, gen_hum, menu_music, intro_radio1, intro_radio2, intro_radio3, outro_radio, click, scare, roar, woman_scream, beat, steps: [steps_v.remove(0), steps_v.remove(0), steps_v.remove(0), steps_v.remove(0)], step_idx: 0, gen_pos, beat_timer: 0.0, radio_d1, radio_d2, radio_dur }
    }

    pub fn update(&mut self, dt: f32, vol: f32, fade: f32, p_pos: Vec3, mimic_dist: f32) {
        let v = vol * fade;
        set_sound_volume(&self.wind, 0.08 * v);
        set_sound_volume(&self.menu_music, if fade < 0.0 { 0.0 } else { 0.42 * vol * (1.0 - fade) });
        let dist = (self.gen_pos - p_pos).length().max(0.5);
        set_sound_volume(&self.gen_hum, (0.35 / (1.0 + dist * dist * 0.35)).min(0.22) * v);
        if fade > 0.5 && mimic_dist < 22.0 {
            let panic = (1.0 - mimic_dist / 22.0).clamp(0.0, 1.0);
            self.beat_timer -= dt;
            if self.beat_timer <= 0.0 {
                self.beat_timer = 1.15 - panic * 0.72;
                play_sound(&self.beat, PlaySoundParams { looped: false, volume: (0.25 + panic * 0.75) * v });
            }
        }
    }

    pub fn play_intro_radio1(&self, vol: f32) { play_sound(&self.intro_radio1, PlaySoundParams { looped: false, volume: vol }); }
    pub fn play_intro_radio2(&self, vol: f32) { play_sound(&self.intro_radio2, PlaySoundParams { looped: false, volume: vol }); }
    pub fn play_intro_radio3(&self, vol: f32) { play_sound(&self.intro_radio3, PlaySoundParams { looped: false, volume: vol }); }
    pub fn play_outro_radio(&self, vol: f32) { play_sound(&self.outro_radio, PlaySoundParams { looped: false, volume: vol }); }
    pub fn stop_outro_radio(&self) { stop_sound(&self.outro_radio); }
    pub fn stop_intro_radio(&self) { stop_sound(&self.intro_radio1); stop_sound(&self.intro_radio2); stop_sound(&self.intro_radio3); stop_sound(&self.outro_radio); }
    pub fn play_roar(&self, vol: f32) { play_sound(&self.roar, PlaySoundParams { looped: false, volume: vol }); }
    pub fn play_woman_scream(&self, vol: f32) { play_sound(&self.woman_scream, PlaySoundParams { looped: false, volume: vol }); }
    pub fn play_scare(&self, vol: f32) { play_sound(&self.scare, PlaySoundParams { looped: false, volume: vol }); }
    pub fn play_click(&self, vol: f32) { play_sound(&self.click, PlaySoundParams { looped: false, volume: 0.85 * vol }); }
    pub fn play_step(&mut self, vol: f32, crouch: bool, sprint: bool) {
        let v = if crouch { 0.45 } else if sprint { 1.0 } else { 0.85 };
        play_sound(&self.steps[self.step_idx], PlaySoundParams { looped: false, volume: v * vol });
        self.step_idx = (self.step_idx + 1) % 4;
    }
    pub fn shutdown(&self) { stop_sound(&self.wind); stop_sound(&self.menu_music); stop_sound(&self.intro_radio1); stop_sound(&self.intro_radio2); stop_sound(&self.intro_radio3); stop_sound(&self.outro_radio); stop_sound(&self.gen_hum); }
}











