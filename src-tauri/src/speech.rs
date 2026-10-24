mod recognition;

use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::ipc::Channel;
use tauri::{Manager, State, WebviewWindow};

use crate::i18n::tr;

/// GigaAM-v3 e2e-ctc (punctuation, digits) in GGUF Q8_0 for transcribe.cpp, and
/// Silero VAD. Revisions are pinned and every file is checked by SHA-256; the
/// licenses and NOTICEs are kept next to the weights, as MIT requires.
const GIGAAM: &str = "https://huggingface.co/handy-computer/gigaam-v3-e2e-ctc-gguf/resolve/59717de63d50dd9dd93b3249da6ad3f2f1c4ad6b";
const SILERO: &str = "https://huggingface.co/handy-computer/silero-vad-v6.2-gguf/resolve/a9b2d61c20ae4ddf3644edb34969e8185a485448";
const MODEL: &str = "gigaam-v3-e2e-ctc-Q8_0.gguf";
const VAD: &str = "silero-vad-v6.2-F32.gguf";
/// (local name, base URL, remote name, SHA-256)
const FILES: [(&str, &str, &str, &str); 6] = [
    (
        "LICENSE",
        GIGAAM,
        "LICENSE",
        "f00de6715714c7a63d08639cdbfaa40224eefc407302614bd19f1a8b98c875aa",
    ),
    (
        "NOTICE",
        GIGAAM,
        "NOTICE",
        "1c62a9e00fc06da6f7da6c80e9e7e856e3c10a179bb5e73b97d61426dfb584af",
    ),
    (
        MODEL,
        GIGAAM,
        MODEL,
        "9ccce4750dc813a493d96ca15ee251712bedec15ac9a02fa3d2bd732f08ae5eb",
    ),
    (
        "silero-vad.LICENSE",
        SILERO,
        "LICENSE",
        "2e63e9a38b6e8fc0c7bc37ce174caca1862870856c6daf5697cfb785e925520b",
    ),
    (
        "silero-vad.NOTICE",
        SILERO,
        "NOTICE",
        "f1ea43096a23b40d4b14fcf131586710035444e69e35051c5d83c60725b50555",
    ),
    (
        VAD,
        SILERO,
        VAD,
        "0d871a800c420fdd5d1b17a3aea4296d1c39608f0ff4a9295eb76457eb1c88e6",
    ),
];

#[derive(Default)]
pub struct SpeechState {
    worker: Mutex<Option<Worker>>,
    context: Arc<AtomicU64>,
}

