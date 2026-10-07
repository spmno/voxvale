# VoxVale

> **English** | [简体中文](README.zh-CN.md)

Local-first voice generation desktop app — describe a voice, input text, export MP3.

Built on GPUI + Candle. Qwen3-TTS model runs **entirely on-device — your data never leaves your machine**.

## Features

- **Voice Design**: Generate a brand-new voice from a pure text description. Built-in children's voice presets (gentle girl / lively boy, etc.), optimized for picture-book and textbook narration — natural breathiness, no robotic AI artifacts
- **Voice Cloning**: Load a local reference audio (wav / mp3 / flac / ogg) and read new text in the same voice; choose between 0.6B (faster) or 1.7B (higher quality) models
- **Parameter Control**: Speed 0.7–1.3 (pitch-preserving time-stretch in cloning mode), emotion, pause, language
- **Preview & Export**: Instant audition, one-click MP3 export (filename auto-derived from the first 20 characters of the text)
- **Model Management**: Auto-download on first use (hf-mirror.com / huggingface.co dual-source), resume support, per-file progress display
- **Full-Chain Logging**: Every step is traceable; adjust log level via `RUST_LOG`

## Privacy

All inference runs locally. The app makes no network requests except for the initial model weight download.

## Performance Reference

| Environment | RTF* |
|---|---|
| AMD Ryzen 9 9950X (CPU, release build) | ~4.4x |
| AMD Radeon Pro W7900 via ZLUDA (experimental) | ~2.7x |

\* RTF = synthesis time / audio duration; lower is faster. 1 minute of audio takes ~4.4 minutes to synthesize on CPU; GPU acceleration gains depend on workload characteristics (see "Known Limitations").

## Build Requirements

- Rust 1.85+ (edition 2024)
- Linux: `libasound2-dev` (ALSA headers) and `pkg-config`
- Windows / macOS: no additional system dependencies

## Build & Run

```bash
git clone https://github.com/spmno/voxvale.git
cd voxvale
cargo build --release
./target/release/voxvale
```

On first use: click "Generate Speech" and the app will auto-download the model from a mirror (~4.2GB, one-time only), cached in `~/.cache/voxvale/models`.

## GPU Acceleration (Optional)

| Platform | Command | Notes |
|---|---|---|
| macOS Apple Silicon | `cargo build --release --features metal` | Native Metal, recommended |
| Windows / Linux + NVIDIA | `cargo build --release --features cuda` | Native CUDA, recommended |
| Linux + AMD (experimental) | `source scripts/env-cuda.sh` then same as above; at runtime set `LD_LIBRARY_PATH=$HOME/zluda/zluda:$HOME/cuda-12.6/lib64:/opt/rocm/lib` | ZLUDA compatibility layer; script includes full environment setup instructions |

> Note: A Candle ROCm community fork also exists for Linux + AMD (PR #3424); switch once its convolution kernels mature.

## Usage

1. Toggle "Voice Design / Voice Cloning" at the top
2. Voice Design: describe the desired voice (or click a preset to auto-fill); Voice Cloning: select a local reference audio
3. Enter narration text and adjust speed / emotion / pause / language as needed
4. "Generate Speech" → "Preview" to audition → "Export MP3"

## Logging

```bash
RUST_LOG=voxvale=debug ./target/release/voxvale
```

Logs cover: mode switching, task submission parameters, model download progress, weight loading time, pure synthesis time (separate from model preparation), WSOLA time-stretch, preview / export, and full error chain on failure.

## Known Limitations

- Candle has no ROCm backend yet; Linux + AMD GPUs fall back to CPU by default (ZLUDA is experimental)
- Pure CPU inference runs at offline batch speed (RTF ~4.4x), not real-time streaming
- Auto-download requires network; fully offline scenarios require manually placing the model directory

## Acknowledgements

- [Qwen3-TTS](https://huggingface.co/Qwen3-TTS) (Alibaba Tongyi) · [Candle](https://github.com/huggingface/candle) · [qwen_tts](https://github.com/danielclough/qwen3-tts-rs)
- [gpui-component](https://github.com/longbridge/gpui-component) (GPUI Kit) · [ZLUDA](https://github.com/vosen/ZLUDA) · [Shine](https://github.com/toots/shine) · [Lucide](https://lucide.dev) icons
