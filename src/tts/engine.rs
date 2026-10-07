use anyhow::{Context, Result, bail};
use candle_core::{Device, Tensor};
use qwen_tts::model::loader::{LoaderConfig, ModelLoader};
use qwen_tts::model::Model;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use crate::audio::decode::{TARGET_SAMPLE_RATE, resample_linear};
use crate::audio::stretch::time_stretch;
use crate::i18n::Language;

pub const DESIGN_MODEL_ID: &str = "Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign";
pub const CLONE_MODEL_06B: &str = "Qwen/Qwen3-TTS-12Hz-0.6B-Base";
pub const CLONE_MODEL_17B: &str = "Qwen/Qwen3-TTS-12Hz-1.7B-Base";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CloneModel {
    Base06B,
    Base17B,
}

impl CloneModel {
    pub fn label(&self, lang: Language) -> &'static str {
        match lang {
            Language::English => match self {
                Self::Base06B => "0.6B (faster, CPU recommended)",
                Self::Base17B => "1.7B (higher quality)",
            },
            Language::Chinese => match self {
                Self::Base06B => "0.6B（更快，推荐 CPU）",
                Self::Base17B => "1.7B（更高质量）",
            },
        }
    }

    pub fn from_label(label: &str) -> Self {
        if label.starts_with("1.7B") {
            Self::Base17B
        } else {
            Self::Base06B
        }
    }

    pub fn repo_id(self) -> &'static str {
        match self {
            Self::Base06B => CLONE_MODEL_06B,
            Self::Base17B => CLONE_MODEL_17B,
        }
    }
}

const PROGRESS_CHUNK: u64 = 1024 * 1024;
const READ_BUF: usize = 256 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TtsMode {
    Design,
    Clone,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DownloadSource {
    Mirror,
    Official,
}

impl DownloadSource {
    pub fn from_label(label: &str) -> Self {
        if label.starts_with("huggingface.co") {
            Self::Official
        } else {
            Self::Mirror
        }
    }

    pub fn endpoint(&self) -> &'static str {
        match self {
            Self::Mirror => "https://hf-mirror.com",
            Self::Official => "https://huggingface.co",
        }
    }

    pub fn label(&self, lang: Language) -> &'static str {
        match lang {
            Language::English => match self {
                Self::Mirror => "hf-mirror.com",
                Self::Official => "huggingface.co",
            },
            Language::Chinese => match self {
                Self::Mirror => "hf-mirror.com",
                Self::Official => "huggingface.co",
            },
        }
    }

    pub fn display_label(&self, lang: Language) -> &'static str {
        match lang {
            Language::English => match self {
                Self::Mirror => "hf-mirror.com (China mirror)",
                Self::Official => "huggingface.co (official)",
            },
            Language::Chinese => match self {
                Self::Mirror => "hf-mirror.com（国内镜像）",
                Self::Official => "huggingface.co（官方）",
            },
        }
    }
}

pub enum SynthRequest {
    Design {
        text: String,
        instruction: String,
        language: String,
        source: DownloadSource,
    },
    Clone {
        text: String,
        ref_pcm: Vec<f32>,
        ref_sample_rate: u32,
        speed: f32,
        language: String,
        source: DownloadSource,
        model: CloneModel,
    },
}

pub enum EngineEvent {
    ModelLoading { repo_id: String },
    DownloadProgress {
        file: String,
        file_index: usize,
        total_files: usize,
        bytes: u64,
        total_bytes: u64,
    },
    Generating,
    Done { samples: Vec<f32>, sample_rate: u32 },
    Failed { message: String },
}

pub struct TtsEngine {
    device: Device,
    models: HashMap<&'static str, Model>,
    dirs: HashMap<&'static str, PathBuf>,
}

impl TtsEngine {
    pub fn new() -> Self {
        Self {
            device: default_device(),
            models: HashMap::new(),
            dirs: HashMap::new(),
        }
    }

    fn ensure_model(
        &mut self,
        repo_id: &'static str,
        source: DownloadSource,
        events: &Sender<EngineEvent>,
    ) -> Result<&mut Model> {
        if self.models.contains_key(repo_id) {
            log::info!("模型 {repo_id} 已在内存中，跳过加载");
            return Ok(self.models.get_mut(repo_id).unwrap());
        }

        let dir = match self.dirs.get(repo_id) {
            Some(d) => d.clone(),
            None => {
                let d = download_model(repo_id, source, events)?;
                self.dirs.insert(repo_id, d.clone());
                d
            }
        };

        let _ = events.send(EngineEvent::ModelLoading {
            repo_id: repo_id.to_string(),
        });
        log::info!("开始加载模型权重 {repo_id} 到 {:?}…", self.device);
        let t = Instant::now();

        let loader = ModelLoader::from_local_dir(&dir)
            .with_context(|| format!("读取模型目录失败: {}", dir.display()))?;
        let model = loader
            .load_tts_model(&self.device, &LoaderConfig::default())
            .context("加载 TTS 模型失败")?;

        log::info!("模型 {repo_id} 加载完成，耗时 {:.1}s", t.elapsed().as_secs_f32());
        self.models.insert(repo_id, model);
        Ok(self.models.get_mut(repo_id).unwrap())
    }

