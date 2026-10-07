# 声谷 VoxVale

> [English](README.md) | **简体中文**

本地优先的语音生成桌面应用 —— 描述声音、输入文本、导出 MP3。

基于 GPUI + Candle 构建，Qwen3-TTS 模型**全程在本机推理，数据不出设备**。

## ✨ 功能

- **音色设计**：纯文字描述生成全新声线，内置儿童音色预设（温柔女童 / 活泼男孩等），专为绘本与课文朗读优化——自然气息、无机械 AI 感
- **语音克隆**：加载本地参考音频（wav / mp3 / flac / ogg），以同款声线朗读新文本；模型可选 0.6B（更快）/ 1.7B（更高质量）
- **参数调节**：语速 0.7~1.3（克隆模式保音高变速）、情绪、停顿、语言
- **预览与导出**：即时试听，一键导出 MP3（文件名自动取朗读文本前 20 字）
- **模型管理**：首次使用自动下载（hf-mirror.com / huggingface.co 双源可选），断点续传，逐文件进度上屏
- **全链路日志**：每一步操作可追溯，`RUST_LOG` 可调级别

## 🔒 隐私

所有推理在本机完成。除首次下载模型权重外，应用不产生任何网络请求。

## ⚡ 性能参考

| 环境 | RTF* |
|---|---|
| AMD Ryzen 9 9950X（CPU，release 构建） | ≈ 4.4x |
| AMD Radeon Pro W7900 via ZLUDA（实验性） | ≈ 2.7x |

\* RTF = 合成耗时 / 音频时长，越小越快。1 分钟音频在 CPU 上约需 4.4 分钟合成；GPU 加速收益取决于负载形态（详见「已知限制」）。

## 🛠 构建要求

- Rust 1.85+（edition 2024）
- Linux：`libasound2-dev`（ALSA 头文件）与 `pkg-config`
- Windows / macOS：无额外系统依赖

## 📦 构建与运行

```bash
git clone https://github.com/spmno/voxvale.git
cd voxvale
cargo build --release
./target/release/voxvale
```

首次使用：点击「生成语音」后应用自动从镜像下载模型（约 4.2GB，仅一次），缓存在 `~/.cache/voxvale/models`。

## 🚀 GPU 加速（可选）

| 平台 | 命令 | 说明 |
|---|---|---|
| macOS Apple Silicon | `cargo build --release --features metal` | 原生 Metal，推荐 |
| Windows / Linux + NVIDIA | `cargo build --release --features cuda` | 原生 CUDA，推荐 |
| Linux + AMD（实验性） | `source scripts/env-cuda.sh` 后同上，运行时 `LD_LIBRARY_PATH=$HOME/zluda/zluda:$HOME/cuda-12.6/lib64:/opt/rocm/lib` | ZLUDA 兼容层，脚本含完整环境搭建说明 |

> 注：Linux + AMD 也存在 Candle ROCm 社区分支方案（PR #3424），待其卷积内核成熟后可切换。

## 📖 使用

1. 顶部切换「音色设计 / 语音克隆」
2. 音色设计：描述想要的声音（或点击预设一键填充）；语音克隆：选择本地参考音频
3. 输入朗读文本，按需调节语速 / 情绪 / 停顿 / 语言
4. 「生成语音」→「预览」试听 →「导出 MP3」

## 📝 日志

```bash
RUST_LOG=voxvale=debug ./target/release/voxvale
```

日志覆盖：模式切换、任务提交参数、模型下载进度、权重加载耗时、纯合成耗时（与模型准备分开计）、WSOLA 变速、预览 / 导出，失败时输出完整错误链。

## ⚠️ 已知限制

- Candle 暂无 ROCm 后端，Linux + AMD 显卡默认走 CPU（ZLUDA 为实验性方案）
- 纯 CPU 推理为离线批处理速度（RTF ≈ 4.4x），非实时流式
- 自动下载依赖网络；完全离线场景需手动放置模型目录

## 🙏 致谢

- [Qwen3-TTS](https://huggingface.co/Qwen3-TTS)（阿里通义）· [Candle](https://github.com/huggingface/candle) · [qwen_tts](https://github.com/danielclough/qwen3-tts-rs)
- [gpui-component](https://github.com/longbridge/gpui-component)（GPUI Kit）· [ZLUDA](https://github.com/vosen/ZLUDA) · [Shine](https://github.com/toots/shine) · [Lucide](https://lucide.dev) 图标
