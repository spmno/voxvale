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
use gpui_kit::component::group_box::GroupBox;
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{
    ActiveTheme, Disableable, Icon, IconName, IndexPath, Sizable, TitleBar, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const LANGUAGES: [&str; 3] = ["chinese", "auto", "english"];
const SPEED_MIN: f32 = 0.7;
const SPEED_MAX: f32 = 1.3;

const VOICE_PRESETS: [(&str, &str); 4] = [
    (
        "温柔女童",
        "6~8岁女童，声音清亮柔和，自然稚嫩，带有自然呼吸与气息声，无机械AI感，适合绘本故事朗读",
    ),
    (
        "活泼男孩",
        "7~9岁男孩，声音明亮有活力，吐字清晰，语速适中，自然生动，适合课文朗读",
    ),
    (
        "知性女声",
        "30岁左右女性，声音温润知性，吐字清晰，语速平稳，适合科普讲解与新闻播报",
    ),
    (
        "沉稳男声",
        "35岁左右男性，声音低沉沉稳，字正腔圆，从容大气，适合纪录片解说",
    ),
];

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
                .placeholder("描述想要的声音，或点击上方预设快速填充")
        });
        let text = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(5)
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
                this.speed_val = v.start();
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

        cx.subscribe_in(
            &download_source,
            window,
            |this, _, ev: &SelectEvent<Vec<String>>, _, cx| {
                if let SelectEvent::Confirm(Some(v)) = ev {
                    this.download_source_val = v.clone();
                    log::info!("下载源切换为 {}", this.download_source_val);
                    cx.notify();
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &clone_model,
            window,
            |this, _, ev: &SelectEvent<Vec<String>>, _, cx| {
                if let SelectEvent::Confirm(Some(v)) = ev {
                    this.clone_model_val = v.clone();
                    log::info!("克隆模型切换为 {}", this.clone_model_val);
                    cx.notify();
                }
            },
        )
        .detach();

        let (req_tx, ev_rx) = spawn_worker();
        let ev_rx = Arc::new(Mutex::new(ev_rx));

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(120))
                    .await;
                if this.update(cx, |view, cx| view.drain_events(cx)).is_err() {
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
                    self.status = "合成中…（本地推理，速度取决于机器性能）".to_string();
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
            log::info!("开始预览播放：{} 样本 @{}Hz", samples.len(), sr);
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
        log::info!(
            "导出 MP3 → {}（默认名 {default_name}.mp3，来自朗读文本前 20 字）",
            path.display()
        );
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

    fn section_label(&self, cx: &Context<Self>, icon: IconName, label: &'static str) -> Div {
        h_flex()
            .gap_1p5()
            .items_center()
            .child(
                Icon::new(icon)
                    .small()
                    .text_color(cx.theme().primary),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child(label.to_string()),
            )
    }

    fn field(&self, cx: &Context<Self>, label: &'static str, el: impl IntoElement) -> Div {
        v_flex()
            .gap_1()
            .flex_1()
            .min_w_0()
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(label.to_string()),
            )
            .child(el)
    }

    fn status_badge(&self, cx: &Context<Self>) -> Div {
        let (icon, color) = if self.busy {
            (None, cx.theme().muted_foreground)
        } else if self.status.starts_with("失败") {
            (Some(IconName::CircleX), cx.theme().danger)
        } else if self.status.starts_with("合成完成") || self.status.starts_with("已导出") {
            (Some(IconName::CircleCheck), cx.theme().success)
        } else {
            (Some(IconName::Info), cx.theme().muted_foreground)
        };

        h_flex()
            .gap_2()
            .items_center()
            .min_w_0()
            .flex_1()
            .child(match icon {
                Some(name) => Icon::new(name).small().text_color(color).into_any_element(),
                None => Spinner::new().small().into_any_element(),
            })
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(color)
                    .truncate()
                    .child(self.status.clone()),
            )
    }
}