    pub fn synthesize(
        &mut self,
        req: SynthRequest,
        events: &Sender<EngineEvent>,
    ) -> Result<(Vec<f32>, u32)> {
        let _ = events.send(EngineEvent::Generating);

        match req {
            SynthRequest::Design {
                text,
                instruction,
                language,
                source,
            } => {
                log::info!(
                    "合成任务[音色设计] 语言={language} 文本{}字 指令={instruction:?} 下载源={}",
                    text.chars().count(),
                    source.label(crate::i18n::get_language())
                );
                let t_total = Instant::now();
                let model = self.ensure_model(DESIGN_MODEL_ID, source, events)?;
                if !model.is_voice_design_model() {
                    bail!("加载的模型不是 VoiceDesign 类型");
                }
                let t_gen = Instant::now();
                let result = model.generate_voice_design_from_text(
                    &text,
                    &instruction,
                    &language,
                    None,
                )?;
                let (samples, sample_rate) = finish(result)?;
                log::info!(
                    "音色设计合成完成：输出 {:.1}s，纯合成 {:.1}s（含模型准备共 {:.1}s）",
                    samples.len() as f32 / sample_rate as f32,
                    t_gen.elapsed().as_secs_f32(),
                    t_total.elapsed().as_secs_f32()
                );
                Ok((samples, sample_rate))
            }
            SynthRequest::Clone {
                text,
                ref_pcm,
                ref_sample_rate,
                speed,
                language,
                source,
                model,
            } => {
                if ref_pcm.is_empty() {
                    bail!("参考音频为空");
                }
                log::info!(
                    "合成任务[语音克隆] 模型={} 语言={language} 文本{}字 参考{}样本@{ref_sample_rate}Hz 语速={speed} 下载源={}",
                    model.repo_id(),
                    text.chars().count(),
                    ref_pcm.len(),
                    source.label(crate::i18n::get_language())
                );
                let t_total = Instant::now();
                let pcm24 = resample_linear(&ref_pcm, ref_sample_rate, TARGET_SAMPLE_RATE);
                log::info!(
                    "参考音频重采样 {ref_sample_rate}Hz→{TARGET_SAMPLE_RATE}Hz：{}→{} 样本",
                    ref_pcm.len(),
                    pcm24.len()
                );
                let len = pcm24.len();
                let device = self.device.clone();
                let tensor = Tensor::from_vec(pcm24, (1, len), &device)?;
                let model_ref = self.ensure_model(model.repo_id(), source, events)?;
                if !model_ref.is_base_model() {
                    bail!("加载的模型不是 Base 类型，无法克隆");
                }
                let t_gen = Instant::now();
                let prompt = model_ref.create_voice_clone_prompt_from_audio(&tensor, None, true)?;
                let result =
                    model_ref.generate_voice_clone_from_text(&text, &prompt, &language, None)?;
                let (mut samples, sample_rate) = finish(result)?;
                if (speed - 1.0).abs() > 0.01 {
                    let before = samples.len();
                    samples = time_stretch(&samples, sample_rate, speed);
                    log::info!("WSOLA 变速 ×{speed:.2}：{before}→{} 样本", samples.len());
                }
                log::info!(
                    "语音克隆合成完成：输出 {:.1}s，纯合成 {:.1}s（含模型准备共 {:.1}s）",
                    samples.len() as f32 / sample_rate as f32,
                    t_gen.elapsed().as_secs_f32(),
                    t_total.elapsed().as_secs_f32()
                );
                Ok((samples, sample_rate))
            }
        }
    }
}

fn finish(result: qwen_tts::model::types::GenerationResult) -> Result<(Vec<f32>, u32)> {
    let sample_rate = result.sample_rate as u32;
    let audio = if result.audio.dims().len() > 1 {
        result.audio.squeeze(0)?
    } else {
        result.audio
    };
    let samples = audio.to_vec1::<f32>().context("音频张量转换失败")?;
    if samples.is_empty() {
        bail!("模型未生成任何音频");
    }
    Ok((samples, sample_rate))
}

