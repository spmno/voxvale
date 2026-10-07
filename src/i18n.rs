use std::sync::Mutex;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Language {
    English,
    Chinese,
}

impl Language {
    pub fn label(&self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Chinese => "中文",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::English => Self::Chinese,
            Self::Chinese => Self::English,
        }
    }
}

static CURRENT_LANGUAGE: Mutex<Language> = Mutex::new(Language::English);

pub fn get_language() -> Language {
    *CURRENT_LANGUAGE.lock().unwrap()
}

pub fn set_language(lang: Language) {
    *CURRENT_LANGUAGE.lock().unwrap() = lang;
}

pub fn t(key: &str) -> &'static str {
    let lang = get_language();
    match lang {
        Language::English => en(key),
        Language::Chinese => zh(key),
    }
}

fn en(key: &str) -> &'static str {
    match key {
        // Title bar
        "app_name" => "VoxVale",
        "app_version" => "v0.1.0",

        // Mode buttons
        "mode_design" => "Voice Design",
        "mode_clone" => "Voice Cloning",

        // Voice design section
        "voice_description" => "Voice Description",
        "quick_presets" => "Quick Presets:",
        "preset_gentle_girl" => "Gentle Girl",
        "preset_lively_boy" => "Lively Boy",
        "preset_intellectual_female" => "Intellectual Female",
        "preset_calm_male" => "Calm Male",
        "voice_desc_placeholder" => "Describe the desired voice, or click a preset above",

        // Voice cloning section
        "reference_audio" => "Reference Audio",
        "choose_file" => "Choose File",
        "no_file_selected" => "None selected (wav / mp3 / flac / ogg)",
        "clone_model" => "Clone Model",

        // Text section
        "narration_text" => "Narration Text",
        "text_placeholder" => "Enter text to narrate…",
        "char_count" => "chars",

        // Parameters
        "parameters" => "Parameters",
        "speed" => "Speed",
        "emotion" => "Emotion",
        "pause" => "Pause",
        "language" => "Language",
        "download_source" => "Download Source",

        // Action buttons
        "generate" => "Generate Speech",
        "generating" => "Generating…",
        "preview" => "Preview",
        "stop" => "Stop",
        "export_mp3" => "Export MP3",

        // Status messages
        "status_idle" => "Enter text and click \"Generate Speech\" to start",
        "status_design_mode" => "Switched to Voice Design mode",
        "status_clone_mode" => "Switched to Voice Cloning mode",
        "status_text_required" => "Please enter narration text first",
        "status_ref_required" => "Voice cloning requires a reference audio file",
        "status_submitted" => "Synthesis task submitted…",
        "status_worker_dead" => "Synthesis thread has exited, please restart the app",
        "status_generating" => "Generating… (local inference, speed depends on your machine)",
        "status_done" => "Synthesis complete, duration {:.1}s, preview or export MP3",
        "status_failed" => "Failed: {}",
        "status_decode_failed" => "Failed to decode reference audio: {:#}",
        "status_preview_failed" => "Preview failed: {:#}",
        "status_exported" => "Exported: {}",
        "status_export_failed" => "Export failed: {:#}",

        // Download progress
        "downloading_model" => "Downloading model {}/{} {}: {} / {} ({:.0}%)",
        "downloading_model_no_total" => "Downloading model {}/{} {}: {}",
        "model_loading" => "Model files ready, loading weights into memory ({})…",

        // Footer
        "local_inference" => "Local inference · Data never leaves your device",

        // Emotion labels
        "emotion_natural" => "Natural",
        "emotion_warm" => "Warm",
        "emotion_cheerful" => "Cheerful",
        "emotion_calm" => "Calm",
        "emotion_sad" => "Sad",
        "emotion_excited" => "Excited",
        "emotion_serious" => "Serious",

        // Pause labels
        "pause_natural" => "Natural",
        "pause_more" => "More",
        "pause_less" => "Less",

        // Clone model labels
        "clone_06b" => "0.6B (faster, CPU recommended)",
        "clone_17b" => "1.7B (higher quality)",

        // Download source labels
        "source_mirror" => "hf-mirror.com (China mirror)",
        "source_official" => "huggingface.co (official)",

        // Language labels
        "lang_chinese" => "chinese",
        "lang_auto" => "auto",
        "lang_english" => "english",

        // File dialog
        "audio_files" => "Audio Files",
        "mp3_audio" => "MP3 Audio",

        // Log messages
        "log_startup" => "VoxVale started (local TTS · data never leaves device)",
        "log_log_level" => "Log level adjustable via RUST_LOG, e.g. RUST_LOG=voxvale=debug",
        "log_mode_design" => "Switched to Voice Design mode",
        "log_mode_clone" => "Switched to Voice Cloning mode",
        "log_preset_applied" => "Applied voice preset: {}",
        "log_ref_selected" => "Selected reference audio: {}",
        "log_preview_play" => "Preview started: {} samples @{}Hz",
        "log_preview_stop" => "Preview stopped",
        "log_export_mp3" => "Export MP3 → {} (default name {}.mp3, from first 20 chars)",
        "log_export_done" => "Export complete {} ({}), took {:.1}s",
        "log_export_failed" => "Export failed: {:#}",
        "log_synth_design" => "Submit synthesis [Voice Design] speed={:.2} emotion={} pause={} language={} source={}",
        "log_synth_clone" => "Submit synthesis [Voice Cloning] model={} speed={:.2} language={} source={} ref={}",
        "log_decode_failed" => "Failed to decode reference audio {:?}: {:#}",
        "log_preview_failed" => "Preview playback failed: {:#}",
        "log_source_switch" => "Download source switched to {}",
        "log_clone_model_switch" => "Clone model switched to {}",
        "log_model_cached" => "Model {} already in memory, skipping load",
        "log_model_load_start" => "Loading model weights {} to {:?}…",
        "log_model_load_done" => "Model {} loaded, took {:.1}s",
        "log_design_done" => "Voice Design synthesis complete: output {:.1}s, pure synthesis {:.1}s (total {:.1}s)",
        "log_clone_done" => "Voice Cloning synthesis complete: output {:.1}s, pure synthesis {:.1}s (total {:.1}s)",
        "log_resample" => "Reference audio resampled {}Hz→{}Hz: {}→{} samples",
        "log_wsolA" => "WSOLA time-stretch ×{:.2}: {}→{} samples",
        "log_worker_started" => "TTS worker thread started",
        "log_worker_exited" => "TTS worker thread exited (request channel closed)",
        "log_synth_failed" => "Synthesis failed: {:#}",
        "log_download_start" => "Preparing to download model {}, source={}, cache dir={}",
        "log_repo_list" => "Repository file list: {} files",
        "log_no_single_file" => "No single-file weights, using shard mode (fetch index first)",
        "log_shards" => "Weights split into {} shards",
        "log_download_plan" => "Download plan: {} files, total {}",
        "log_cache_hit" => "[{}/{}] {} cache hit, skipping download ({})",
        "log_download_start_file" => "[{}/{}] Starting download {} ({})",
        "log_download_done" => "[/] {} downloaded {}, took {:.1}s",
        "log_download_failed_optional" => "Optional file {} download failed, skipping: {}",
        "log_download_failed_required" => "Failed to download required file {}. Try switching download source and retry",
        "log_download_incomplete" => "Incomplete download: expected {} bytes, got {} bytes",
        "log_model_ready" => "Model {} all files ready, total time {:.1}s, directory {}",
        "log_no_resume" => "Server does not support resume (HTTP {}), downloading from start",
        "log_part_exceeded" => "Residual .part ({}) exceeds expected size, re-downloading",
        "log_target_exists" => "Target file exists and size matches, discarding .part",
        "log_streaming_download" => "Streaming download {} (resume={}, start offset {}, total {})",
        "log_window_opened" => "Main window opened",
        "log_gpui_init" => "gpui initialization complete",
        "log_audio_init_failed" => "Audio output device initialization failed, preview unavailable",

        // Error messages
        "err_model_not_voice_design" => "Loaded model is not VoiceDesign type",
        "err_model_not_base" => "Loaded model is not Base type, cannot clone",
        "err_ref_audio_empty" => "Reference audio is empty",
        "err_no_audio_generated" => "Model generated no audio",
        "err_tokenizer_missing" => "Neither tokenizer.json nor vocab.json obtained, text tokenizer missing",
        "err_read_model_dir" => "Failed to read model directory: {}",
        "err_load_tts" => "Failed to load TTS model",
        "err_tensor_convert" => "Audio tensor conversion failed",
        "err_http" => "HTTP {}",
        "err_repo_list_failed" => "Failed to get repository file list ({})",
        "err_repo_list_parse" => "Failed to parse repository file list",
        "err_repo_list_format" => "Repository file list format error",
        "err_index_read" => "Failed to read weight index",
        "err_index_no_weight_map" => "Weight index missing weight_map",
        "err_index_download" => "Failed to download model.safetensors.index.json",
        "err_file_list_failed" => "Failed to get file list for {}. If network cannot access {}, try switching download source in the UI",

        // Window title
        "window_title" => "VoxVale",

        _ => "unknown",
    }
}

