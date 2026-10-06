use anyhow::Result;
use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};
use std::num::NonZeroU32;
use std::sync::Arc;

pub struct AudioPreview {
    _sink: Option<MixerDeviceSink>,
    player: Option<Player>,
}

impl AudioPreview {
    pub fn new() -> Result<Self> {
        let sink = DeviceSinkBuilder::open_default_sink()?;
        let player = Player::connect_new(sink.mixer());
        Ok(Self {
            _sink: Some(sink),
            player: Some(player),
        })
    }

    pub fn dummy() -> Self {
        Self {
            _sink: None,
            player: None,
        }
    }

    pub fn play(&mut self, samples: Arc<Vec<f32>>, sample_rate: u32) -> Result<()> {
        let Some(player) = &self.player else {
            anyhow::bail!("音频输出设备不可用");
        };
        player.stop();
        let data = Arc::try_unwrap(samples).unwrap_or_else(|arc| (*arc).clone());
        player.append(SamplesBuffer::new(
            1.try_into().unwrap(),
            NonZeroU32::new(sample_rate).unwrap_or(NonZeroU32::MIN),
            data,
        ));
        player.play();
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(player) = &self.player {
            player.stop();
        }
    }

    pub fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(|p| !p.empty())
    }
}
