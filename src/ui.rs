use crate::audio::decode::decode_audio_file;
use crate::audio::export::{export_mp3, mp3_default_name};
use crate::audio::playback::AudioPreview;
use crate::tts::engine::{
    CLONE_MODEL_LABELS, SOURCE_LABELS, CloneModel, DownloadSource, EngineEvent, SynthRequest,
    TtsMode, fmt_bytes, spawn_worker,
};
use crate::tts::instruct::{
    EMOTION_LABELS, Emotion, PAUSE_LABELS, PauseStyle, VoiceParams, build_voice_instruction,
};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::component::{Disableable, IndexPath};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const LANGUAGES: [&str; 3] = ["chinese", "auto", "english"];
const SPEED_MIN: f32 = 0.7;
const SPEED_MAX: f32 = 1.3;

pub struct VoxValeView {
    mode: TtsMode,
    voice_desc: Entity<TextareaState>,
    text: Entity<TextareaState>,
    ref_path: Option<PathBuf>,
    ref_name: String,
    speed: Entity<SliderState>,
    speed_val: f32,
    emotion: Entity<SelectState<Vec<String>>>,
    emotion_val: String,
    pause: Entity<SelectState<Vec<String>>>,
    pause_val: String,
    language: Entity<SelectState<Vec<String>>>,
    language_val: String,
    download_source: Entity<SelectState<Vec<String>>>,
    download_source_val: String,
    clone_model: Entity<SelectState<Vec<String>>>,
    clone_model_val: String,
    busy: bool,
    status: String,
    result: Option<Arc<Vec<f32>>>,
    result_sr: u32,
    result_text: String,
    player: AudioPreview,
    req_tx: Sender<SynthRequest>,
    ev_rx: Arc<Mutex<Receiver<EngineEvent>>>,
}

