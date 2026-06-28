# Seraph

A desktop chiptune tracker / DAW for the **Sega Genesis / Mega Drive** sound hardware. Compose music using cycle-accurate emulation of the console's actual sound chips, then export it to a format that plays back on real hardware (or in a ROM).

Built with **Tauri v2** (Rust core) and **React 19 + TypeScript** (UI).

## What it does

- **Real-time playback** through emulation of the **Yamaha YM2612** FM synth chip (via the [Nuked-OPN2](https://github.com/nukeykt/Nuked-OPN2) core) and the **SN76489** PSG (pure-Rust emulator). The sequencer runs inside the audio thread for sample-accurate timing.
- **Three instrument types** matching the hardware: **FM** (6 channels), **PSG** (square + noise), and **DAC** (PCM samples) — each with a dedicated editor (FM operator/algorithm knobs, drawable PSG envelopes, DAC waveform import).
- **Arrangement view + piano roll** — organize tracks and regions on a timeline, edit notes per-region, with per-note velocity, pan, detune, and modulation.
- **Live spectrum analyzer + VU meters** for visual feedback while playing.

## Import / Export

| Direction | Formats |
|---|---|
| **Import** | SMPS (Sega's sound-driver assembly), VGM, common FM instrument banks, Zyrinx; bundled PSG envelope tables and a Universal Voice Bank |
| **Export** | **Flamedriver SMPS assembly** (`.asm`, assembles straight into a ROM), VGM |

This makes Seraph a companion to the ROM-hacking projects in the parent workspace: compose here, export SMPS, drop it into the Z80 Flamedriver sound driver.

## Tech stack

- **Core:** Rust, [cpal](https://github.com/RustAudio/cpal) audio output, [rtrb](https://github.com/mgeier/rtrb) lock-free ring buffer
- **Sound chips:** Nuked-OPN2 (YM2612, vendored C + FFI), custom Rust SN76489
- **UI:** React 19, TypeScript 5.8, HTML5 Canvas rendering, CSS Modules
- **Shell:** Tauri v2 with IPC commands bridging UI ↔ Rust core

## Development

Prerequisites: a [Rust toolchain](https://rustup.rs/), Node.js, and the [Tauri v2 system dependencies](https://v2.tauri.app/start/prerequisites/) for your platform.

```bash
npm install            # install frontend dependencies
npm run tauri dev      # run the app (Vite dev server + Tauri window)
npm run tauri build    # produce a release build
```

Frontend-only commands:

```bash
npm run dev            # Vite dev server (no Tauri window)
npm run build          # type-check + build frontend
npm run gen:theme      # regenerate CSS theme tokens from tokens.json
```

## Project layout

```
src/                       React + TypeScript UI
  components/              arrangement, piano roll, instrument editors, spectrum analyzer
  api/ipc.ts              typed wrappers over Tauri commands
  theme/                  design tokens + generated CSS
src-tauri/src/            Rust core
  audio/                  real-time engine, mixing, spectrum analysis
  ym2612/                 YM2612 wrapper over Nuked-OPN2
  dac/                    PCM import + playback pipeline
  sequencer/              tick-based sequencer + snapshots
  driver/                 Flamedriver SMPS profile (voice packing)
  import/  export/        SMPS / VGM / FM format parsers and writers
  model/  project/        data model + project file management
  vendor/nuked-opn2/      vendored Nuked-OPN2 C source
docs/                     design specs and implementation plans
```

## Status

Early development (`0.1.0`). The audio engine, instrument editors, sequencer, and SMPS/VGM import-export are functional; expect rough edges.