pub fn default_device() -> Device {
    #[cfg(feature = "cuda")]
    {
        if let Ok(d) = Device::new_cuda(0) {
            return d;
        }
    }
    #[cfg(feature = "metal")]
    {
        if let Ok(d) = Device::new_metal(0) {
            return d;
        }
    }
    Device::Cpu
}

fn model_cache_root() -> Result<PathBuf> {
    let base = std::env::var("XDG_CACHE_HOME")
        .or_else(|_| std::env::var("HOME").map(|h| format!("{h}/.cache")))
        .unwrap_or_else(|_| "/tmp".to_string());
    Ok(PathBuf::from(base).join("voxvale/models"))
}

fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(30))
        .user_agent("voxvale/0.1")
        .build()
}

fn resolve_url(endpoint: &str, repo_id: &str, file: &str) -> String {
    format!("{endpoint}/{repo_id}/resolve/main/{file}")
}

fn fetch_to_string(agent: &ureq::Agent, url: &str) -> Result<String> {
    let resp = agent.get(url).call()?;
    let mut reader = resp.into_reader();
    let mut body = String::new();
    reader.read_to_string(&mut body)?;
    Ok(body)
}

fn repo_file_sizes(
    agent: &ureq::Agent,
    endpoint: &str,
    repo_id: &str,
) -> Result<HashMap<String, u64>> {
    let url = format!("{endpoint}/api/models/{repo_id}/tree/main?recursive=true");
    let body = fetch_to_string(agent, &url)
        .with_context(|| format!("获取仓库文件清单失败（{endpoint}）"))?;
    let entries: serde_json::Value =
        serde_json::from_str(&body).context("解析仓库文件清单失败")?;
    let arr = entries.as_array().context("仓库文件清单格式异常")?;

    let mut sizes = HashMap::new();
    for e in arr {
        if e.get("type").and_then(|t| t.as_str()) != Some("file") {
            continue;
        }
        if let (Some(path), Some(size)) =
            (e.get("path").and_then(|p| p.as_str()), e.get("size").and_then(|s| s.as_u64()))
        {
            sizes.insert(path.to_string(), size);
        }
    }
    Ok(sizes)
}

struct PlannedFile {
    remote: String,
    required: bool,
    size: Option<u64>,
}

