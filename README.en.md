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
the current position is underlined, and longer notes scroll automatically. Slides still advance
manually; tracking resets for each slide. Cues marked with `>` are excluded. Click the microphone
button to stop following.

On first use, GigaAM v3 CTC and Silero VAD are downloaded (about 230 MB) and verified with
SHA-256. Recognition then works offline: audio is never saved or sent to a server. Models live
in the application's data directory, separately from the installer. Closing the presentation
turns off the microphone. On macOS, allow microphone access when prompted.

Tracking updates about every 300 ms when the computer can keep up. It usually processes
the latest 2 seconds of audio, extending to 3 seconds every 1.5 seconds and at phrase endings
to recover context. The update interval grows automatically on slower computers. This interval
is not the highlighting latency: recognition time and matching several spoken words also matter.

Currently supports Russian speech only. This experimental feature can lag or miss passages
with noise, improvisation, numbers or English terms. Phrases shorter than three words do not
advance the cursor. Manual scrolling remains available; turn tracking off for long digressions.
It uses the open base model, not the private Glas weights fine-tuned for English loanwords.

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
| `+` `−` | text size |
| `A` | audience window |
| `?` | all shortcuts |

## Install

| | |
|---|---|
| **macOS** | `.dmg` for Apple Silicon and Intel. On first launch: right click → Open |
| **Windows** | `.exe` or `.msi` |
| **Linux** | `.deb`, `.rpm` or `.AppImage` for x64 and ARM64 |

Everything is in [Releases](https://github.com/semao0/pitch-presenter/releases/latest).