fn zh(key: &str) -> &'static str {
    match key {
        // Title bar
        "app_name" => "声谷 VoxVale",
        "app_version" => "v0.1.0",

        // Mode buttons
        "mode_design" => "音色设计",
        "mode_clone" => "语音克隆",

        // Voice design section
        "voice_description" => "音色描述",
        "quick_presets" => "快速预设：",
        "preset_gentle_girl" => "温柔女童",
        "preset_lively_boy" => "活泼男孩",
        "preset_intellectual_female" => "知性女声",
        "preset_calm_male" => "沉稳男声",
        "voice_desc_placeholder" => "描述想要的声音，或点击上方预设快速填充",

        // Voice cloning section
        "reference_audio" => "参考音频",
        "choose_file" => "选择文件",
        "no_file_selected" => "未选择（支持 wav / mp3 / flac / ogg）",
        "clone_model" => "克隆模型",

        // Text section
        "narration_text" => "朗读文本",
        "text_placeholder" => "输入要朗读的文本…",
        "char_count" => "字",

        // Parameters
        "parameters" => "参数",
        "speed" => "语速",
        "emotion" => "情绪",
        "pause" => "停顿",
        "language" => "语言",
        "download_source" => "下载源",

        // Action buttons
        "generate" => "生成语音",
        "generating" => "合成中…",
        "preview" => "预览",
        "stop" => "停止",
        "export_mp3" => "导出 MP3",

        // Status messages
        "status_idle" => "输入文本，点击「生成语音」开始",
        "status_design_mode" => "已切换到音色设计模式",
        "status_clone_mode" => "已切换到语音克隆模式",
        "status_text_required" => "请先输入朗读文本",
        "status_ref_required" => "语音克隆需要先选择参考音频",
        "status_submitted" => "已提交合成任务…",
        "status_worker_dead" => "合成线程已退出，请重启应用",
        "status_generating" => "合成中…（本地推理，速度取决于机器性能）",
        "status_done" => "合成完成，时长 {:.1} 秒，可预览或导出 MP3",
        "status_failed" => "失败：{}",
        "status_decode_failed" => "参考音频解码失败：{:#}",
        "status_preview_failed" => "预览失败：{:#}",
        "status_exported" => "已导出：{}",
        "status_export_failed" => "导出失败：{:#}",

        // Download progress
        "downloading_model" => "下载模型 {}/{} {}：{} / {}（{:.0}%）",
        "downloading_model_no_total" => "下载模型 {}/{} {}：{}",
        "model_loading" => "模型文件就绪，加载权重到内存（{}）…",

        // Footer
        "local_inference" => "本地推理 · 数据不出设备",

        // Emotion labels
        "emotion_natural" => "自然",
        "emotion_warm" => "温暖",
        "emotion_cheerful" => "欢快",
        "emotion_calm" => "平静",
        "emotion_sad" => "忧伤",
        "emotion_excited" => "激动",
        "emotion_serious" => "严肃",

        // Pause labels
        "pause_natural" => "自然停顿",
        "pause_more" => "多停顿",
        "pause_less" => "少停顿",

        // Clone model labels
        "clone_06b" => "0.6B（更快，推荐 CPU）",
        "clone_17b" => "1.7B（更高质量）",

        // Download source labels
        "source_mirror" => "hf-mirror.com（国内镜像）",
        "source_official" => "huggingface.co（官方）",

        // Language labels
        "lang_chinese" => "chinese",
        "lang_auto" => "auto",
        "lang_english" => "english",

        // File dialog
        "audio_files" => "音频文件",
        "mp3_audio" => "MP3 音频",

        // Log messages
        "log_startup" => "声谷 VoxVale 启动（本地 TTS · 数据不出设备）",
        "log_log_level" => "日志级别可通过 RUST_LOG 调整，如 RUST_LOG=voxvale=debug",
        "log_mode_design" => "切换到音色设计模式",
        "log_mode_clone" => "切换到语音克隆模式",
        "log_preset_applied" => "应用音色预设：{}",
        "log_ref_selected" => "已选择参考音频：{}",
        "log_preview_play" => "开始预览播放：{} 样本 @{}Hz",
        "log_preview_stop" => "预览停止",
        "log_export_mp3" => "导出 MP3 → {}（默认名 {default_name}.mp3，来自朗读文本前 20 字）",
        "log_export_done" => "导出完成 {}（{}），耗时 {:.1}s",
        "log_export_failed" => "导出失败：{:#}",
        "log_synth_design" => "提交合成[音色设计] 语速×{:.2} 情绪={} 停顿={} 语言={} 下载源={}",
        "log_synth_clone" => "提交合成[语音克隆] 模型={} 语速×{:.2} 语言={} 下载源={} 参考={}",
        "log_decode_failed" => "参考音频解码失败 {:?}：{:#}",
        "log_preview_failed" => "预览播放失败：{:#}",
        "log_source_switch" => "下载源切换为 {}",
        "log_clone_model_switch" => "克隆模型切换为 {}",
        "log_model_cached" => "模型 {} 已在内存中，跳过加载",
        "log_model_load_start" => "开始加载模型权重 {} 到 {:?}…",
        "log_model_load_done" => "模型 {} 加载完成，耗时 {:.1}s",
        "log_design_done" => "音色设计合成完成：输出 {:.1}s，纯合成 {:.1}s（含模型准备共 {:.1}s）",
        "log_clone_done" => "语音克隆合成完成：输出 {:.1}s，纯合成 {:.1}s（含模型准备共 {:.1}s）",
        "log_resample" => "参考音频重采样 {}Hz→{}Hz：{}→{} 样本",
        "log_wsolA" => "WSOLA 变速 ×{:.2}：{}→{} 样本",
        "log_worker_started" => "TTS 工作线程已启动",
        "log_worker_exited" => "TTS 工作线程退出（请求通道已关闭）",
        "log_synth_failed" => "合成失败：{:#}",
        "log_download_start" => "准备下载模型 {}，来源={}，缓存目录={}",
        "log_repo_list" => "仓库清单：{} 个文件",
        "log_no_single_file" => "无单文件权重，改用分片模式（先取索引）",
        "log_shards" => "权重分为 {} 个分片",
        "log_download_plan" => "下载计划共 {} 个文件，合计 {}",
        "log_cache_hit" => "[{}/{}] {} 缓存命中，跳过下载（{}）",
        "log_download_start_file" => "[{}/{}] 开始下载 {}（{}）",
        "log_download_done" => "[{}/{}] {} 下载完成 {}，耗时 {:.1}s",
        "log_download_failed_optional" => "可选文件 {} 下载失败，跳过：{}",
        "log_download_failed_required" => "下载必需文件 {} 失败。可尝试在「下载源」中切换为另一个镜像后重试",
        "log_download_incomplete" => "下载不完整：预期 {} 字节，实际 {} 字节",
        "log_model_ready" => "模型 {} 全部文件就绪，总耗时 {:.1}s，目录 {}",
        "log_no_resume" => "服务器未支持断点续传（HTTP {}），从头下载",
        "log_part_exceeded" => "残留 .part（{}）超过预期大小，重新下载",
        "log_target_exists" => "目标文件已存在且大小一致，丢弃 .part 直接完成",
        "log_streaming_download" => "流式下载 {}（断点续传={}，起始偏移 {}，总量 {}）",
        "log_window_opened" => "主窗口已打开",
        "log_gpui_init" => "gpui 初始化完成",
        "log_audio_init_failed" => "音频输出设备初始化失败，预览不可用",

        // Error messages
        "err_model_not_voice_design" => "加载的模型不是 VoiceDesign 类型",
        "err_model_not_base" => "加载的模型不是 Base 类型，无法克隆",
        "err_ref_audio_empty" => "参考音频为空",
        "err_no_audio_generated" => "模型未生成任何音频",
        "err_tokenizer_missing" => "tokenizer.json 与 vocab.json 均未获取到，文本分词器缺失",
        "err_read_model_dir" => "读取模型目录失败: {}",
        "err_load_tts" => "加载 TTS 模型失败",
        "err_tensor_convert" => "音频张量转换失败",
        "err_http" => "HTTP {}",
        "err_repo_list_failed" => "获取仓库文件清单失败（{}）",
        "err_repo_list_parse" => "解析仓库文件清单失败",
        "err_repo_list_format" => "仓库文件清单格式异常",
        "err_index_read" => "读取权重索引失败",
        "err_index_no_weight_map" => "权重索引缺少 weight_map",
        "err_index_download" => "下载 model.safetensors.index.json 失败",
        "err_file_list_failed" => "获取 {} 文件清单失败。若网络无法访问 {}，请在界面「下载源」中切换后重试",

        // Window title
        "window_title" => "声谷 VoxVale",

        _ => "unknown",
    }
}
