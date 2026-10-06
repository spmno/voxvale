use anyhow::{Context, Result};
use shine_rs::{Mp3Encoder, Mp3EncoderConfig, StereoMode};
use std::path::Path;

const MAX_NAME_CHARS: usize = 20;

pub fn mp3_default_name(text: &str) -> String {
    let sanitized: String = text
        .chars()
        .take(MAX_NAME_CHARS)
        .map(|c| {
            if matches!(
                c,
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
            ) || c.is_control()
            {
                ' '
            } else {
                c
            }
        })
        .collect();

    let trimmed = sanitized.trim().trim_matches('.').to_string();
    let mut name = trimmed;
    while name.contains("  ") {
        name = name.replace("  ", " ");
    }

    if name.is_empty() {
        "voxvale_output".to_string()
    } else {
        name
    }
}

pub fn export_mp3(samples: &[f32], sample_rate: u32, path: &Path) -> Result<()> {
    let pcm: Vec<i16> = samples
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect();

    let mut encoder = Mp3Encoder::new(
        Mp3EncoderConfig::new()
            .sample_rate(sample_rate)
            .bitrate(64)
            .channels(1)
            .stereo_mode(StereoMode::Mono),
    )?;

    let frame_size = encoder.samples_per_frame();
    let mut out: Vec<u8> = Vec::new();

    for chunk in pcm.chunks(frame_size) {
        let padded;
        let frame: &[i16] = if chunk.len() == frame_size {
            chunk
        } else {
            padded = {
                let mut v = chunk.to_vec();
                v.resize(frame_size, 0);
                v
            };
            &padded
        };
        for mp3_frame in encoder.encode_interleaved(frame)? {
            out.extend_from_slice(&mp3_frame);
        }
    }

    out.extend_from_slice(&encoder.finish()?);

    std::fs::write(path, &out)
        .with_context(|| format!("写入 MP3 失败: {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, sr: u32, secs: f32) -> Vec<f32> {
        let n = (sr as f32 * secs) as usize;
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin() * 0.5)
            .collect()
    }

    #[test]
    fn default_name_takes_first_chars_of_text() {
        let long = "小红帽提着篮子，蹦蹦跳跳地走进了森林深处，阳光透过树叶洒下来，鸟儿在枝头歌唱。";
        let name = mp3_default_name(long);
        assert_eq!(name.chars().count(), 20);
        assert!(long.starts_with(&name));
        assert!(!name.contains('\n'));
    }

    #[test]
    fn default_name_replaces_forbidden_chars() {
        let name = mp3_default_name("第一课：语文/数学*上册?");
        assert_eq!(name, "第一课：语文 数学 上册");
    }

    #[test]
    fn default_name_falls_back_when_empty() {
        assert_eq!(mp3_default_name(""), "voxvale_output");
        assert_eq!(mp3_default_name("  \n\t "), "voxvale_output");
        assert_eq!(mp3_default_name("///"), "voxvale_output");
    }

    #[test]
    fn mp3_export_roundtrip_writes_valid_file() {
        let dir = std::env::temp_dir().join("voxvale_test_export");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.mp3");

        let x = sine(440.0, 24000, 1.0);
        export_mp3(&x, 24000, &path).unwrap();

        let meta = std::fs::metadata(&path).unwrap();
        assert!(meta.len() > 1000, "mp3 too small: {}", meta.len());

        let head = std::fs::read(&path).unwrap();
        assert_eq!(head[0], 0xFF, "missing MPEG frame sync");
        assert_eq!(head[1] & 0xE0, 0xE0, "bad MPEG sync bits");

        std::fs::remove_file(&path).ok();
    }
}
