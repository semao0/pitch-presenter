<p align="center">
  <img src="docs/logo.svg" width="88" alt="">
</p>

<h1 align="center">Pitch Presenter</h1>

<p align="center">
  A presenter view that stays out of your way.<br>
  Slides, notes and time for you. The audience sees only the slides.
</p>

<p align="center">
  <a href="https://github.com/semao0/pitch-presenter/releases/latest"><b>Download</b></a>
  &nbsp;·&nbsp; macOS &nbsp;·&nbsp; Windows &nbsp;·&nbsp; Linux
  &nbsp;·&nbsp; <a href="README.md">Русский</a>
</p>

<p align="center">
  <img src="docs/demo.gif" alt="Presenter view: current and next slide, speaker notes and timer">
</p>

## Why

**Your text in front of you.** Large, with key words highlighted and remarks to yourself. Edit the
file and the notes update right during rehearsal.

**Time under control.** Not just a stopwatch: every slide has its own limit, and the header shows
whether you are ahead of or behind the plan. Without limits, time is split by the amount of text.

**The audience sees only the slides.** Plug in a projector and the audience window opens on it in
fullscreen by itself. Navigate from either window, with clicks or a clicker.

PDF opens directly; PowerPoint, Keynote and ODP go through LibreOffice. Light and dark themes,
English and Russian interface.

<p align="center">
  <img src="docs/dark.png" alt="Dark theme">
</p>

## Follow speech

Click **Follow speech** above the notes to enable the default microphone. Spoken words fade,
the current position is underlined, and longer notes scroll automatically. Tracking resets for
each slide. Cues marked with `>` are excluded. Words are compared fuzzily, so wrong endings and
skipped words do not throw it off.

**Auto-advance** (the switch next to the button, or `L`; off by default): once you have read a
slide's text, the next slide opens after 1.2 s, `Esc` stays. It fires only when the cursor
walked the text rather than jumped to its end, and the slide has been up for at least 4 s.
Slides with no text or just a couple of words, and the last slide, never advance.

On first use GigaAM-v3 and Silero VAD are downloaded (about 275 MB) and verified with SHA-256.
Recognition then works offline: audio is never saved or sent to a server. The model stays in
memory only while following is on. On macOS, allow microphone access when prompted.

Russian speech only for now. Noise, improvisation and English terms may cause misses; turn
following off for long digressions.

### The model, and why

| | |
|---|---|
| Model | [GigaAM-v3](https://huggingface.co/ai-sage/GigaAM-v3) by SberDevices, `e2e_ctc` variant: 220M parameters, punctuated text. MIT |
| File | GGUF Q8_0 from [handy-computer/gigaam-v3-e2e-ctc-gguf](https://huggingface.co/handy-computer/gigaam-v3-e2e-ctc-gguf), pinned revision. WER on FLEURS-ru 5.53% vs 5.50% for F32 |
| Engine | [transcribe.cpp](https://github.com/handy-computer/transcribe.cpp) (ggml): Metal on Mac, CPU on Windows and Linux. Silero VAD from the same library |

Before 2.2, following ran on sherpa-onnx (int8, CPU). Measured on a MacBook Air M2, 8 GB, on 29 s
of Russian speech:

| Audio window | sherpa-onnx int8, CPU | transcribe.cpp Q8_0, Metal |
|---|---:|---:|
| 2 s | 119 ms | 30 ms |
| 3 s | 176 ms | 41 ms |
| 6 s | 338 ms | 66 ms |
| Process memory | 278 MB | 48 MB |

So it now listens to the last 3 s, and up to 6 s every 1.5 s. The first model load on a Mac
takes ~17 s while Metal shaders compile; after that it is under a second.
[GigaAM-Multilingual](https://huggingface.co/ai-sage/GigaAM-Multilingual) is not supported by
transcribe.cpp, and through PyTorch it is no more accurate on Russian and several times heavier.

Model authors: Kutsakov et al., [GigaAM: Efficient Self-Supervised Learner for Speech
Recognition](https://arxiv.org/abs/2506.01192), InterSpeech 2025. Licenses of the model and libraries
are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), which ships with every build.

## Speaker notes

Put `talk.toml` next to `talk.pdf` and it is picked up automatically.

```toml
minutes = 5                 # talk length

[[slide]]
n = 1
title = "The pain"
time = "0:30"               # limit for this slide
text = '''
Is what happens on site what **should** happen according to the plan?

> pause — a remark to yourself
'''
```

`**bold**` · `*italic*` · `==highlight==` · `- lists` · `> remarks` · `reserve = true` — a slide you
may skip. Full example: [example.toml](example.toml).

## Keys

| | |
|---|---|
| `→` `Space` · click | next |
| `←` · right click | back |
| `12` `Enter` | go to slide 12 |
| `G` | all slides |
| `B` | black screen |
| `T` · `Shift R` | timer: pause · reset |
| `L` | auto-advance |
| `+` `−` | text size |
| `A` | audience window |
| `?` | all shortcuts |

## Install

| | |
|---|---|
| **macOS** | `.dmg` for Apple Silicon. On first launch on macOS 14: right click → Open; on macOS 15: System Settings → Privacy & Security → Open Anyway |
| **Windows** | `.exe` or `.msi` |
| **Linux** | `.deb`, `.rpm` or `.AppImage` for x64 and ARM64 |

Everything is in [Releases](https://github.com/semao0/pitch-presenter/releases/latest).

## What stays on disk

Settings, the speech model and the slide cache live in the app's directories
(`dev.semao0.PitchPresenter`). Unfinished downloads and models from earlier versions are
removed when following is turned on. To remove everything on macOS:

```sh
rm -rf ~/Library/{Application\ Support,Caches,WebKit}/dev.semao0.PitchPresenter ~/Library/Preferences/dev.semao0.PitchPresenter.plist
```
