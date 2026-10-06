use std::sync::mpsc::channel;
use std::time::Instant;
use voxvale::tts::engine::{DownloadSource, SynthRequest, TtsEngine};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let (events, _ev_rx) = channel();
    let mut engine = TtsEngine::new();

    let text = "这是声谷桌面语音工具的性能基准测试，用于测量本地合成的实时率。";
    let t0 = Instant::now();
    let result = engine.synthesize(
        SynthRequest::Design {
            text: text.to_string(),
            instruction: "沉稳的青年男声，语气自然".to_string(),
            language: "chinese".to_string(),
            source: DownloadSource::Mirror,
        },
        &events,
    );

    match result {
        Ok((samples, sr)) => {
            let elapsed = t0.elapsed().as_secs_f32();
            let audio_secs = samples.len() as f32 / sr as f32;
            println!("音频时长: {audio_secs:.2}s");
            println!("合成耗时: {elapsed:.2}s");
            println!("RTF (耗时/音频时长): {elapsed:.2} / {audio_secs:.2} = {:.2}x", elapsed / audio_secs);
        }
        Err(e) => {
            eprintln!("BENCH FAILED: {e:#}");
        }
    }
}
