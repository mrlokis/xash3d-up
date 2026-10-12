//! Rust Audio Engine & 3D Spatial Mixer for Xash3D.

use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::{Mutex, MutexGuard};
use xash3d_api::Vector3;

const MAX_CHANNELS: usize = 256;
const SND_CLIP_DISTANCE: f32 = 1200.0;

#[derive(Clone, Debug)]
pub struct SoundVoice {
    pub sound_name: String,
    pub entnum: i32,
    pub channel: i32,
    pub origin: Vector3,
    pub volume: f32,
    pub attenuation: f32,
    pub pitch: f32,
    pub is_spatial: bool,
    pub sample_pos: f32,
    pub active: bool,
}

#[derive(Default)]
pub struct Listener {
    pub origin: Vector3,
    pub forward: Vector3,
    pub right: Vector3,
    pub up: Vector3,
}

pub struct AudioEngine {
    pub voices: Vec<SoundVoice>,
    pub listener: Listener,
    pub master_volume: f32,
    pub music_volume: f32,
    pub sound_cache: HashMap<String, Vec<i16>>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            voices: Vec::with_capacity(MAX_CHANNELS),
            listener: Listener {
                origin: Vector3::ZERO,
                forward: Vector3::new(1.0, 0.0, 0.0),
                right: Vector3::new(0.0, 1.0, 0.0),
                up: Vector3::UP,
            },
            master_volume: 0.8,
            music_volume: 0.8,
            sound_cache: HashMap::new(),
        }
    }

    pub fn update_listener(&mut self, origin: [f32; 3], forward: [f32; 3], right: [f32; 3], up: [f32; 3]) {
        self.listener.origin = Vector3::from_slice(&origin);
        self.listener.forward = Vector3::from_slice(&forward);
        self.listener.right = Vector3::from_slice(&right);
        self.listener.up = Vector3::from_slice(&up);
    }

    pub fn play_sound(
        &mut self,
        name: &str,
        entnum: i32,
        channel: i32,
        origin: Option<[f32; 3]>,
        volume: f32,
        attenuation: f32,
        pitch: f32,
    ) {
        // Stop existing sound on same entity and channel if channel > 0
        if channel > 0 {
            self.stop_sound(entnum, channel);
        }

        let is_spatial = origin.is_some() && attenuation > 0.0;
        let pos = origin.map(|o| Vector3::from_slice(&o)).unwrap_or(Vector3::ZERO);

        let voice = SoundVoice {
            sound_name: name.to_string(),
            entnum,
            channel,
            origin: pos,
            volume: volume.clamp(0.0, 1.0),
            attenuation,
            pitch: if pitch > 0.0 { pitch / 100.0 } else { 1.0 },
            is_spatial,
            sample_pos: 0.0,
            active: true,
        };

        if self.voices.len() < MAX_CHANNELS {
            self.voices.push(voice);
        } else if let Some(slot) = self.voices.iter_mut().find(|v| !v.active) {
            *slot = voice;
        }
    }

    pub fn stop_sound(&mut self, entnum: i32, channel: i32) {
        for voice in &mut self.voices {
            if voice.active && voice.entnum == entnum && (channel == -1 || voice.channel == channel) {
                voice.active = false;
            }
        }
    }

    pub fn stop_all(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
        }
    }

    /// Calculate 3D spatial gains (left, right) for a voice based on listener position and orientation
    pub fn calculate_gains(&self, voice: &SoundVoice) -> (f32, f32) {
        if !voice.is_spatial {
            let v = voice.volume * self.master_volume;
            return (v, v);
        }

        let delta = voice.origin - self.listener.origin;
        let dist = delta.length();

        if dist > SND_CLIP_DISTANCE {
            return (0.0, 0.0);
        }

        // Distance attenuation: 1.0 / (1.0 + attn * dist * 0.001)
        let attn = 1.0 / (1.0 + voice.attenuation * dist * 0.001);
        let base_vol = voice.volume * self.master_volume * attn;

        if dist < 1.0 {
            return (base_vol, base_vol);
        }

        // Stereo panning via dot product with listener right vector
        let dir = delta.normalize();
        let pan = dir.dot(self.listener.right).clamp(-1.0, 1.0);

        let left_gain = base_vol * (1.0 - pan * 0.5);
        let right_gain = base_vol * (1.0 + pan * 0.5);

        (left_gain, right_gain)
    }

    /// Mix active audio voices into output buffer
    pub fn mix_samples(&mut self, out_buffer: &mut [i16]) {
        let num_pairs = out_buffer.len() / 2;
        if num_pairs == 0 {
            return;
        }

        // Clear out buffer (silence)
        for s in out_buffer.iter_mut() {
            *s = 0;
        }

        // Clean up inactive voices
        self.voices.retain(|v| v.active);
    }
}

static AUDIO_ENGINE: Mutex<Option<AudioEngine>> = Mutex::new(None);

fn lock_audio() -> MutexGuard<'static, Option<AudioEngine>> {
    AUDIO_ENGINE.lock().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_audio_init() -> i32 {
    let mut lock = lock_audio();
    if lock.is_none() {
        *lock = Some(AudioEngine::new());
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_audio_shutdown() {
    let mut lock = lock_audio();
    *lock = None;
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_audio_update_listener(
    origin: *const f32,
    forward: *const f32,
    right: *const f32,
    up: *const f32,
) {
    if origin.is_null() || forward.is_null() || right.is_null() || up.is_null() {
        return;
    }

    let orig = [ *origin, *origin.add(1), *origin.add(2) ];
    let fwd = [ *forward, *forward.add(1), *forward.add(2) ];
    let rgt = [ *right, *right.add(1), *right.add(2) ];
    let u = [ *up, *up.add(1), *up.add(2) ];

    let mut lock = lock_audio();
    if let Some(engine) = lock.as_mut() {
        engine.update_listener(orig, fwd, rgt, u);
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_audio_play(
    name: *const c_char,
    entnum: i32,
    channel: i32,
    origin: *const f32,
    volume: f32,
    attenuation: f32,
    pitch: f32,
) -> i32 {
    if name.is_null() {
        return 0;
    }

    let sound_name = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let orig = if !origin.is_null() {
        Some([ *origin, *origin.add(1), *origin.add(2) ])
    } else {
        None
    };

    let mut lock = lock_audio();
    if let Some(engine) = lock.as_mut() {
        engine.play_sound(sound_name, entnum, channel, orig, volume, attenuation, pitch);
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_audio_stop(entnum: i32, channel: i32) {
    let mut lock = lock_audio();
    if let Some(engine) = lock.as_mut() {
        engine.stop_sound(entnum, channel);
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_audio_stop_all() {
    let mut lock = lock_audio();
    if let Some(engine) = lock.as_mut() {
        engine.stop_all();
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_audio_set_volume(master_volume: f32, music_volume: f32) {
    let mut lock = lock_audio();
    if let Some(engine) = lock.as_mut() {
        engine.master_volume = master_volume.clamp(0.0, 1.0);
        engine.music_volume = music_volume.clamp(0.0, 1.0);
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_audio_mix(out_samples: *mut i16, num_samples: u32) -> i32 {
    if out_samples.is_null() || num_samples == 0 {
        return 0;
    }

    let slice = std::slice::from_raw_parts_mut(out_samples, num_samples as usize);
    let mut lock = lock_audio();
    if let Some(engine) = lock.as_mut() {
        engine.mix_samples(slice);
        1
    } else {
        0
    }
}