struct Worker {
    cancel: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl SpeechState {
    pub fn stop(&self) {
        if let Some(worker) = self.worker.lock().expect("speech lock").as_ref() {
            worker.cancel.store(true, Ordering::Relaxed);
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SpeechEvent {
    Download { file: String, bytes: u64 },
    Loading,
    Listening,
    Text { text: String, context: u64 },
    Error { message: String },
    Stopped,
}

fn check_window(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == crate::windows::MAIN {
        Ok(())
    } else {
        Err(tr!(
            "Доступ только из окна спикера",
            "Only available in the presenter window"
        ))
    }
}

#[tauri::command]
pub fn speech_start(
    window: WebviewWindow,
    state: State<'_, SpeechState>,
    context: u64,
    on_event: Channel<SpeechEvent>,
) -> Result<(), String> {
    check_window(&window)?;
    let mut worker = state.worker.lock().expect("speech lock");
    if worker
        .as_ref()
        .is_some_and(|worker| !worker.thread.is_finished())
    {
        return Err(tr!(
            "Распознавание ещё останавливается",
            "Speech recognition is still stopping"
        ));
    }
    if let Some(previous) = worker.take() {
        let _ = previous.thread.join();
    }
    let directory = window
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("speech/gigaam-v3");
    let cancel = Arc::new(AtomicBool::new(false));
    state.context.store(context, Ordering::Relaxed);
    let thread_cancel = cancel.clone();
    let context = state.context.clone();
    let thread = std::thread::spawn(move || {
        let result = run(&directory, &thread_cancel, &context, &on_event);
        if let Err(message) = result {
            if !thread_cancel.load(Ordering::Relaxed) {
                let _ = on_event.send(SpeechEvent::Error { message });
            }
        }
        let _ = on_event.send(SpeechEvent::Stopped);
    });
    *worker = Some(Worker { cancel, thread });
    Ok(())
}

#[tauri::command]
pub fn speech_stop(window: WebviewWindow, state: State<'_, SpeechState>) -> Result<(), String> {
    check_window(&window)?;
    state.stop();
    Ok(())
}

#[tauri::command]
pub fn speech_context(
    window: WebviewWindow,
    state: State<'_, SpeechState>,
    context: u64,
) -> Result<(), String> {
    check_window(&window)?;
    state.context.store(context, Ordering::Relaxed);
    Ok(())
}

fn cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err(String::new())
    } else {
        Ok(())
    }
}

fn hash_file(path: &Path, cancel: &AtomicBool) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 65536];
    loop {
        cancelled(cancel)?;
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    let mut encoded = String::with_capacity(64);
    for byte in hash.finalize() {
        use std::fmt::Write;
        write!(encoded, "{byte:02x}").expect("write to string");
    }
    Ok(encoded)
}

fn prepare(
    directory: &Path,
    cancel: &AtomicBool,
    events: &Channel<SpeechEvent>,
) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    remove_stale(directory);
    for (name, base, remote, hash) in FILES {
        cancelled(cancel)?;
        let path = directory.join(name);
        if hash_file(&path, cancel).is_ok_and(|actual| actual == hash) {
            continue;
        }
        let url = format!("{base}/{remote}");
        let temporary = path.with_extension("part");
        let download = || -> Result<(), String> {
            let response = ureq::get(&url)
                .config()
                .timeout_connect(Some(Duration::from_secs(20)))
                .timeout_per_read(Some(Duration::from_secs(15)))
                .build()
                .call()
                .map_err(|error| error.to_string())?;
            let mut reader = response.into_body().into_reader();
            let mut output =
                std::fs::File::create(&temporary).map_err(|error| error.to_string())?;
            let mut buffer = vec![0; 65536];
            let mut bytes = 0_u64;
            let mut report = Instant::now();
            loop {
                cancelled(cancel)?;
                let count = reader
                    .read(&mut buffer)
                    .map_err(|error| error.to_string())?;
                if count == 0 {
                    break;
                }
                bytes += count as u64;
                if bytes > 300_000_000 {
                    return Err(tr!(
                        "Файл модели слишком большой",
                        "Model file is too large"
                    ));
                }
                output
                    .write_all(&buffer[..count])
                    .map_err(|error| error.to_string())?;
                if report.elapsed() >= Duration::from_millis(250) {
                    events
                        .send(SpeechEvent::Download {
                            file: name.to_owned(),
                            bytes,
                        })
                        .map_err(|error| error.to_string())?;
                    report = Instant::now();
                }
            }
            output.sync_all().map_err(|error| error.to_string())?;
            drop(output);
            if hash_file(&temporary, cancel)? != hash {
                return Err(tr!(
                    "Контрольная сумма модели не совпала",
                    "Model checksum mismatch"
                ));
            }
            if path.exists() {
                std::fs::remove_file(&path).map_err(|error| error.to_string())?;
            }
            std::fs::rename(&temporary, &path).map_err(|error| error.to_string())
        };
        if let Err(error) = download() {
            let _ = std::fs::remove_file(&temporary);
            return Err(tr!(
                "Не удалось скачать {name}: {error}",
                "Could not download {name}: {error}"
            ));
        }
    }
    Ok(())
}

