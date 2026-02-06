use crate::recorder::capture::core::wasapi::AudioSample;
use std::sync::{Arc, Mutex};

pub struct AudioMixer {
    game_buffer: Arc<Mutex<Vec<AudioSample>>>,
    mic_buffer: Arc<Mutex<Vec<AudioSample>>>,
    has_game_audio: bool,
    has_microphone: bool,

    game_volume: u8,
    mic_volume: u8,
}

impl AudioMixer {
    pub fn new(
        has_game_audio: bool,
        has_microphone: bool,
        game_volume: u8,
        mic_volume: u8,
    ) -> Self {
        Self {
            game_buffer: Arc::new(Mutex::new(Vec::new())),
            mic_buffer: Arc::new(Mutex::new(Vec::new())),
            has_game_audio,
            has_microphone,
            game_volume: game_volume.min(100),
            mic_volume: mic_volume.min(100),
        }
    }

    pub fn add_game_sample(&self, sample: AudioSample) {
        if self.has_game_audio {
            self.game_buffer.lock().unwrap().push(sample);
        }
    }

    pub fn add_mic_sample(&self, sample: AudioSample) {
        if self.has_microphone {
            self.mic_buffer.lock().unwrap().push(sample);
        }
    }

    /// Get next mixed sample
    /// Returns None if no samples available yet
    pub fn get_next_mixed(&self) -> Option<AudioSample> {
        match (self.has_game_audio, self.has_microphone) {
            (true, true) => {
                // Both sources - need to mix
                let mut game_buf = self.game_buffer.lock().unwrap();
                let mut mic_buf = self.mic_buffer.lock().unwrap();

                // Wait until we have samples from both sources
                if game_buf.is_empty() || mic_buf.is_empty() {
                    return None;
                }

                let game_sample = game_buf.remove(0);
                let mic_sample = mic_buf.remove(0);

                Some(self.mix_two_samples(&game_sample, &mic_sample))
            }
            (true, false) => {
                // no microphone
                let mut buf = self.game_buffer.lock().unwrap();
                if buf.is_empty() {
                    None
                } else {
                    let sample = buf.remove(0);
                    Some(self.apply_volume(sample, self.game_volume))
                }
            }
            (false, true) => {
                // microphone only
                let mut buf = self.mic_buffer.lock().unwrap();
                if buf.is_empty() {
                    None
                } else {
                    let sample = buf.remove(0);
                    Some(self.apply_volume(sample, self.mic_volume))
                }
            }
            (false, false) => {
                None
            }
        }
    }

    fn mix_two_samples(&self, game: &AudioSample, mic: &AudioSample) -> AudioSample {
        // Use the longer sample as base
        let max_len = game.data.len().max(mic.data.len());
        let mut mixed_data = Vec::with_capacity(max_len);

        // Convert to i16 slices for mixing
        let game_samples = bytemuck::cast_slice::<u8, i16>(&game.data);
        let mic_samples = bytemuck::cast_slice::<u8, i16>(&mic.data);

        let len = game_samples.len().min(mic_samples.len());

        for i in 0..len {
            // Apply volume and mix
            let game_adjusted = (game_samples[i] as i32 * self.game_volume as i32) / 100;
            let mic_adjusted = (mic_samples[i] as i32 * self.mic_volume as i32) / 100;

            let mixed = ((game_adjusted + mic_adjusted) / 2).clamp(-32768, 32767) as i16;
            mixed_data.extend_from_slice(&mixed.to_le_bytes());
        }

        if game_samples.len() > len {
            for i in len..game_samples.len() {
                mixed_data.extend_from_slice(&game_samples[i].to_le_bytes());
            }
        } else if mic_samples.len() > len {
            for i in len..mic_samples.len() {
                mixed_data.extend_from_slice(&mic_samples[i].to_le_bytes());
            }
        }

        AudioSample {
            data: mixed_data,
            timestamp: game.timestamp,
        }
    }

    fn apply_volume(&self, mut sample: AudioSample, volume: u8) -> AudioSample {
        if volume == 100 {
            return sample;
        }

        let samples = bytemuck::cast_slice_mut::<u8, i16>(&mut sample.data);

        for sample_val in samples.iter_mut() {
            let adjusted = ((*sample_val as i32 * volume as i32) / 100)
                .clamp(-32768, 32767);
            *sample_val = adjusted as i16;
        }

        sample
    }
}