impl VoxValeView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let voice_desc = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(3)
                .placeholder("描述想要的声音，如：6~8岁女童，声音清亮柔和，自然无机械感")
        });
        let text = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(6)
                .placeholder("输入要朗读的文本…")
        });

        let speed = cx.new(|_| {
            SliderState::new()
                .min(SPEED_MIN)
                .max(SPEED_MAX)
                .default_value(1.0)
                .step(0.05)
        });

        let emotion = cx.new(|cx| {
            SelectState::new(
                EMOTION_LABELS.iter().map(|s| s.to_string()).collect(),
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let pause = cx.new(|cx| {
            SelectState::new(
                PAUSE_LABELS.iter().map(|s| s.to_string()).collect(),
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let language = cx.new(|cx| {
            SelectState::new(
                LANGUAGES.iter().map(|s| s.to_string()).collect(),
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let download_source = cx.new(|cx| {
            SelectState::new(
                SOURCE_LABELS.iter().map(|s| s.to_string()).collect(),
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let clone_model = cx.new(|cx| {
            SelectState::new(
                CLONE_MODEL_LABELS.iter().map(|s| s.to_string()).collect(),
                Some(IndexPath::default()),
                window,
                cx,
            )
        });

        cx.subscribe_in(&speed, window, |this, _, ev: &SliderEvent, _, cx| {
            if let SliderEvent::Change(v) = ev {
                this.speed_val = v.start() as f32;
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&emotion, window, |this, _, ev: &SelectEvent<Vec<String>>, _, cx| {
            if let SelectEvent::Confirm(Some(v)) = ev {
                this.emotion_val = v.clone();
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&pause, window, |this, _, ev: &SelectEvent<Vec<String>>, _, cx| {
            if let SelectEvent::Confirm(Some(v)) = ev {
                this.pause_val = v.clone();
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&language, window, |this, _, ev: &SelectEvent<Vec<String>>, _, cx| {
            if let SelectEvent::Confirm(Some(v)) = ev {
                this.language_val = v.clone();
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&download_source, window, |this, _, ev: &SelectEvent<Vec<String>>, _, cx| {
            if let SelectEvent::Confirm(Some(v)) = ev {
                this.download_source_val = v.clone();
                log::info!("下载源切换为 {}", this.download_source_val);
                cx.notify();
            }
        })
        .detach();

        cx.subscribe_in(&clone_model, window, |this, _, ev: &SelectEvent<Vec<String>>, _, cx| {
            if let SelectEvent::Confirm(Some(v)) = ev {
                this.clone_model_val = v.clone();
                log::info!("克隆模型切换为 {}", this.clone_model_val);
                cx.notify();
            }
        })
        .detach();

        let (req_tx, ev_rx) = spawn_worker();
        let ev_rx = Arc::new(Mutex::new(ev_rx));

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(120))
                    .await;
                if this
                    .update(cx, |view, cx| view.drain_events(cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        let player = AudioPreview::new().unwrap_or_else(|_| {
            eprintln!("音频输出设备初始化失败，预览不可用");
            AudioPreview::dummy()
        });

        Self {
            mode: TtsMode::Design,
            voice_desc,
            text,
            ref_path: None,
            ref_name: String::new(),
            speed,
            speed_val: 1.0,
            emotion,
            emotion_val: EMOTION_LABELS[0].to_string(),
            pause,
            pause_val: PAUSE_LABELS[0].to_string(),
            language,
            language_val: LANGUAGES[0].to_string(),
            download_source,
            download_source_val: SOURCE_LABELS[0].to_string(),
            clone_model,
            clone_model_val: CLONE_MODEL_LABELS[0].to_string(),
            busy: false,
            status: "输入文本，点击「生成语音」开始".to_string(),
            result: None,
            result_sr: 24000,
            result_text: String::new(),
            player,
            req_tx,
            ev_rx,
        }
    }

    fn drain_events(&mut self, cx: &mut Context<Self>) {
        let rx = self.ev_rx.clone();
        let mut changed = false;
        while let Ok(ev) = rx.lock().unwrap().try_recv() {
            changed = true;
            match ev {
                EngineEvent::DownloadProgress {
                    file,
                    file_index,
                    total_files,
                    bytes,
                    total_bytes,
                } => {
                    self.busy = true;
                    self.status = if total_bytes > 0 {
                        format!(
                            "下载模型 {file_index}/{total_files} {file}：{} / {}（{:.0}%）",
                            fmt_bytes(bytes),
                            fmt_bytes(total_bytes),
                            bytes as f64 / total_bytes as f64 * 100.0
                        )
                    } else {
                        format!(
                            "下载模型 {file_index}/{total_files} {file}：{}",
                            fmt_bytes(bytes)
                        )
                    };
                }
                EngineEvent::ModelLoading { repo_id } => {
                    self.busy = true;
                    self.status = format!("模型文件就绪，加载权重到内存（{repo_id}）…");
                }
                EngineEvent::Generating => {
                    self.busy = true;
                    self.status = "合成中…（本地 CPU 推理，语速取决于机器性能）".to_string();
                }
                EngineEvent::Done {
                    samples,
                    sample_rate,
                } => {
                    self.busy = false;
                    let secs = samples.len() as f32 / sample_rate as f32;
                    self.status = format!("合成完成，时长 {secs:.1} 秒，可预览或导出 MP3");
                    self.result = Some(Arc::new(samples));
                    self.result_sr = sample_rate;
                }
                EngineEvent::Failed { message } => {
                    self.busy = false;
                    self.status = format!("失败：{message}");
                }
            }
        }
        if changed {
            cx.notify();
        }
    }

    fn start_generate(&mut self, cx: &mut Context<Self>) {
        let text = self.text.read(cx).value().trim().to_string();
        if text.is_empty() {
            self.status = "请先输入朗读文本".to_string();
            cx.notify();
            return;
        }
        self.result_text = text.clone();
        if self.mode == TtsMode::Clone && self.ref_path.is_none() {
            self.status = "语音克隆需要先选择参考音频".to_string();
            cx.notify();
            return;
        }

        let language = self.language_val.clone();
        let source = DownloadSource::from_label(&self.download_source_val);
        let req = match self.mode {
            TtsMode::Design => {
                let desc = self.voice_desc.read(cx).value().to_string();
                let params = VoiceParams {
                    speed: self.speed_val,
                    emotion: Emotion::from_label(&self.emotion_val),
                    pause: PauseStyle::from_label(&self.pause_val),
                };
                let instruction = build_voice_instruction(&desc, &params);
                log::info!(
                    "提交合成[音色设计] 语速×{:.2} 情绪={} 停顿={} 语言={language} 下载源={}",
                    params.speed,
                    self.emotion_val,
                    self.pause_val,
                    source.label()
                );
                SynthRequest::Design {
                    text,
                    instruction,
                    language,
                    source,
                }
            }
            TtsMode::Clone => {
                let path = self.ref_path.clone().unwrap();
                let clone_model = CloneModel::from_label(&self.clone_model_val);
                log::info!(
                    "提交合成[语音克隆] 模型={} 语速×{:.2} 语言={language} 下载源={} 参考={}",
                    clone_model.repo_id(),
                    self.speed_val,
                    source.label(),
                    path.display()
                );
                match decode_audio_file(&path) {
                    Ok((pcm, sr)) => SynthRequest::Clone {
                        text,
                        ref_pcm: pcm,
                        ref_sample_rate: sr,
                        speed: self.speed_val,
                        language,
                        source,
                        model: clone_model,
                    },
                    Err(e) => {
                        log::error!("参考音频解码失败 {path:?}：{e:#}");
                        self.status = format!("参考音频解码失败：{e:#}");
                        cx.notify();
                        return;
                    }
                }
            }
        };

        self.busy = true;
        self.status = "已提交合成任务…".to_string();
        if self.req_tx.send(req).is_err() {
            self.busy = false;
            self.status = "合成线程已退出，请重启应用".to_string();
        }
        cx.notify();
    }

    fn toggle_preview(&mut self, _cx: &mut Context<Self>) {
        if self.player.is_playing() {
            log::info!("预览停止");
            self.player.stop();
            return;
        }
        if let Some(samples) = self.result.clone() {
            let sr = self.result_sr;
            log::info!(
                "开始预览播放：{} 样本 @{}Hz",
                samples.len(),
                sr
            );
            if let Err(e) = self.player.play(samples, sr) {
                log::error!("预览播放失败：{e:#}");
                self.status = format!("预览失败：{e:#}");
            }
        }
    }

    fn export(&mut self, _cx: &mut Context<Self>) {
        let Some(samples) = self.result.clone() else {
            return;
        };
        let default_name = mp3_default_name(&self.result_text);
        let Some(path) = rfd::FileDialog::new()
            .add_filter("MP3 音频", &["mp3"])
            .set_file_name(&format!("{default_name}.mp3"))
            .save_file()
        else {
            return;
        };
        log::info!("导出 MP3 → {}（默认名 {default_name}.mp3，来自朗读文本前 20 字）", path.display());
        let t = Instant::now();
        match export_mp3(&samples, self.result_sr, &path) {
            Ok(()) => {
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                log::info!(
                    "导出完成 {}（{}），耗时 {:.1}s",
                    path.display(),
                    fmt_bytes(size),
                    t.elapsed().as_secs_f32()
                );
                self.status = format!("已导出：{}", path.display());
            }
            Err(e) => {
                log::error!("导出失败：{e:#}");
                self.status = format!("导出失败：{e:#}");
            }
        }
    }

    fn pick_ref_audio(&mut self, _cx: &mut Context<Self>) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("音频文件", &["wav", "mp3", "flac", "ogg", "m4a"])
            .pick_file()
        {
            log::info!("已选择参考音频：{}", path.display());
            self.ref_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            self.ref_path = Some(path);
        }
    }

    fn param_row_labelled(&self, label: &'static str, child: impl IntoElement) -> Div {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w(px(52.0))
                    .child(label.to_string()),
            )
            .child(child)
    }

    fn slider_label(&self, label: &'static str) -> Div {
        div().w(px(52.0)).child(label.to_string())
    }
}

impl Render for VoxValeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let gray = rgb(0x8a8a8a);
        let design_mode = self.mode == TtsMode::Design;

        let generate_label = if self.busy { "合成中…" } else { "生成语音" };
        let preview_label = if self.player.is_playing() {
            "■ 停止"
        } else {
            "▶ 预览"
        };
        let has_result = self.result.is_some();

        div()
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(20.0))
                            .font_weight(FontWeight::BOLD)
                            .child("声谷 VoxVale"),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(gray)
                            .child("本地语音生成 · 数据不出设备"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("mode-design")
                            .label("音色设计")
                            .when(design_mode, |b| b.primary())
                            .on_click(cx.listener(|this, _, _, cx| {
                                log::info!("切换到音色设计模式");
                                this.mode = TtsMode::Design;
                                this.status = "已切换到音色设计模式".to_string();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("mode-clone")
                            .label("语音克隆")
                            .when(!design_mode, |b| b.primary())
                            .on_click(cx.listener(|this, _, _, cx| {
                                log::info!("切换到语音克隆模式");
                                this.mode = TtsMode::Clone;
                                this.status = "已切换到语音克隆模式".to_string();
                                cx.notify();
                            })),
                    ),
            )
            .when(design_mode, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(gray)
                                .child("音色描述"),
                        )
                        .child(Textarea::new(&self.voice_desc)),
                )
            })
            .when(!design_mode, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(gray)
                                .child("参考音频（保留其声线，仅用于克隆）"),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    Button::new("pick-ref")
                                        .label("选择文件")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.pick_ref_audio(cx);
                                        })),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .text_color(gray)
                                        .child(if self.ref_name.is_empty() {
                                            "未选择（支持 wav / mp3 / flac）".to_string()
                                        } else {
                                            self.ref_name.clone()
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .w(px(52.0))
                                        .text_size(px(13.0))
                                        .text_color(gray)
                                        .child("克隆模型"),
                                )
                                .child(
                                    div()
                                        .w(px(200.0))
                                        .child(Select::new(&self.clone_model)),
                                ),
                        ),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(gray)
                            .child("朗读文本"),
                    )
                    .child(Textarea::new(&self.text)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(self.slider_label("语速"))
                            .child(div().flex_1().child(Slider::new(&self.speed)))
                            .child(
                                div()
                                    .w(px(64.0))
                                    .text_size(px(13.0))
                                    .child(format!("×{:.2}", self.speed_val)),
                            ),
                    )
                    .child(self.param_row_labelled(
                        "情绪",
                        div()
                            .w(px(150.0))
                            .when(!design_mode, |d| d.opacity(0.4))
                            .child(Select::new(&self.emotion)),
                    ))
                    .child(self.param_row_labelled(
                        "停顿",
                        div()
                            .w(px(150.0))
                            .when(!design_mode, |d| d.opacity(0.4))
                            .child(Select::new(&self.pause)),
                    ))
                    .child(self.param_row_labelled(
                        "语言",
                        div().w(px(150.0)).child(Select::new(&self.language)),
                    ))
                    .child(self.param_row_labelled(
                        "下载源",
                        div().w(px(200.0)).child(Select::new(&self.download_source)),
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("generate")
                            .label(generate_label)
                            .primary()
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.start_generate(cx);
                            })),
                    )
                    .child(
                        Button::new("preview")
                            .label(preview_label)
                            .disabled(self.busy || !has_result)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_preview(cx);
                            })),
                    )
                    .child(
                        Button::new("export")
                            .label("导出 MP3")
                            .disabled(self.busy || !has_result)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.export(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(gray)
                    .child(self.status.clone()),
            )
    }
}