fn download_model(
    repo_id: &'static str,
    source: DownloadSource,
    events: &Sender<EngineEvent>,
) -> Result<PathBuf> {
    let endpoint = source.endpoint();
    let agent = http_agent();
    let root = model_cache_root()?.join(repo_id);
    let t0 = Instant::now();

    log::info!("准备下载模型 {repo_id}，来源={}，缓存目录={}", source.label(crate::i18n::get_language()), root.display());

    let file_sizes = repo_file_sizes(&agent, endpoint, repo_id).with_context(|| {
        format!(
            "获取 {repo_id} 文件清单失败。若网络无法访问 {endpoint}，请在界面「下载源」中切换后重试"
        )
    })?;
    log::info!("仓库清单：{} 个文件", file_sizes.len());
    std::fs::create_dir_all(&root)?;

    let weight_files = if file_sizes.contains_key("model.safetensors") {
        vec!["model.safetensors".to_string()]
    } else {
        log::info!("无单文件权重，改用分片模式（先取索引）");
        let idx_url = resolve_url(endpoint, repo_id, "model.safetensors.index.json");
        let idx_dest = root.join("model.safetensors.index.json");
        fetch_to_file(&agent, &idx_url, &idx_dest)
            .context("下载 model.safetensors.index.json 失败")?;
        let index: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&idx_dest).context("读取权重索引失败")?,
        )?;
        let mut shards: Vec<String> = index
            .get("weight_map")
            .and_then(|m| m.as_object())
            .context("权重索引缺少 weight_map")?
            .values()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect();
        shards.sort();
        shards.dedup();
        log::info!("权重分为 {} 个分片", shards.len());
        shards
    };

    let size_of = |f: &str| file_sizes.get(f).copied();
    let mut plan: Vec<PlannedFile> = vec![PlannedFile {
        remote: "config.json".into(),
        required: true,
        size: size_of("config.json"),
    }];
    for w in &weight_files {
        plan.push(PlannedFile {
            remote: w.clone(),
            required: true,
            size: size_of(w),
        });
    }
    for (f, required) in [
        ("tokenizer.json", false),
        ("vocab.json", false),
        ("merges.txt", false),
        ("tokenizer_config.json", false),
        ("speech_tokenizer/config.json", true),
        ("speech_tokenizer/model.safetensors", true),
    ] {
        plan.push(PlannedFile {
            remote: f.into(),
            required,
            size: size_of(f),
        });
    }

    let total_files = plan.len();
    log::info!(
        "下载计划共 {total_files} 个文件，合计 {}",
        fmt_bytes(plan.iter().filter_map(|p| p.size).sum())
    );

    let mut has_tokenizer = false;
    for (i, item) in plan.iter().enumerate() {
        let file_index = i + 1;
        let dest = root.join(&item.remote);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if let Some(size) = item.size
            && dest.is_file()
            && dest.metadata()?.len() == size
        {
            log::info!("[{file_index}/{total_files}] {} 缓存命中，跳过下载（{}）", item.remote, fmt_bytes(size));
            let _ = events.send(EngineEvent::DownloadProgress {
                file: item.remote.clone(),
                file_index,
                total_files,
                bytes: size,
                total_bytes: size,
            });
            has_tokenizer |= item.remote == "tokenizer.json" || item.remote == "vocab.json";
            continue;
        }

        log::info!(
            "[{file_index}/{total_files}] 开始下载 {}（{}）",
            item.remote,
            item.size.map(fmt_bytes).unwrap_or_else(|| "大小未知".into())
        );
        let t = Instant::now();
        let result = fetch_with_progress(
            &agent,
            &resolve_url(endpoint, repo_id, &item.remote),
            &dest,
            item.size,
            events,
            file_index,
            total_files,
        );
        match result {
            Ok(n) => {
                log::info!(
                    "[{file_index}/{total_files}] {} 下载完成 {}，耗时 {:.1}s",
                    item.remote,
                    fmt_bytes(n),
                    t.elapsed().as_secs_f32()
                );
                has_tokenizer |= item.remote == "tokenizer.json" || item.remote == "vocab.json";
            }
            Err(e) => {
                if item.required {
                    return Err(e).with_context(|| {
                        format!(
                            "下载必需文件 {} 失败。可尝试在「下载源」中切换为另一个镜像后重试",
                            item.remote
                        )
                    });
                }
                log::warn!("可选文件 {} 下载失败，跳过：{e}", item.remote);
            }
        }
    }

    if !has_tokenizer {
        bail!("tokenizer.json 与 vocab.json 均未获取到，文本分词器缺失");
    }

    log::info!(
        "模型 {repo_id} 全部文件就绪，总耗时 {:.1}s，目录 {}",
        t0.elapsed().as_secs_f32(),
        root.display()
    );
    Ok(root)
}

fn fetch_to_file(agent: &ureq::Agent, url: &str, dest: &Path) -> Result<u64> {
    let resp = agent.get(url).call()?;
    let mut reader = resp.into_reader();
    let mut out = std::fs::File::create(dest)?;
    let mut buf = vec![0u8; READ_BUF];
    let mut n_total = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        n_total += n as u64;
    }
    Ok(n_total)
}

fn fetch_with_progress(
    agent: &ureq::Agent,
    url: &str,
    dest: &Path,
    expected_size: Option<u64>,
    events: &Sender<EngineEvent>,
    file_index: usize,
    total_files: usize,
) -> Result<u64> {
    let part = dest.with_file_name(format!(
        "{}.part",
        dest.file_name().unwrap_or_default().to_string_lossy()
    ));

    let mut have = if part.is_file() {
        part.metadata()?.len()
    } else {
        0
    };
    if let Some(exp) = expected_size
        && have > exp
    {
        log::warn!("残留 .part（{}）超过预期大小，重新下载", fmt_bytes(have));
        have = 0;
    }

    let (resp, resumed) = match agent
        .get(url)
        .set("Range", &format!("bytes={have}-"))
        .call()
    {
        Ok(r) if r.status() == 206 && have > 0 => (r, true),
        Ok(r) => {
            if have > 0 {
                log::info!("服务器未支持断点续传（HTTP {}），从头下载", r.status());
            }
            (r, false)
        }
        Err(ureq::Error::Status(code, _)) => {
            bail!("HTTP {code}")
        }
        Err(e) => return Err(e.into()),
    };
    if !resumed {
        have = 0;
    }

    let total = expected_size.unwrap_or(0);
    let name = dest.file_name().unwrap_or_default().to_string_lossy().to_string();
    if let Some(exp) = expected_size
        && dest.is_file()
        && dest.metadata()?.len() == exp
    {
        log::info!("目标文件已存在且大小一致，丢弃 .part 直接完成");
        std::fs::remove_file(&part).ok();
        let _ = events.send(EngineEvent::DownloadProgress {
            file: name,
            file_index,
            total_files,
            bytes: exp,
            total_bytes: exp,
        });
        return Ok(exp);
    }

    log::debug!("流式下载 {url}（断点续传={resumed}，起始偏移 {have}，总量 {total}）");
    let mut reader = resp.into_reader();
    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(!resumed)
        .append(resumed)
        .open(&part)?;

    let mut buf = vec![0u8; READ_BUF];
    let mut done = have;
    let mut last_emit = have;
    if resumed {
        let _ = events.send(EngineEvent::DownloadProgress {
            file: name.clone(),
            file_index,
            total_files,
            bytes: have,
            total_bytes: total,
        });
    }

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        done += n as u64;
        if done - last_emit >= PROGRESS_CHUNK {
            let _ = events.send(EngineEvent::DownloadProgress {
                file: name.clone(),
                file_index,
                total_files,
                bytes: done,
                total_bytes: total,
            });
            last_emit = done;
        }
    }

    if let Some(exp) = expected_size
        && done != exp
    {
        bail!("下载不完整：预期 {exp} 字节，实际 {done} 字节");
    }

    let _ = events.send(EngineEvent::DownloadProgress {
        file: name,
        file_index,
        total_files,
        bytes: done,
        total_bytes: total,
    });
    std::fs::rename(&part, dest)?;
    Ok(done)
}

