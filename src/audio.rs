use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use cpal::OutputCallbackInfo;
use cpal::Sample;
use cpal::StreamConfig;

use crate::error::AudioError;

pub fn start_audio_thread() -> Result<cpal::Stream, AudioError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(AudioError::WithMessage("No default output device found"))?;
    let supported_config = device
        .supported_output_configs()?
        .next()
        .ok_or(AudioError::WithMessage(
            "No audio configuration is available",
        ))?
        .with_max_sample_rate();
    let sample_format = supported_config.sample_format();
    dbg!(sample_format);
    let stream_config = StreamConfig::from(supported_config);

    let callback = AudioOutputCallback::default();
    let error_callback = |err| eprintln!("an error occurred on audio stream: {:?}", err);
    let stream = device.build_output_stream(
        &stream_config,
        callback.into_callback::<f32>(),
        error_callback,
    )?;
    stream.play()?;
    Ok(stream)
}

#[derive(Default)]
struct AudioOutputCallback {
    i: usize,
}

impl AudioOutputCallback {
    fn callback<S>(&mut self, samples: &mut [S], _info: &OutputCallbackInfo)
    where
        S: Sample,
    {
        for (i, out) in samples.iter_mut().enumerate() {
            let next = (self.i as f32 / 44100.0 * 440.0 * std::f32::consts::TAU).sin();
            *out = S::from(&next);
            if i % 2 == 1 {
                self.i += 1;
            }
        }
    }

    fn into_callback<S>(mut self) -> impl FnMut(&mut [S], &OutputCallbackInfo) + Send + 'static
    where
        S: Sample,
    {
        move |a, b| self.callback(a, b)
    }
}
