use crate::audio::decode::decode_audio_file;
use crate::audio::export::{export_mp3, mp3_default_name};
use crate::audio::playback::AudioPreview;
use crate::i18n::{Language, get_language, set_language, t};
use crate::tts::engine::{
    CloneModel, DownloadSource, EngineEvent, SynthRequest, TtsMode, fmt_bytes, spawn_worker,
};
use crate::tts::instruct::{
    Emotion, PauseStyle, VoiceParams, build_voice_instruction,
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

fn voice_presets() -> [(&'static str, &'static str); 4] {
    match get_language() {
        Language::English => [
            (
                "Gentle Girl",
                "6-8 year old girl, clear and soft voice, naturally childlike, with natural breathiness, no robotic AI feel, suitable for picture book narration",
            ),
            (
                "Lively Boy",
                "7-9 year old boy, bright and energetic voice, clear articulation, moderate pace, natural and vivid, suitable for textbook narration",
            ),
            (
                "Intellectual Female",
                "~30 year old female, warm and intellectual voice, clear articulation, steady pace, suitable for science explanation and news broadcasting",
            ),
            (
                "Calm Male",
                "~35 year old male, deep and calm voice, precise articulation, composed and grand, suitable for documentary narration",
            ),
        ],
        Language::Chinese => [
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
        ],
    }
}

fn emotion_labels() -> Vec<String> {
    let lang = get_language();
    [
        Emotion::Natural,
        Emotion::Warm,
        Emotion::Cheerful,
        Emotion::Calm,
        Emotion::Sad,
        Emotion::Excited,
        Emotion::Serious,
    ]
    .iter()
    .map(|e| e.label(lang).to_string())
    .collect()
}

fn pause_labels() -> Vec<String> {
    let lang = get_language();
    [PauseStyle::Natural, PauseStyle::More, PauseStyle::Less]
        .iter()
        .map(|p| p.label(lang).to_string())
        .collect()
}

fn clone_model_labels() -> Vec<String> {
    let lang = get_language();
    [CloneModel::Base06B, CloneModel::Base17B]
        .iter()
        .map(|m| m.label(lang).to_string())
        .collect()
}

fn source_labels() -> Vec<String> {
    let lang = get_language();
    [DownloadSource::Mirror, DownloadSource::Official]
        .iter()
        .map(|s| s.display_label(lang).to_string())
        .collect()
}

pub struct VoxValeView {
    mode: TtsMode,
    lang: Language,
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
        let lang = get_language();
        let voice_desc = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(3)
                .placeholder(t("voice_desc_placeholder"))
        });
        let text = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(5)
                .placeholder(t("text_placeholder"))
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
                emotion_labels(),
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let pause = cx.new(|cx| {
            SelectState::new(
                pause_labels(),
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
        let default_source = match lang {
            Language::English => DownloadSource::Official,
            Language::Chinese => DownloadSource::Mirror,
        };
        let source_items = source_labels();
        let source_initial_row = source_items
            .iter()
            .position(|label| *label == default_source.display_label(lang))
            .unwrap_or(0);
        let download_source = cx.new(|cx| {
            SelectState::new(
                source_items,
                Some(IndexPath::new(source_initial_row)),
                window,
                cx,
            )
        });
        let clone_model = cx.new(|cx| {
            SelectState::new(
                clone_model_labels(),
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
                    log::info!("{}", t("log_source_switch").replace("{}", &v));
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
                    log::info!("{}", t("log_clone_model_switch").replace("{}", &v));
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
            eprintln!("{}", t("log_audio_init_failed"));
            AudioPreview::dummy()
        });

        Self {
            mode: TtsMode::Design,
            lang,
            voice_desc,
            text,
            ref_path: None,
            ref_name: String::new(),
            speed,
            speed_val: 1.0,
            emotion,
            emotion_val: Emotion::Natural.label(lang).to_string(),
            pause,
            pause_val: PauseStyle::Natural.label(lang).to_string(),
            language,
            language_val: LANGUAGES[0].to_string(),
            download_source,
            download_source_val: match lang {
                Language::English => DownloadSource::Official.display_label(lang).to_string(),
                Language::Chinese => DownloadSource::Mirror.display_label(lang).to_string(),
            },
            clone_model,
            clone_model_val: CloneModel::Base06B.label(lang).to_string(),
            busy: false,
            status: t("status_idle").to_string(),
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
                        t("downloading_model")
                            .replacen("{}", &file_index.to_string(), 1)
                            .replacen("{}", &total_files.to_string(), 1)
                            .replacen("{}", &file, 1)
                            .replacen("{}", &fmt_bytes(bytes), 1)
                            .replacen("{}", &fmt_bytes(total_bytes), 1)
                            .replace("{:.0}", &format!("{:.0}", bytes as f64 / total_bytes as f64 * 100.0))
                    } else {
                        t("downloading_model_no_total")
                            .replacen("{}", &file_index.to_string(), 1)
                            .replacen("{}", &total_files.to_string(), 1)
                            .replacen("{}", &file, 1)
                            .replacen("{}", &fmt_bytes(bytes), 1)
                    };
                }
                EngineEvent::ModelLoading { repo_id } => {
                    self.busy = true;
                    self.status = t("model_loading").replace("{}", &repo_id);
                }
                EngineEvent::Generating => {
                    self.busy = true;
                    self.status = t("status_generating").to_string();
                }
                EngineEvent::Done {
                    samples,
                    sample_rate,
                } => {
                    self.busy = false;
                    let secs = samples.len() as f32 / sample_rate as f32;
                    self.status = t("status_done").replace("{:.1}", &format!("{secs:.1}"));
                    self.result = Some(Arc::new(samples));
                    self.result_sr = sample_rate;
                }
                EngineEvent::Failed { message } => {
                    self.busy = false;
                    self.status = t("status_failed").replace("{}", &message);
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
            self.status = t("status_text_required").to_string();
            cx.notify();
            return;
        }
        self.result_text = text.clone();
        if self.mode == TtsMode::Clone && self.ref_path.is_none() {
            self.status = t("status_ref_required").to_string();
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
                    "{}",
                    t("log_synth_design")
                        .replace("{:.2}", &format!("{:.2}", params.speed))
                        .replacen("{}", &self.emotion_val, 1)
                        .replacen("{}", &self.pause_val, 1)
                        .replacen("{}", &language, 1)
                        .replacen("{}", &source.label(self.lang), 1)
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
                    "{}",
                    t("log_synth_clone")
                        .replacen("{}", &clone_model.repo_id(), 1)
                        .replace("{:.2}", &format!("{:.2}", self.speed_val))
                        .replacen("{}", &language, 1)
                        .replacen("{}", &source.label(self.lang), 1)
                        .replacen("{}", &path.display().to_string(), 1)
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
                        log::error!(
                            "{}",
                            t("log_decode_failed")
                                .replace("{:?}", &format!("{path:?}"))
                                .replace("{:#}", &format!("{e:#}"))
                        );
                        self.status = t("status_decode_failed")
                            .replace("{:#}", &format!("{e:#}"));
                        cx.notify();
                        return;
                    }
                }
            }
        };

        self.busy = true;
        self.status = t("status_submitted").to_string();
        if self.req_tx.send(req).is_err() {
            self.busy = false;
            self.status = t("status_worker_dead").to_string();
        }
        cx.notify();
    }

    fn toggle_preview(&mut self, _cx: &mut Context<Self>) {
        if self.player.is_playing() {
            log::info!("{}", t("log_preview_stop"));
            self.player.stop();
            return;
        }
        if let Some(samples) = self.result.clone() {
            let sr = self.result_sr;
            log::info!(
                "{}",
                t("log_preview_play")
                    .replacen("{}", &samples.len().to_string(), 1)
                    .replacen("{}", &sr.to_string(), 1)
            );
            if let Err(e) = self.player.play(samples, sr) {
                log::error!(
                    "{}",
                    t("log_preview_failed").replace("{:#}", &format!("{e:#}"))
                );
                self.status = t("status_preview_failed")
                    .replace("{:#}", &format!("{e:#}"));
            }
        }
    }

    fn export(&mut self, _cx: &mut Context<Self>) {
        let Some(samples) = self.result.clone() else {
            return;
        };
        let default_name = mp3_default_name(&self.result_text);
        let Some(path) = rfd::FileDialog::new()
            .add_filter(t("mp3_audio"), &["mp3"])
            .set_file_name(&format!("{default_name}.mp3"))
            .save_file()
        else {
            return;
        };
        log::info!(
            "{}",
            t("log_export_mp3")
                .replace("{}", &path.display().to_string())
                .replace("{default_name}", &default_name)
        );
        let start = Instant::now();
        match export_mp3(&samples, self.result_sr, &path) {
            Ok(()) => {
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                log::info!(
                    "{}",
                    t("log_export_done")
                        .replacen("{}", &path.display().to_string(), 1)
                        .replacen("{}", &fmt_bytes(size), 1)
                        .replace("{:.1}", &format!("{:.1}", start.elapsed().as_secs_f32()))
                );
                self.status = t("status_exported")
                    .replace("{}", &path.display().to_string());
            }
            Err(e) => {
                log::error!(
                    "{}",
                    t("log_export_failed").replace("{:#}", &format!("{e:#}"))
                );
                self.status = t("status_export_failed")
                    .replace("{:#}", &format!("{e:#}"));
            }
        }
    }

    fn pick_ref_audio(&mut self, _cx: &mut Context<Self>) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(t("audio_files"), &["wav", "mp3", "flac", "ogg", "m4a"])
            .pick_file()
        {
            log::info!(
                "{}",
                t("log_ref_selected").replace("{}", &path.display().to_string())
            );
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
                    .child(t(label)),
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
                    .child(t(label)),
            )
            .child(el)
    }

    fn refresh_select(
        entity: &Entity<SelectState<Vec<String>>>,
        items: Vec<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ix = entity.read(cx).selected_index(cx);
        entity.update(cx, |state, cx| {
            state.set_items(items, window, cx);
            state.set_selected_index(ix, window, cx);
        });
    }

    fn switch_language(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let new_lang = self.lang.toggle();
        set_language(new_lang);
        self.lang = new_lang;

        self.emotion_val = Emotion::from_label(&self.emotion_val)
            .label(new_lang)
            .to_string();
        self.pause_val = PauseStyle::from_label(&self.pause_val)
            .label(new_lang)
            .to_string();
        self.clone_model_val = CloneModel::from_label(&self.clone_model_val)
            .label(new_lang)
            .to_string();
        self.download_source_val = DownloadSource::from_label(&self.download_source_val)
            .display_label(new_lang)
            .to_string();
        self.status = t("status_idle").to_string();

        Self::refresh_select(&self.emotion, emotion_labels(), window, cx);
        Self::refresh_select(&self.pause, pause_labels(), window, cx);
        Self::refresh_select(&self.clone_model, clone_model_labels(), window, cx);
        Self::refresh_select(&self.download_source, source_labels(), window, cx);

        self.voice_desc.update(cx, |state, cx| {
            state.set_placeholder(t("voice_desc_placeholder"), window, cx);
        });
        self.text.update(cx, |state, cx| {
            state.set_placeholder(t("text_placeholder"), window, cx);
        });

        window.set_window_title(t("window_title"));
        log::info!("language switched to {:?}", new_lang);
        cx.notify();
    }

    fn status_badge(&self, cx: &Context<Self>) -> Div {
        let (icon, color) = if self.busy {
            (None, cx.theme().muted_foreground)
        } else if self.status.starts_with("失败") || self.status.starts_with("Failed") {
            (Some(IconName::CircleX), cx.theme().danger)
        } else if self.status.starts_with("合成完成") || self.status.starts_with("Synthesis complete") || self.status.starts_with("已导出") || self.status.starts_with("Exported") {
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
                                .child(t("app_name")),
                        )
                        .child(div().text_size(px(11.0)).text_color(muted).child(t("app_version"))),
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
                                    .label(t("mode_design"))
                                    .when(design_mode, |b| b.primary())
                                    .flex_1()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        log::info!("{}", t("log_mode_design"));
                                        this.mode = TtsMode::Design;
                                        this.status = t("status_design_mode").to_string();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("mode-clone")
                                    .icon(Icon::new(IconName::Copy))
                                    .label(t("mode_clone"))
                                    .when(!design_mode, |b| b.primary())
                                    .flex_1()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        log::info!("{}", t("log_mode_clone"));
                                        this.mode = TtsMode::Clone;
                                        this.status = t("status_clone_mode").to_string();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("lang-toggle")
                                    .icon(Icon::new(IconName::Globe))
                                    .label(self.lang.label())
                                    .small()
                                    .ghost()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.switch_language(window, cx);
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
                                            self.section_label(cx, IconName::User, "voice_description"),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_1()
                                                .items_center()
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .text_color(muted)
                                                        .child(t("quick_presets")),
                                                )
                                                .children({
                                                    let presets: &'static [(&'static str, &'static str)] =
                                                        Box::leak(voice_presets().to_vec().into_boxed_slice());
                                                    presets.iter().enumerate().map(
                                                        |(i, (name, _))| {
                                                            Button::new(("preset", i))
                                                                .label(*name)
                                                                .small()
                                                                .ghost()
                                                                .on_click(cx.listener(
                                                                    move |this, _, window, cx| {
                                                                        let text =
                                                                            voice_presets()[i].1;
                                                                        log::info!(
                                                                            "{}",
                                                                            t("log_preset_applied")
                                                                                .replace("{}", name)
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
                                                    ).collect::<Vec<_>>()
                                                }),
                                        ),
                                )
                                .child(div().w_full().child(Textarea::new(&self.voice_desc))),
                        )
                    })
                    .when(!design_mode, |d| {
                        d.child(
                            GroupBox::new()
                                .title(self.section_label(cx, IconName::Copy, "reference_audio"))
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            Button::new("pick-ref")
                                                .icon(Icon::new(IconName::FolderOpen))
                                                .label(t("choose_file"))
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
                                                    t("no_file_selected").to_string()
                                                } else {
                                                    self.ref_name.clone()
                                                }),
                                        ),
                                )
                                .child(
                                    self.field(
                                        cx,
                                        "clone_model",
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
                                        self.section_label(cx, IconName::BookOpen, "narration_text"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(muted)
                                            .child(format!("{} {}", text_chars, t("char_count"))),
                                    ),
                            )
                            .child(div().w_full().child(Textarea::new(&self.text))),
                    )
                    .child(
                        GroupBox::new()
                            .title(
                                self.section_label(cx, IconName::Settings2, "parameters"),
                            )
                            .child(
                                h_flex()
                                    .gap_3()
                                    .w_full()
                                    .child(
                                        self.field(
                                            cx,
                                            "speed",
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
                                            "emotion",
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
                                            "pause",
                                            div().w_full().child(Select::new(&self.pause)),
                                        ),
                                        design_mode,
                                    ))
                                    .child(self.field(
                                        cx,
                                        "language",
                                        div().w_full().child(Select::new(&self.language)),
                                    ))
                                    .child(self.field(
                                        cx,
                                        "download_source",
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
                                    .label(if busy { t("generating") } else { t("generate") })
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
                                    .label(if playing { t("stop") } else { t("preview") })
                                    .secondary()
                                    .disabled(busy || !has_result)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_preview(cx);
                                    })),
                            )
                            .child(
                                Button::new("export")
                                    .icon(Icon::new(IconName::File))
                                    .label(t("export_mp3"))
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
                                    .child(t("local_inference")),
                            ),
                    ),
            )
    }
}
