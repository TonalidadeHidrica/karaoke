use super::bpm_detector::BpmDetectorData;
use crate::schema::BeatLength;
use crate::schema::BeatPosition;
use crate::schema::Score;
use druid::text::Selection;
use druid::Data;
use druid::Lens;

#[derive(Clone, Debug, Data, Lens)]
pub struct ScoreEditorData {
    pub score: Score,

    pub cursor_position: BeatPosition,
    pub cursor_delta: BeatLength,
    pub selected_track: Option<usize>,
    pub playing_music: bool,
    pub music_volume: f64,
    pub metronome_volume: f64,
    pub bpm_detector_data: BpmDetectorData,
    #[data(same_fn = "PartialEq::eq")]
    pub selection: Option<Selection>,

    pub music_playback_position: Option<MusicPlaybackPositionData>,
}

#[derive(Clone, Debug, Data)]
pub struct MusicPlaybackPositionData {
    pub time: f64,
    pub beat: BeatPosition,
}

impl Default for ScoreEditorData {
    fn default() -> Self {
        ScoreEditorData {
            score: Score::default(),

            cursor_position: BeatPosition::zero(),
            cursor_delta: BeatLength::one(),
            selected_track: None,
            playing_music: false,
            music_volume: 0.4,
            metronome_volume: 0.4,
            bpm_detector_data: BpmDetectorData::default(),
            selection: None,

            music_playback_position: None,
        }
    }
}
