use std::collections::VecDeque;
use std::path::Path;
use std::time::{Duration, Instant};

use sherpa_onnx::{
    OfflineRecognizer, OfflineRecognizerConfig, SileroVadModelConfig, VadModelConfig,
    VoiceActivityDetector,
};

use crate::i18n::tr;

pub struct Recognizer {
    model: OfflineRecognizer,
    vad: VoiceActivityDetector,
    samples: VecDeque<f32>,
    pending: VecDeque<f32>,
    last_decode: Instant,
    last_full: Instant,
    interval: Duration,
    heard_speech: bool,
}

impl Recognizer {
    pub fn load(directory: &Path) -> Result<Self, String> {
        let threads =
            std::thread::available_parallelism().map_or(2, |count| (count.get() / 2).clamp(1, 8));
        let threads = i32::try_from(threads).unwrap_or(2);
        let model_path = |name: &str| Some(directory.join(name).to_string_lossy().into_owned());
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.nemo_ctc.model = model_path("model.int8.onnx");
        config.model_config.tokens = model_path("tokens.txt");
        config.model_config.num_threads = threads;
        let model = OfflineRecognizer::create(&config)
            .ok_or_else(|| tr!("Не удалось загрузить GigaAM", "Could not load GigaAM"))?;
        let vad = VoiceActivityDetector::create(
            &VadModelConfig {
                silero_vad: SileroVadModelConfig {
                    model: model_path("silero_vad.onnx"),
                    threshold: 0.5,
                    min_silence_duration: 0.3,
                    min_speech_duration: 0.15,
                    window_size: 512,
                    max_speech_duration: 10.0,
                },
                sample_rate: 16000,
                num_threads: 1,
                ..Default::default()
            },
            15.0,
        )
        .ok_or_else(|| {
            tr!(
                "Не удалось загрузить детектор речи",
                "Could not load speech detector"
            )
        })?;
        Ok(Self {
            model,
            vad,
            samples: VecDeque::with_capacity(48_512),
            pending: VecDeque::with_capacity(1024),
            last_decode: Instant::now(),
            last_full: Instant::now(),
            interval: Duration::from_millis(300),
            heard_speech: false,
        })
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        self.pending.clear();
        self.vad.reset();
        self.heard_speech = false;
        self.last_full = Instant::now();
    }

    pub fn accept(&mut self, samples: &[f32]) {
        self.pending.extend(samples);
        let mut frame = [0.0; 512];
        while self.pending.len() >= frame.len() {
            for sample in &mut frame {
                *sample = self.pending.pop_front().expect("complete audio frame");
            }
            self.vad.accept_waveform(&frame);
            self.heard_speech |= self.vad.detected();
            while !self.vad.is_empty() {
                self.vad.pop();
            }
            self.samples.extend(frame);
            if self.samples.len() > 48_000 {
                self.samples.drain(..self.samples.len() - 48_000);
            }
        }
    }

    pub fn decode(&mut self) -> Option<String> {
        if !self.heard_speech
            || self.samples.len() < 12_800
            || self.last_decode.elapsed() < self.interval
        {
            return None;
        }
        self.last_decode = Instant::now();
        let full = !self.vad.detected() || self.last_full.elapsed() >= Duration::from_millis(1500);
        if full {
            self.last_full = self.last_decode;
        }
        let samples = self.samples.make_contiguous();
        let start = if full {
            0
        } else {
            samples.len().saturating_sub(32_000)
        };
        let stream = self.model.create_stream();
        stream.accept_waveform(16000, &samples[start..]);
        self.model.decode(&stream);
        self.interval = (self.last_decode.elapsed() + Duration::from_millis(50))
            .max(Duration::from_millis(300));
        if !self.vad.detected() {
            self.samples.clear();
        }
        self.heard_speech = false;
        stream.get_result().map(|result| result.text)
    }
}
