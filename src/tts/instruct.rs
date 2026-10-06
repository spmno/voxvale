use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Emotion {
    Natural,
    Warm,
    Cheerful,
    Calm,
    Sad,
    Excited,
    Serious,
}

pub const EMOTION_LABELS: [&str; 7] = [
    "自然",
    "温暖",
    "欢快",
    "平静",
    "忧伤",
    "激动",
    "严肃",
];

impl Emotion {
    pub fn from_label(label: &str) -> Self {
        match label {
            "温暖" => Self::Warm,
            "欢快" => Self::Cheerful,
            "平静" => Self::Calm,
            "忧伤" => Self::Sad,
            "激动" => Self::Excited,
            "严肃" => Self::Serious,
            _ => Self::Natural,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Natural => "自然",
            Self::Warm => "温暖",
            Self::Cheerful => "欢快",
            Self::Calm => "平静",
            Self::Sad => "忧伤",
            Self::Excited => "激动",
            Self::Serious => "严肃",
        }
    }

    fn phrase(self) -> &'static str {
        match self {
            Self::Natural => "语气自然亲切",
            Self::Warm => "情绪温暖柔和",
            Self::Cheerful => "情绪欢快明亮",
            Self::Calm => "情绪平静沉稳",
            Self::Sad => "情绪略带忧伤",
            Self::Excited => "情绪饱满激动",
            Self::Serious => "语气认真严肃",
        }
    }
}

impl fmt::Display for Emotion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PauseStyle {
    Natural,
    More,
    Less,
}

pub const PAUSE_LABELS: [&str; 3] = ["自然停顿", "多停顿", "少停顿"];

impl PauseStyle {
    pub fn from_label(label: &str) -> Self {
        match label {
            "多停顿" => Self::More,
            "少停顿" => Self::Less,
            _ => Self::Natural,
        }
    }

    fn phrase(self) -> &'static str {
        match self {
            Self::Natural => "句间停顿自然",
            Self::More => "句间停顿稍长，节奏从容",
            Self::Less => "停顿简短，语流连贯",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VoiceParams {
    pub speed: f32,
    pub emotion: Emotion,
    pub pause: PauseStyle,
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            speed: 1.0,
            emotion: Emotion::Natural,
            pause: PauseStyle::Natural,
        }
    }
}

pub fn speed_phrase(speed: f32) -> &'static str {
    if speed < 0.85 {
        "语速缓慢，吐字从容"
    } else if speed < 0.95 {
        "语速稍慢"
    } else if speed <= 1.08 {
        "语速自然"
    } else if speed <= 1.2 {
        "语速稍快"
    } else {
        "语速很快，节奏紧凑"
    }
}

const CHILD_HINT: &str =
    "声音自然真实，带有自然呼吸与气息，没有机械AI感，适合绘本故事和课文朗读";

pub fn is_child_voice(description: &str) -> bool {
    let d = description.to_lowercase();
    ["童", "孩", "小朋", "岁", "幼", "kid", "child"]
        .iter()
        .any(|k| d.contains(k))
}

pub fn build_voice_instruction(description: &str, params: &VoiceParams) -> String {
    let trimmed = description.trim();
    let desc = trimmed.trim_end_matches([
        '。', '！', '？', '，', '、', '；', '.', '!', '?', ',', ';',
    ]);
    let base = if desc.is_empty() {
        "温柔亲切的自然人声"
    } else {
        desc
    };

    let mut parts = vec![base.to_string()];
    parts.push(params.emotion.phrase().to_string());
    parts.push(params.pause.phrase().to_string());
    parts.push(speed_phrase(params.speed).to_string());

    if is_child_voice(desc) {
        parts.push(CHILD_HINT.to_string());
    }

    parts.join("，")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_punctuation_is_stripped_before_join() {
        let p = VoiceParams::default();
        let inst = build_voice_instruction("11岁男生，声音明亮不尖锐。", &p);
        assert!(inst.contains("不尖锐，语气自然亲切"), "{inst}");
        assert!(!inst.contains("。，"), "{inst}");
    }

    #[test]
    fn child_description_appends_naturalness_hint() {
        let p = VoiceParams::default();
        let inst = build_voice_instruction("6~8岁女童，声音清亮柔和", &p);
        assert!(inst.contains(CHILD_HINT));
        assert!(inst.contains("语速自然"));
        assert!(inst.contains("语气自然亲切"));
    }

    #[test]
    fn adult_description_has_no_child_hint() {
        let p = VoiceParams::default();
        let inst = build_voice_instruction("沉稳的中年男声", &p);
        assert!(!inst.contains(CHILD_HINT));
    }

    #[test]
    fn empty_description_uses_default_voice() {
        let p = VoiceParams::default();
        let inst = build_voice_instruction("  ", &p);
        assert!(inst.starts_with("温柔亲切的自然人声"));
    }

    #[test]
    fn slow_speed_maps_to_slow_phrase() {
        assert_eq!(speed_phrase(0.7), "语速缓慢，吐字从容");
        assert_eq!(speed_phrase(0.9), "语速稍慢");
        assert_eq!(speed_phrase(1.0), "语速自然");
        assert_eq!(speed_phrase(1.1), "语速稍快");
        assert_eq!(speed_phrase(1.3), "语速很快，节奏紧凑");
    }

    #[test]
    fn emotion_and_pause_roundtrip_from_labels() {
        let e = Emotion::from_label("温暖");
        assert_eq!(e, Emotion::Warm);
        let p = PauseStyle::from_label("多停顿");
        assert_eq!(p, PauseStyle::More);

        let params = VoiceParams {
            speed: 0.9,
            emotion: e,
            pause: p,
        };
        let inst = build_voice_instruction("知性女声", &params);
        assert!(inst.contains("情绪温暖柔和"));
        assert!(inst.contains("句间停顿稍长"));
    }
}
