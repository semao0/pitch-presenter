//! GigaAM-v3 via transcribe.cpp (ggml: Metal on Apple, CPU elsewhere) with
//! Silero VAD from the same library.

use std::collections::VecDeque;
use std::path::Path;
use std::time::{Duration, Instant};

use transcribe_cpp::{Backend, Model, ModelOptions, RunOptions, Session, VadSession};

use crate::i18n::tr;

pub const RATE: usize = 16_000;
/// Audio kept for recognition: the last 6 s.
const KEEP: usize = 6 * RATE;
/// Between full passes only the last 3 s are decoded.
const PARTIAL: usize = 3 * RATE;
/// Less than this is too little to align against the text.
const MIN_AUDIO: usize = RATE * 4 / 5;
/// Silence this long ends a phrase.
const PHRASE_END: Duration = Duration::from_millis(300);
/// Silero frame: 512 samples, 32 ms.
const FRAME_MS: u64 = 32;
const SPEECH: f32 = 0.5;
const SILENCE: f32 = 0.35;

pub struct Recognizer {
    session: Session,
    vad: VadSession,
    options: RunOptions,
    samples: VecDeque<f32>,
    last_decode: Instant,
    last_full: Instant,
    interval: Duration,
    heard_speech: bool,
    speaking: bool,
    silent_frames: u64,
}

impl Recognizer {
    /// Loads the model and the VAD. The model lives in this value only: once the
    /// recognizer is dropped, transcribe.cpp frees its weights and GPU buffers.
    pub fn load(model: &Path, vad: &Path) -> Result<Self, String> {
        let load_error = |error: transcribe_cpp::Error| {
            tr!(
                "Не удалось загрузить GigaAM: {error}",
                "Could not load GigaAM: {error}"
            )
        };
        // ggml reports every graph allocation to stderr
        transcribe_cpp::disable_logging();
        let session = Model::load(model)
            .and_then(|model| model.session())
            .map_err(load_error)?;
        // The VAD scores 32 ms frames; GPU dispatch would cost more than the math.
        let cpu = ModelOptions {
            backend: Backend::Cpu,
            ..ModelOptions::default()
        };
        let vad = Model::load_with(vad, &cpu)
            .and_then(|model| model.vad_session())
            .map_err(|error| {
                tr!(
                    "Не удалось загрузить детектор речи: {error}",
                    "Could not load speech detector: {error}"
                )
            })?;
        Ok(Self {
            session,
            vad,
            options: RunOptions::default(),
            samples: VecDeque::with_capacity(KEEP),
            last_decode: Instant::now(),
            last_full: Instant::now(),
            interval: Duration::from_millis(300),
            heard_speech: false,
            speaking: false,
            silent_frames: 0,
        })
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        let _ = self.vad.stream_flush();
        self.heard_speech = false;
        self.speaking = false;
        self.silent_frames = 0;
        self.last_full = Instant::now();
    }

    /// Takes 16 kHz mono audio.
    pub fn accept(&mut self, samples: &[f32]) {
        if let Ok(result) = self.vad.stream_feed(samples) {
            for p in result.probs {
                if p >= SPEECH {
                    self.speaking = true;
                    self.heard_speech = true;
                    self.silent_frames = 0;
                } else if p < SILENCE {
                    self.silent_frames += 1;
                    if Duration::from_millis(self.silent_frames * FRAME_MS) >= PHRASE_END {
                        self.speaking = false;
                    }
                }
            }
        }
        self.samples.extend(samples);
        if self.samples.len() > KEEP {
            self.samples.drain(..self.samples.len() - KEEP);
        }
    }