impl Render for VoxValeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let border = cx.theme().border;
        let design_mode = self.mode == TtsMode::Design;
        let busy = self.busy;
        let has_result = self.result.is_some();
        let playing = self.player.is_playing();
        let text_chars = self.text.read(cx).value().chars().count();
        let dim = |el: Div, active: bool| {
            if active {
                el
            } else {
                el.opacity(0.4)
            }
        };

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                TitleBar::new().child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("声谷 VoxVale"),
                        )
                        .child(div().text_size(px(11.0)).text_color(muted).child("v0.1.0")),
                ),
            )
            .child(
                div()
                    .id("content")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .p_4()
                    .gap_3()
                    .overflow_y_scroll()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("mode-design")
                                    .icon(Icon::new(IconName::Plus))
                                    .label("音色设计")
                                    .when(design_mode, |b| b.primary())
                                    .flex_1()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        log::info!("切换到音色设计模式");
                                        this.mode = TtsMode::Design;
                                        this.status = "已切换到音色设计模式".to_string();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("mode-clone")
                                    .icon(Icon::new(IconName::Copy))
                                    .label("语音克隆")
                                    .when(!design_mode, |b| b.primary())
                                    .flex_1()
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
                            GroupBox::new()
                                .title(
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            self.section_label(cx, IconName::User, "音色描述"),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_1()
                                                .items_center()
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(muted)
                                                        .child("快速预设："),
                                                )
                                                .children(
                                                    VOICE_PRESETS.iter().enumerate().map(
                                                        |(i, (name, _))| {
                                                            Button::new(("preset", i))
                                                                .label(*name)
                                                                .small()
                                                                .ghost()
                                                                .on_click(cx.listener(
                                                                    move |this, _, window, cx| {
                                                                        let text =
                                                                            VOICE_PRESETS[i].1;
                                                                        log::info!(
                                                                            "应用音色预设：{name}"
                                                                        );
                                                                        this.voice_desc.update(
                                                                            cx,
                                                                            |state, cx| {
                                                                                state.set_value(
                                                                                    text,
                                                                                    window,
                                                                                    cx,
                                                                                )
                                                                            },
                                                                        );
                                                                        cx.notify();
                                                                    },
                                                                ))
                                                        },
                                                    ),
                                                ),
                                        ),
                                )
                                .child(div().w_full().child(Textarea::new(&self.voice_desc))),
                        )
                    })
                    .when(!design_mode, |d| {
                        d.child(
                            GroupBox::new()
                                .title(self.section_label(cx, IconName::Copy, "参考音频"))
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            Button::new("pick-ref")
                                                .icon(Icon::new(IconName::FolderOpen))
                                                .label("选择文件")
                                                .secondary()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.pick_ref_audio(cx);
                                                })),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(13.0))
                                                .text_color(muted)
                                                .truncate()
                                                .child(if self.ref_name.is_empty() {
                                                    "未选择（支持 wav / mp3 / flac / ogg）"
                                                        .to_string()
                                                } else {
                                                    self.ref_name.clone()
                                                }),
                                        ),
                                )
                                .child(
                                    self.field(
                                        cx,
                                        "克隆模型",
                                        div()
                                            .w_full()
                                            .max_w(px(280.0))
                                            .child(Select::new(&self.clone_model)),
                                    ),
                                ),
                        )
                    })
                    .child(
                        GroupBox::new()
                            .title(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        self.section_label(cx, IconName::BookOpen, "朗读文本"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(muted)
                                            .child(format!("{text_chars} 字")),
                                    ),
                            )
                            .child(div().w_full().child(Textarea::new(&self.text))),
                    )
                    .child(
                        GroupBox::new()
                            .title(
                                self.section_label(cx, IconName::Settings2, "参数"),
                            )
                            .child(
                                h_flex()
                                    .gap_3()
                                    .w_full()
                                    .child(
                                        self.field(
                                            cx,
                                            "语速",
                                            h_flex()
                                                .w_full()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .child(Slider::new(&self.speed)),
                                                )
                                                .child(
                                                    div()
                                                        .w(px(52.0))
                                                        .text_size(px(12.0))
                                                        .child(format!(
                                                            "×{:.2}",
                                                            self.speed_val
                                                        )),
                                                ),
                                        ),
                                    )
                                    .child(dim(
                                        self.field(
                                            cx,
                                            "情绪",
                                            div().w_full().child(Select::new(&self.emotion)),
                                        ),
                                        design_mode,
                                    )),
                            )
                            .child(
                                h_flex()
                                    .gap_3()
                                    .w_full()
                                    .child(dim(
                                        self.field(
                                            cx,
                                            "停顿",
                                            div().w_full().child(Select::new(&self.pause)),
                                        ),
                                        design_mode,
                                    ))
                                    .child(self.field(
                                        cx,
                                        "语言",
                                        div().w_full().child(Select::new(&self.language)),
                                    ))
                                    .child(self.field(
                                        cx,
                                        "下载源",
                                        div().w_full().child(Select::new(&self.download_source)),
                                    )),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("generate")
                                    .when(busy, |b| b.icon(Spinner::new()))
                                    .label(if busy { "合成中…" } else { "生成语音" })
                                    .primary()
                                    .disabled(busy)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.start_generate(cx);
                                    })),
                            )
                            .child(
                                Button::new("preview")
                                    .icon(Icon::new(if playing {
                                        IconName::Square
                                    } else {
                                        IconName::Play
                                    }))
                                    .label(if playing { "停止" } else { "预览" })
                                    .secondary()
                                    .disabled(busy || !has_result)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_preview(cx);
                                    })),
                            )
                            .child(
                                Button::new("export")
                                    .icon(Icon::new(IconName::File))
                                    .label("导出 MP3")
                                    .secondary()
                                    .disabled(busy || !has_result)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.export(cx);
                                    })),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .px_4()
                    .py_2()
                    .border_t_1()
                    .border_color(border)
                    .child(self.status_badge(cx))
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                                                        .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(muted)
                                    .child("本地推理 · 数据不出设备"),
                            ),
                    ),
            )
    }
}
