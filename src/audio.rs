use std::collections::VecDeque;
use std::io::BufReader;
use std::iter;
use std::iter::Peekable;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::time::Instant;

use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use cpal::OutputCallbackInfo;
use cpal::Sample;
use cpal::Stream;
use cpal::StreamConfig;
use dasp::signal;
use dasp::signal::ConstHz;
use dasp::signal::Delay;
use dasp::signal::FromIterator;
use dasp::signal::ScaleAmp;
use dasp::signal::Sine;
use dasp::signal::Take;
use dasp::Signal;
use derive_getters::Getters;
use fs_err::File;
use rodio::Decoder;
use tokio::sync::watch;
use universal_audio_decoder::new_uniform_source_iterator;
use universal_audio_decoder::TrueUniformSourceIterator;

use crate::dasp_signal_ext::Multiplexed;
use crate::dasp_signal_ext::SignalExt;
use crate::error::AudioError;

#[derive(Getters)]
pub struct AudioManager {
    #[getter(skip)]
    _stream: Stream,
    command_sender: mpsc::Sender<AudioCommand>,
    state_receiver: watch::Receiver<AudioState>,
}

pub enum AudioCommand {
    Play,
    Pause,
    Seek(f64),
    LoadMusic(PathBuf),

    SetVolume(f64),

    SetSoundEffectSchedules(SESchedulesBox),
    SetSoundEffectVolume(f64),
}

pub enum AudioState {
    NotPlaying,
    Playing {
        instant: Instant,
        music_position: f64,
    },
}

impl AudioManager {
    pub fn new() -> Result<Self, AudioError> {
        let (command_sender, command_receiver) = mpsc::channel();
        let (state_sender, state_receiver) = watch::channel(AudioState::NotPlaying);
        let _stream = Self::build_stream(command_receiver, state_sender)?;
        let manager = AudioManager {
            _stream,
            command_sender,
            state_receiver,
        };
        Ok(manager)
    }

    fn build_stream(
        command_receiver: mpsc::Receiver<AudioCommand>,
        state_sender: watch::Sender<AudioState>,
    ) -> Result<Stream, AudioError> {
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

        let callback =
            AudioOutputCallback::new(stream_config.clone(), command_receiver, state_sender);
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

impl AudioManager {
    pub fn playback_position(&self) -> Option<f64> {
        use AudioState::*;
        match *self.state_receiver.borrow() {
            NotPlaying => None,
            Playing {
                instant,
                music_position,
            } => {
                let play_speed = 1.0;
                let now = Instant::now();
                let diff = if now > instant {
                    (now - instant).as_secs_f64()
                } else {
                    -(instant - now).as_secs_f64()
                };
                Some(music_position + diff * play_speed)
            }
        }
    }
}

type MusicSource = TrueUniformSourceIterator<Decoder<BufReader<File>>>;
type SoundEffect = Multiplexed<Delay<FromIterator<Take<ScaleAmp<Sine<ConstHz>>>>>>;

struct AudioOutputCallback {
    output_stream_config: StreamConfig,
    command_receiver: mpsc::Receiver<AudioCommand>,
    state_sender: watch::Sender<AudioState>,

    music: Option<MusicSource>,
    playing: bool,
    music_volume: f64,

    playback_time: f64,

    sound_effect_schedules: Peekable<SESchedulesBox>,
    sound_effects: VecDeque<SoundEffect>,
    sound_effect_volume: f64,
}

#[derive(Debug)]
pub struct SoundEffectSchedule {
    pub time: f64,
    pub frequency: f64,
}
pub type SESchedulesBox = Box<dyn Iterator<Item = SoundEffectSchedule> + Send>;

impl AudioOutputCallback {
    fn new(
        output_stream_config: StreamConfig,
        command_receiver: mpsc::Receiver<AudioCommand>,
        state_sender: watch::Sender<AudioState>,
    ) -> Self {
        Self {
            output_stream_config,
            command_receiver,
            state_sender,

            music: None,
            playing: false,
            music_volume: 0.0,

            playback_time: 0.0,

            sound_effect_schedules: Self::empty_schedules(),
            sound_effects: VecDeque::new(),
            sound_effect_volume: 0.0,
        }
    }
}

impl AudioOutputCallback {
    fn callback<S>(&mut self, out: &mut [S], callback_info: &OutputCallbackInfo)
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

        self.refresh_state(callback_info);
        let playback_end = self.playback_time
            + if self.playing {
                1.0 / self.output_stream_config.sample_rate.0 as f64 * out.len() as f64
                    / self.output_stream_config.channels as f64
            } else {
                0.0
            };

        while let Some(next) = self.sound_effect_schedules.peek() {
            if playback_end < next.time {
                break;
            }
            let next = self.sound_effect_schedules.next().expect("Always exists");
            let wave = signal::from_iter(
                signal::rate(self.output_stream_config.sample_rate.0 as _)
                    .const_hz(next.frequency)
                    .sine()
                    .scale_amp(self.sound_effect_volume)
                    .take(self.output_stream_config.sample_rate.0 as usize / 20), // 0.05 seconds
            )
            .delay(
                ((next.time - self.playback_time).max(0.0)
                    * self.output_stream_config.sample_rate.0 as f64) as _,
            )
            .multiplexed(self.output_stream_config.channels as _);
            self.sound_effects.push_back(wave);
        }

        self.sound_effects.retain(|x| !x.is_exhausted());

        for out in out.iter_mut() {
            let next = match &mut self.music {
                Some(music) if self.playing => music.next(),
                _ => None,
            };
            let mut next = next.unwrap_or(0.0);
            next *= self.music_volume as f32;
            next += self.sound_effects.iter_mut().map(|x| x.next()).sum::<f64>() as f32;
            let next = next.clamp(-1.5, 1.5); // Prevent too large sound
            *out = S::from(&next);
        }

        self.playback_time = playback_end;
    }

    fn refresh_state(&self, callback_info: &OutputCallbackInfo) {
        let state = if self.playing {
            let pb = &callback_info.timestamp().playback;
            let cb = &callback_info.timestamp().callback;
            let instant = Instant::now() + pb.duration_since(cb).unwrap_or_default();
            AudioState::Playing {
                instant,
                music_position: self.playback_time,
            }
        } else {
            AudioState::NotPlaying
        };
        let _ = self.state_sender.send(state);
    }

    fn process_command(&mut self, command: AudioCommand) {
        use AudioCommand::*;
        match command {
            Play => self.playing = true,
            Pause => self.playing = false,
            Seek(time) => {
                // TODO negative seek
                self.sound_effect_schedules = Self::empty_schedules();
                self.sound_effects.clear();
                self.playback_time = time;
                if let Some(music) = &mut self.music {
                    music.seek(time.max(0.0)).unwrap();
                }
                self.playing = false;
            }
            LoadMusic(path) => {
                if let Err(e) = self.load_music(path) {
                    eprintln!("{}", e);
                }
            }
            SetVolume(vol) => self.music_volume = vol,
            SetSoundEffectSchedules(schedules) => {
                self.sound_effect_schedules = schedules.peekable()
            }
            SetSoundEffectVolume(vol) => self.sound_effect_volume = vol,
        };
    }

    fn load_music(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let file = fs_err::File::open(path)?;
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

    fn empty_schedules() -> Peekable<SESchedulesBox> {
        let ret: SESchedulesBox = Box::new(iter::empty());
        ret.peekable()
    }
}
