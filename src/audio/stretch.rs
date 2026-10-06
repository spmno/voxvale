const MIN_SPEED: f32 = 0.5;
const MAX_SPEED: f32 = 2.0;

pub fn time_stretch(input: &[f32], sample_rate: u32, speed: f32) -> Vec<f32> {
    assert!(
        (MIN_SPEED..=MAX_SPEED).contains(&speed),
        "speed {speed} out of range [{MIN_SPEED}, {MAX_SPEED}]"
    );

    if input.is_empty() || (speed - 1.0).abs() < 0.01 {
        return input.to_vec();
    }

    let win = (sample_rate as usize / 10).max(240);
    let syn_hop = win / 2;
    let ana_hop = ((syn_hop as f64) * (speed as f64)).round() as usize;
    let search = win / 4;
    let overlap = syn_hop;

    let out_len = (input.len() as f64 / speed as f64).ceil() as usize + win * 2;
    let mut out = vec![0.0f32; out_len];

    let hann: Vec<f32> = (0..win)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / win as f64).cos())
        .map(|w| w as f32)
        .collect();

    let mut ana_pos = 0usize;
    let mut out_pos = 0usize;
    let mut first = true;

    while ana_pos + win < input.len() && out_pos + win < out_len {
        let start = if !first && ana_pos > search {
            find_alignment(
                &out[out_pos..out_pos + overlap],
                input,
                ana_pos - search,
                search,
                overlap,
            )
            .unwrap_or(ana_pos)
        } else {
            ana_pos
        };

        if start + win > input.len() {
            break;
        }

        if first {
            for i in 0..win {
                out[out_pos + i] = input[start + i] * hann[i];
            }
            first = false;
        } else {
            for i in 0..overlap {
                let t = i as f32 / overlap as f32;
                let old = out[out_pos + i];
                let new = input[start + i];
                out[out_pos + i] = old * (1.0 - t) + new * t;
            }
            for i in overlap..win {
                out[out_pos + i] = input[start + i];
            }
        }

        out_pos += syn_hop;
        ana_pos += ana_hop;
    }

    out.truncate((out_pos + win).min(out_len));
    out
}

fn find_alignment(
    target: &[f32],
    input: &[f32],
    base: usize,
    search: usize,
    len: usize,
) -> Option<usize> {
    let mut best = base;
    let mut best_score = f32::NEG_INFINITY;

    for d in 0..=2 * search {
        let start = base + d;
        if start + len > input.len() {
            break;
        }
        let mut score = 0.0f32;
        for i in 0..len {
            score += target[i] * input[start + i];
        }
        if score > best_score {
            best_score = score;
            best = start;
        }
    }

    Some(best)
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
    fn faster_stretch_shortens_length() {
        let sr = 24000;
        let x = sine(220.0, sr, 2.0);
        let y = time_stretch(&x, sr, 1.25);
        let expect = x.len() as f64 / 1.25;
        assert!(
            (y.len() as f64 - expect).abs() / expect < 0.06,
            "len {} vs expect {}",
            y.len(),
            expect
        );
    }

    #[test]
    fn slower_stretch_lengthens_audio() {
        let sr = 24000;
        let x = sine(220.0, sr, 2.0);
        let y = time_stretch(&x, sr, 0.8);
        let expect = x.len() as f64 / 0.8;
        assert!((y.len() as f64 - expect).abs() / expect < 0.06);
    }

    #[test]
    fn identity_speed_returns_input() {
        let x = sine(440.0, 24000, 0.5);
        let y = time_stretch(&x, 24000, 1.0);
        assert_eq!(x, y);
    }

    #[test]
    fn output_is_continuous_no_glitches() {
        let sr = 24000;
        let x = sine(440.0, sr, 1.0);
        let y = time_stretch(&x, sr, 1.3);
        let max_jump = y
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_jump < 0.3,
            "adjacent-sample jump {max_jump} indicates glitch"
        );
        assert!(y.iter().all(|s| s.is_finite()));
    }
}
