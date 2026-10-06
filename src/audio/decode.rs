use anyhow::{Context, Result, bail};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub const TARGET_SAMPLE_RATE: u32 = 24000;

pub fn decode_audio_file(path: &Path) -> Result<(Vec<f32>, u32)> {
    let src = std::fs::File::open(path)
        .with_context(|| format!("无法打开音频文件: {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("文件中未找到音频轨道"))?
        .clone();

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| anyhow::anyhow!("未知采样率"))?;
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(1)
        .max(1);

    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let mut interleaved: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(_))
            | Err(symphonia::core::errors::Error::ResetRequired) => break,
            Err(e) => return Err(e).context("读取音频数据失败"),
        };

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let mut buf = SampleBuffer::<f32>::new(decoded.frames() as u64, *decoded.spec());
                buf.copy_interleaved_ref(decoded);
                interleaved.extend_from_slice(buf.samples());
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e).context("解码音频失败"),
        }
    }

    if interleaved.is_empty() {
        bail!("音频文件为空: {}", path.display());
    }

    let mono = if channels > 1 {
        interleaved
            .chunks(channels)
            .map(|ch| ch.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        interleaved
    };

    Ok((mono, sample_rate))
}

pub fn resample_linear(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let out_len = ((input.len() as f64) * (to as f64 / from as f64)).round() as usize;
    let ratio = from as f64 / to as f64;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let i0 = pos.floor() as usize;
            let i1 = (i0 + 1).min(input.len() - 1);
            let frac = (pos - i0 as f64) as f32;
            input[i0] * (1.0 - frac) + input[i1] * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_identity() {
        let x = vec![0.1, 0.2, 0.3];
        assert_eq!(resample_linear(&x, 24000, 24000), x);
    }

    #[test]
    fn resample_upsample_length() {
        let x = vec![0.0, 1.0, 0.0, -1.0];
        let y = resample_linear(&x, 12000, 24000);
        assert_eq!(y.len(), x.len() * 2);
    }

    #[test]
    fn resample_downsample_length() {
        let x = vec![0.0; 4800];
        let y = resample_linear(&x, 48000, 24000);
        assert_eq!(y.len(), 2400);
    }
}
