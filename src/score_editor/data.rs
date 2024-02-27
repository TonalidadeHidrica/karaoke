use std::path::PathBuf;

use super::bpm_detector::BpmDetectorData;
use crate::schema::BeatLength;
use crate::schema::BeatPosition;
use crate::schema::Score;
use derive_new::new;
use druid::text::Selection;
use druid::Data;
use druid::Lens;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, new, Data, Lens)]
pub struct ScoreEditorData {
    pub score: Score,
    #[serde(skip)]
    #[data(eq)]
    #[new(default)]
    pub save_path: PathBuf,

    #[new(value = "BeatPosition::zero()")]
    pub cursor_position: BeatPosition,
    #[new(value = "BeatLength::one()")]
    pub cursor_delta: BeatLength,
    #[new(default)]
    pub selected_track: Option<usize>,
    #[new(default)]
    pub playing_music: bool,
    #[new(value = "0.4")]
    pub music_volume: f64,
    #[new(value = "0.4")]
    pub metronome_volume: f64,
    #[serde(skip)]
    #[new(default)]
    pub bpm_detector_data: BpmDetectorData,
    #[serde(skip)]
    #[new(default)]
    #[data(eq)]
    pub selection: Option<Selection>,

    #[new(default)]
    pub music_playback_position: Option<MusicPlaybackPositionData>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Data)]
pub struct MusicPlaybackPositionData {
    pub time: f64,
    pub beat: BeatPosition,
}

// impl Default for ScoreEditorData {
//     fn default() -> Self {
//         ScoreEditorData {
//             score: Score::default(),
//
//             cursor_position: BeatPosition::zero(),
//             cursor_delta: BeatLength::one(),
//             selected_track: None,
//             playing_music: false,
//             music_volume: 0.4,
//             metronome_volume: 0.4,
//             bpm_detector_data: BpmDetectorData::default(),
//             selection: None,
//
//             music_playback_position: None,
//         }
//     }
// }