pub fn fmt_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= KB.powi(3) {
        format!("{:.2} GB", b / KB.powi(3))
    } else if b >= KB.powi(2) {
        format!("{:.1} MB", b / KB.powi(2))
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

pub fn spawn_worker() -> (Sender<SynthRequest>, Receiver<EngineEvent>) {
    let (req_tx, req_rx) = channel::<SynthRequest>();
    let (ev_tx, ev_rx) = channel::<EngineEvent>();

    std::thread::Builder::new()
        .name("voxvale-tts".into())
        .spawn(move || {
            log::info!("TTS 工作线程已启动");
            let mut engine = TtsEngine::new();
            while let Ok(req) = req_rx.recv() {
                match engine.synthesize(req, &ev_tx) {
                    Ok((samples, sample_rate)) => {
                        let _ = ev_tx.send(EngineEvent::Done {
                            samples,
                            sample_rate,
                        });
                    }
                    Err(e) => {
                        log::error!("合成失败：{e:#}");
                        let _ = ev_tx.send(EngineEvent::Failed {
                            message: format!("{e:#}"),
                        });
                    }
                }
            }
            log::warn!("TTS 工作线程退出（请求通道已关闭）");
        })
        .expect("启动 TTS 工作线程失败");

    (req_tx, ev_rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "需要网络"]
    fn fetch_config_json_from_mirror() {
        let agent = http_agent();
        let sizes = repo_file_sizes(&agent, "https://hf-mirror.com", "Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign")
            .expect("tree API 获取失败");
        let cfg = sizes
            .get("config.json")
            .copied()
            .expect("清单缺少 config.json");
        assert!(cfg > 100, "config.json 大小异常: {cfg}");
        assert!(
            sizes.contains_key("model.safetensors") || sizes.contains_key("model.safetensors.index.json"),
            "清单缺少模型权重"
        );
        log::info!(
            "镜像 tree API 验证通过：config.json {}，共 {} 个文件",
            fmt_bytes(cfg),
            sizes.len()
        );

        let url = resolve_url(
            "https://hf-mirror.com",
            "Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign",
            "config.json",
        );
        let dest = std::env::temp_dir().join("voxvale_test_config.json");
        let n = fetch_to_file(&agent, &url, &dest).unwrap();
        assert!(n > 100, "下载字节数异常: {n}");
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&dest).unwrap()).unwrap();
        assert!(parsed.get("model_type").is_some() || parsed.get("architectures").is_some());
        std::fs::remove_file(&dest).ok();
        log::info!("镜像下载验证通过：config.json {n} 字节");
    }

    #[test]
    fn fmt_bytes_scales() {
        assert_eq!(fmt_bytes(0), "0 B");
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(2048), "2 KB");
        assert_eq!(fmt_bytes(1536 * 1024), "1.5 MB");
        assert_eq!(fmt_bytes(3_800_000_000), "3.54 GB");
    }

    #[test]
    fn download_source_maps_from_labels() {
        assert_eq!(
            DownloadSource::from_label("hf-mirror.com（国内镜像）"),
            DownloadSource::Mirror
        );
        assert_eq!(
            DownloadSource::from_label("huggingface.co（官方）"),
            DownloadSource::Official
        );
        assert_eq!(
            DownloadSource::Mirror.endpoint(),
            "https://hf-mirror.com"
        );
        assert_eq!(
            DownloadSource::Official.endpoint(),
            "https://huggingface.co"
        );
    }
}
