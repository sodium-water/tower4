use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct VoiceChat {
    pub mic_level: Arc<Mutex<f32>>,
    pub out_queue: Arc<Mutex<VecDeque<Vec<u8>>>>,
    pub in_queue: Arc<Mutex<VecDeque<(Vec<u8>, f32)>>>,
    pub ptt_active: Arc<Mutex<bool>>,
    pub test_loopback: Arc<Mutex<bool>>,
    pub voice_clips: Arc<Mutex<Vec<Vec<Vec<u8>>>>>,
    pub active_clip: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl VoiceChat {
    pub fn new() -> Self {
        let (mic_level, out_queue, in_queue, ptt_active, test_loopback) = (
            Arc::new(Mutex::new(0.0f32)), Arc::new(Mutex::new(VecDeque::new())),
            Arc::new(Mutex::new(VecDeque::<(Vec<u8>, f32)>::new())), Arc::new(Mutex::new(false)),
            Arc::new(Mutex::new(false)),
        );
        let voice_clips = Arc::new(Mutex::new(Vec::<Vec<Vec<u8>>>::new()));
        let active_clip = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));

        #[cfg(target_os = "windows")]
        {
            #[cfg(target_os = "windows")]
            use winapi::shared::minwindef::{DWORD, UINT};
            #[cfg(target_os = "windows")]
            use winapi::shared::mmreg::*;
            #[cfg(target_os = "windows")]
            use winapi::um::mmeapi::*;
            #[cfg(target_os = "windows")]
            use winapi::um::mmsystem::*;
            const WHDR_DONE: DWORD = 0x00000001;

            let (ml, oq, iq_in, ptt, lb, vc_mic, ac_mic) = (mic_level.clone(), out_queue.clone(), in_queue.clone(), ptt_active.clone(), test_loopback.clone(), voice_clips.clone(), active_clip.clone());
            #[cfg(target_os = "windows")]
            std::thread::spawn(move || unsafe {
                let fmt = WAVEFORMATEX { wFormatTag: WAVE_FORMAT_PCM, nChannels: 1, nSamplesPerSec: 11025, nAvgBytesPerSec: 11025, nBlockAlign: 1, wBitsPerSample: 8, cbSize: 0 };
                let mut h_in: HWAVEIN = std::ptr::null_mut();
                if waveInOpen(&mut h_in, WAVE_MAPPER, &fmt, 0, 0, CALLBACK_NULL) != 0 { return; }
                let mut bufs = vec![vec![128u8; 550]; 4];
                let mut hdrs: Vec<WAVEHDR> = bufs.iter_mut().map(|b| WAVEHDR { lpData: b.as_mut_ptr() as *mut i8, dwBufferLength: 550, dwBytesRecorded: 0, dwUser: 0, dwFlags: 0, dwLoops: 0, lpNext: std::ptr::null_mut(), reserved: 0 }).collect();
                for h in &mut hdrs { waveInPrepareHeader(h_in, h, std::mem::size_of::<WAVEHDR>() as UINT); waveInAddBuffer(h_in, h, std::mem::size_of::<WAVEHDR>() as UINT); }
                waveInStart(h_in);
                let mut idx = 0;
                loop {
                    if (hdrs[idx].dwFlags & WHDR_DONE) != 0 {
                        let talking = *ptt.lock().unwrap();
                        let testing = *lb.lock().unwrap();
                        let data = &bufs[idx][..hdrs[idx].dwBytesRecorded as usize];
                        let peak = data.iter().map(|&s| (s as f32 - 128.0).abs() / 128.0).fold(0.0f32, f32::max);
                        *ml.lock().unwrap() = if talking || testing { peak } else { 0.0 };
                        if talking && !data.is_empty() {
                            let pkt = data.to_vec();
                            let mut q = oq.lock().unwrap();
                            if 8 > q.len() { q.push_back(pkt.clone()); }
                            let mut ac = ac_mic.lock().unwrap();
                            if 160 > ac.len() { ac.push(pkt); }
                        } else if testing && !data.is_empty() {
                            let mut q = iq_in.lock().unwrap();
                            if 6 > q.len() { q.push_back((data.to_vec(), 1.1)); }
                        } else {
                            let mut ac = ac_mic.lock().unwrap();
                            if ac.len() >= 6 {
                                let mut clips = vc_mic.lock().unwrap();
                                if clips.len() >= 10 { clips.remove(0); }
                                clips.push(ac.clone());
                            }
                            ac.clear();
                        }
                        hdrs[idx].dwFlags &= !WHDR_DONE;
                        waveInAddBuffer(h_in, &mut hdrs[idx], std::mem::size_of::<WAVEHDR>() as UINT);
                        idx = (idx + 1) % 4;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(12));
                }
            });

            let iq = in_queue.clone();
            #[cfg(target_os = "windows")]
            std::thread::spawn(move || unsafe {
                let fmt = WAVEFORMATEX { wFormatTag: WAVE_FORMAT_PCM, nChannels: 1, nSamplesPerSec: 11025, nAvgBytesPerSec: 11025, nBlockAlign: 1, wBitsPerSample: 8, cbSize: 0 };
                let mut h_out: HWAVEOUT = std::ptr::null_mut();
                if waveOutOpen(&mut h_out, WAVE_MAPPER, &fmt, 0, 0, CALLBACK_NULL) != 0 { return; }
                loop {
                    let next_pkt: Option<(Vec<u8>, f32)> = iq.lock().unwrap().pop_front();
                    if let Some((mut pcm, gain)) = next_pkt {
                        for s in pcm.iter_mut() { *s = (((*s as f32 - 128.0) * gain) + 128.0).clamp(0.0, 255.0) as u8; }
                        let mut hdr = WAVEHDR { lpData: pcm.as_mut_ptr() as *mut i8, dwBufferLength: pcm.len() as DWORD, dwBytesRecorded: 0, dwUser: 0, dwFlags: 0, dwLoops: 0, lpNext: std::ptr::null_mut(), reserved: 0 };
                        waveOutPrepareHeader(h_out, &mut hdr, std::mem::size_of::<WAVEHDR>() as UINT);
                        waveOutWrite(h_out, &mut hdr, std::mem::size_of::<WAVEHDR>() as UINT);
                        std::thread::sleep(std::time::Duration::from_millis(48));
                        waveOutUnprepareHeader(h_out, &mut hdr, std::mem::size_of::<WAVEHDR>() as UINT);
                    } else { std::thread::sleep(std::time::Duration::from_millis(10)); }
                }
            });
        }
        #[cfg(not(target_os = "windows"))]
        {
            let iq = in_queue.clone();
            std::thread::spawn(move || {
                loop {
                    let mut batch = Vec::new();
                    {
                        let mut q = iq.lock().unwrap();
                        while let Some((pcm, gain)) = q.pop_front() {
                            for &s in &pcm {
                                batch.push((((s as f32 - 128.0) * gain) + 128.0).clamp(0.0, 255.0) as u8);
                            }
                            if batch.len() >= 4400 { break; }
                        }
                    }
                    if !batch.is_empty() {
                        let len = batch.len() as u32;
                        let mut wav = Vec::with_capacity(44 + batch.len());
                        wav.extend_from_slice(b"RIFF");
                        wav.extend_from_slice(&(36 + len).to_le_bytes());
                        wav.extend_from_slice(b"WAVEfmt ");
                        wav.extend_from_slice(&16u32.to_le_bytes());
                        wav.extend_from_slice(&1u16.to_le_bytes());
                        wav.extend_from_slice(&1u16.to_le_bytes());
                        wav.extend_from_slice(&11025u32.to_le_bytes());
                        wav.extend_from_slice(&11025u32.to_le_bytes());
                        wav.extend_from_slice(&1u16.to_le_bytes());
                        wav.extend_from_slice(&8u16.to_le_bytes());
                        wav.extend_from_slice(b"data");
                        wav.extend_from_slice(&len.to_le_bytes());
                        wav.extend_from_slice(&batch);
                        let tmp = std::env::temp_dir().join("tower4_voice.wav");
                        if std::fs::write(&tmp, &wav).is_ok() {
                            #[cfg(target_os = "macos")]
                            let _ = std::process::Command::new("afplay").arg(&tmp).status();
                            #[cfg(target_os = "linux")]
                            let _ = std::process::Command::new("aplay").arg("-q").arg(&tmp).status();
                        }
                    } else {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                }
            });
        }

        Self { mic_level, out_queue, in_queue, ptt_active, test_loopback, voice_clips, active_clip }
    }
    pub fn record_peer_packet(&self, pkt: &[u8]) {
        let mut clips = self.voice_clips.lock().unwrap();
        if clips.is_empty() || clips.last().unwrap().len() >= 150 {
            if clips.len() >= 10 { clips.remove(0); }
            clips.push(Vec::new());
        }
        clips.last_mut().unwrap().push(pkt.to_vec());
    }
    pub fn trigger_mimic_voice_clone(&self, gain: f32) -> bool {
        let clips = self.voice_clips.lock().unwrap();
        if clips.is_empty() { return false; }
        let idx = ((macroquad::time::get_time() * 3.7) as usize + clips.len() - 1) % clips.len();
        let mut iq = self.in_queue.lock().unwrap();
        for frame in &clips[idx] {
            if 160 > iq.len() { iq.push_back((frame.clone(), gain.clamp(0.45, 1.35))); }
        }
        true
    }
    pub fn trigger_distant_voice_echo(&self, gain: f32) -> bool {
        let clips = self.voice_clips.lock().unwrap();
        if clips.is_empty() { return false; }
        let idx = ((macroquad::time::get_time() * 3.7) as usize + clips.len() - 1) % clips.len();
        let mut iq = self.in_queue.lock().unwrap();
        let dist_gain = gain.clamp(0.14, 0.30);
        let mut lp = 128.0f32;
        for frame in &clips[idx] {
            if 160 > iq.len() {
                let mut muff = Vec::with_capacity(frame.len());
                for &b in frame {
                    lp += 0.30 * (b as f32 - lp);
                    muff.push(lp.clamp(0.0, 255.0) as u8);
                }
                iq.push_back((muff, dist_gain));
            }
        }
        for frame in clips[idx].iter().take(24) {
            if 160 > iq.len() { iq.push_back((frame.clone(), dist_gain * 0.42)); }
        }
        true
    }
}