/// Deletes what is not ours to keep: unfinished downloads and the files of the
/// sherpa-onnx model used before 2.2 (≈230 МБ).
fn remove_stale(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !FILES.iter().any(|(file, ..)| name == *file) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn input<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    sender: mpsc::SyncSender<Vec<f32>>,
    errors: mpsc::SyncSender<String>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &_| {
                let mono = data
                    .chunks_exact(channels)
                    .map(|frame| {
                        frame
                            .iter()
                            .map(|sample| sample.to_sample::<f32>())
                            .sum::<f32>()
                            / f32::from(u16::try_from(channels).unwrap_or(1))
                    })
                    .collect();
                let _ = sender.try_send(mono);
            },
            move |error| {
                let _ = errors.try_send(error.to_string());
            },
            None,
        )
        .map_err(|error| error.to_string())
}

fn microphone(
    sender: mpsc::SyncSender<Vec<f32>>,
    errors: mpsc::SyncSender<String>,
) -> Result<(cpal::Stream, u32), String> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or_else(|| tr!("Микрофон не найден", "No microphone found"))?;
    let config = device
        .default_input_config()
        .map_err(|error| error.to_string())?;
    let rate = config.sample_rate();
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => input::<f32>(&device, config.into(), sender, errors),
        cpal::SampleFormat::F64 => input::<f64>(&device, config.into(), sender, errors),
        cpal::SampleFormat::I16 => input::<i16>(&device, config.into(), sender, errors),
        cpal::SampleFormat::I32 => input::<i32>(&device, config.into(), sender, errors),
        cpal::SampleFormat::U16 => input::<u16>(&device, config.into(), sender, errors),
        format => Err(tr!(
            "Формат микрофона не поддерживается: {format}",
            "Unsupported microphone format: {format}"
        )),
    }?;
    Ok((stream, rate))
}

fn run(
    directory: &Path,
    cancel: &AtomicBool,
    context: &AtomicU64,
    events: &Channel<SpeechEvent>,
) -> Result<(), String> {
    prepare(directory, cancel, events)?;
    cancelled(cancel)?;
    let _ = events.send(SpeechEvent::Loading);
    let mut recognizer =
        recognition::Recognizer::load(&directory.join(MODEL), &directory.join(VAD))?;
    cancelled(cancel)?;
    let (sender, receiver) = mpsc::sync_channel(128);
    let (errors, error_receiver) = mpsc::sync_channel(1);
    let (stream, rate) = microphone(sender, errors).map_err(|error| {
        tr!(
            "Не удалось включить микрофон: {error}",
            "Could not start microphone: {error}"
        )
    })?;
    let mut resampler = recognition::Resampler::new(rate);
    stream.play().map_err(|error| error.to_string())?;
    let _ = events.send(SpeechEvent::Listening);
    let mut current_context = context.load(Ordering::Relaxed);
    let mut previous_text = String::new();
    while !cancel.load(Ordering::Relaxed) {
        if let Ok(error) = error_receiver.try_recv() {
            return Err(tr!(
                "Ошибка микрофона: {error}",
                "Microphone error: {error}"
            ));
        }
        let next_context = context.load(Ordering::Relaxed);
        if next_context != current_context {
            current_context = next_context;
            recognizer.reset();
            previous_text.clear();
            resampler.reset();
            while receiver.try_recv().is_ok() {}
        }
        let audio = match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(audio) => audio,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(tr!("Микрофон отключён", "Microphone disconnected"));
            }
        };
        for chunk in std::iter::once(audio).chain(receiver.try_iter().take(127)) {
            recognizer.accept(&resampler.resample(&chunk));
        }
        if let Some(text) = recognizer
            .decode()
            .filter(|text| !text.trim().is_empty() && *text != previous_text)
        {
            if !cancel.load(Ordering::Relaxed) && context.load(Ordering::Relaxed) == current_context
            {
                previous_text.clone_from(&text);
                events
                    .send(SpeechEvent::Text {
                        text,
                        context: current_context,
                    })
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(())
}