    pub fn decode(&mut self) -> Option<String> {
        if !self.heard_speech
            || self.samples.len() < MIN_AUDIO
            || self.last_decode.elapsed() < self.interval
        {
            return None;
        }
        self.last_decode = Instant::now();
        // At the end of a phrase and every 1.5 s the whole buffer is decoded,
        // to recover context the short window cuts off.
        let full = !self.speaking || self.last_full.elapsed() >= Duration::from_millis(1500);
        if full {
            self.last_full = self.last_decode;
        }
        let samples = self.samples.make_contiguous();
        let start = if full {
            0
        } else {
            samples.len().saturating_sub(PARTIAL)
        };
        let text = self
            .session
            .run(&samples[start..], &self.options)
            .ok()?
            .text;
        // A slow machine decodes less often instead of falling behind.
        self.interval = (self.last_decode.elapsed() + Duration::from_millis(50))
            .max(Duration::from_millis(300));
        if !self.speaking {
            self.samples.clear();
        }
        self.heard_speech = false;
        Some(text)
    }
}

/// Microphone rate → 16 kHz. Downsampling averages each output period (a box
/// filter: enough for speech under 8 kHz), upsampling interpolates linearly.
pub struct Resampler {
    /// Input samples per output sample.
    step: f64,
    phase: f64,
    sum: f32,
    count: u32,
    last: f32,
}

impl Resampler {
    pub fn new(rate: u32) -> Self {
        Self {
            step: f64::from(rate) / 16_000.0,
            phase: 0.0,
            sum: 0.0,
            count: 0,
            last: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.sum = 0.0;
        self.count = 0;
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    pub fn resample(&mut self, input: &[f32]) -> Vec<f32> {
        let mut out = Vec::with_capacity((input.len() as f64 / self.step) as usize + 1);
        if self.step >= 1.0 {
            for &x in input {
                self.sum += x;
                self.count += 1;
                self.phase += 1.0;
                if self.phase >= self.step {
                    out.push(self.sum / self.count as f32);
                    self.phase -= self.step;
                    self.sum = 0.0;
                    self.count = 0;
                }
            }
        } else {
            for &x in input {
                while self.phase < 1.0 {
                    out.push(self.last + (x - self.last) * self.phase as f32);
                    self.phase += self.step;
                }
                self.phase -= 1.0;
                self.last = x;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{RATE, Recognizer, Resampler};

    /// The whole pipeline on a recording, in real time:
    /// `PITCH_PRESENTER_SPEECH_DIR=<app data>/speech/gigaam-v3 PITCH_PRESENTER_WAV=speech16k.wav
    /// cargo test --release -- --ignored --nocapture recognizer`
    #[test]
    #[ignore = "needs the downloaded model and a 16 kHz WAV"]
    fn recognizer_follows_a_recording() {
        let dir = std::path::PathBuf::from(std::env::var_os("PITCH_PRESENTER_SPEECH_DIR").unwrap());
        let wav = std::fs::read(std::env::var_os("PITCH_PRESENTER_WAV").unwrap()).unwrap();
        // 16-bit PCM after a canonical 44-byte header
        let pcm: Vec<f32> = wav[44..]
            .chunks_exact(2)
            .map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0)
            .collect();
        let mut recognizer =
            Recognizer::load(&dir.join(super::super::MODEL), &dir.join(super::super::VAD)).unwrap();
        let mut texts = 0;
        for chunk in pcm.chunks(RATE / 10) {
            recognizer.accept(chunk);
            if let Some(text) = recognizer.decode() {
                texts += 1;
                println!("{text}");
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(texts > 10, "{texts} decodes");
    }

    #[test]
    fn resampler_rates() {
        let second = vec![0.5_f32; 48_000];
        assert_eq!(Resampler::new(48_000).resample(&second).len(), 16_000);
        let n = Resampler::new(44_100)
            .resample(&vec![0.5_f32; 44_100])
            .len();
        assert!((15_999..=16_001).contains(&n), "{n}");
        let up = Resampler::new(8_000).resample(&vec![0.5_f32; 8_000]);
        assert_eq!(up.len(), 16_000);
        // a constant signal stays constant once the ramp from silence is over
        assert!(up[2..].iter().all(|x| (x - 0.5).abs() < 1e-6));
    }

    #[test]
    fn resampler_keeps_phase_across_chunks() {
        let mut r = Resampler::new(44_100);
        let total: usize = (0..10)
            .map(|_| r.resample(&vec![0.1_f32; 4_410]).len())
            .sum();
        assert!((15_999..=16_001).contains(&total), "{total}");
    }
}
