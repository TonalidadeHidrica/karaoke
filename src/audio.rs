use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::mpsc::TryRecvError;

use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use cpal::OutputCallbackInfo;
use cpal::Sample;
use cpal::Stream;
use cpal::StreamConfig;
use derive_getters::Getters;
use rodio::Decoder;
use universal_audio_decoder::new_uniform_source_iterator;
use universal_audio_decoder::TrueUniformSourceIterator;

use crate::error::AudioError;

#[derive(Getters)]
pub struct AudioManager {
    #[getter(skip)]
    _stream: Stream,
    command_sender: Sender<AudioCommand>,
}

pub enum AudioCommand {
    Play,
    Pause,
    // Seek(f64),
    LoadMusic(PathBuf),
}

impl AudioManager {
    pub fn new() -> Result<Self, AudioError> {
        let (command_sender, command_receiver) = mpsc::channel();
        let _stream = Self::build_stream(command_receiver)?;
        let manager = AudioManager {
            _stream,
            command_sender,
        };
        Ok(manager)
    }

    fn build_stream(command_receiver: Receiver<AudioCommand>) -> Result<Stream, AudioError> {
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
        let stream_config = StreamConfig::from(supported_config);

        let callback = AudioOutputCallback::new(stream_config.clone(), command_receiver);
        let error_callback = |err| eprintln!("an error occurred on audio stream: {:?}", err);
        let stream = {
            use cpal::SampleFormat::*;
            let (sc, ec) = (&stream_config, error_callback);
            match sample_format {
                I16 => device.build_output_stream(sc, callback.into_callback::<i16>(), ec),
                U16 => device.build_output_stream(sc, callback.into_callback::<u16>(), ec),
                F32 => device.build_output_stream(sc, callback.into_callback::<f32>(), ec),
            }
        }?;
        stream.play()?;
        Ok(stream)
    }
}

type MusicSource = TrueUniformSourceIterator<Decoder<BufReader<File>>>;

struct AudioOutputCallback {
    output_stream_config: StreamConfig,
    command_receiver: Receiver<AudioCommand>,

    music: Option<MusicSource>,
    playing: bool,
}

impl AudioOutputCallback {
    fn new(output_stream_config: StreamConfig, command_receiver: Receiver<AudioCommand>) -> Self {
        Self {
            output_stream_config,
            command_receiver,
            music: None,
            playing: false,
        }
    }
}

impl AudioOutputCallback {
    fn callback<S>(&mut self, out: &mut [S], _info: &OutputCallbackInfo)
    where
        S: Sample,
    {
        while let Some(command) = match self.command_receiver.try_recv() {
            Ok(command) => Some(command),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => panic!("The main thread has stopped"),
        } {
            self.process_command(command);
        }
        for out in out.iter_mut() {
            let next = match &mut self.music {
                Some(music) if self.playing => music.next(),
                _ => None,
            };
            *out = S::from(&next.unwrap_or(0.0));
        }
    }

    fn process_command(&mut self, command: AudioCommand) {
        match command {
            AudioCommand::Play => self.playing = true,
            AudioCommand::Pause => self.playing = false,
            AudioCommand::LoadMusic(path) => {
                if let Err(e) = self.load_music(path) {
                    eprintln!("{}", e);
                }
            }
        };
    }

    fn load_music(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let file = std::fs::File::open(path)?;
        let decoder = rodio::Decoder::new(BufReader::new(file))?;
        let ret = new_uniform_source_iterator(decoder, &self.output_stream_config);
        self.music = Some(ret);
        Ok(())
    }

    fn into_callback<S>(mut self) -> impl FnMut(&mut [S], &OutputCallbackInfo) + Send + 'static
    where
        S: Sample,
    {
        move |a, b| self.callback(a, b)
    }
}
