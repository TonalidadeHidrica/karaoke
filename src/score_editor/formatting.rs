use crate::schema::BeatLength;
use crate::schema::BeatPosition;
use num_rational::BigRational;
use num_traits::ToPrimitive;
use num_traits::Zero;

use super::data::ScoreEditorData;

pub fn beat_label_string(data: &ScoreEditorData) -> String {
    let (playing, pos) = match data.music_playback_position.as_ref() {
        Some(pos) => (true, &pos.beat),
        None => (false, &data.cursor_position),
    };
    let (start_beat, len) = match data.score.measure_lengths.range(..=pos).next_back() {
        Some((a, b)) => (a.to_owned(), b.to_owned().into()),
        None => (BeatPosition::zero(), BeatLength::four()),
    };
    let delta_beat = (pos - &start_beat).0;
    let measure_index = (&delta_beat / &len.0).trunc();
    let beat = &delta_beat - &measure_index * &len.0;
    let fraction_str = if playing {
        format!("{:.0}", beat.to_f64().unwrap().trunc())
    } else {
        format_beat_position(&BeatPosition::from(beat))
    };
    format!("{}:{}", measure_index, fraction_str)
}

pub fn format_beat_position(pos: &BeatPosition) -> String {
    let fract = match pos.0.fract() {
        a if a == BigRational::zero() => String::new(),
        a => format!("+{}", a),
    };
    format!("{}{}", pos.0.trunc(), fract)
}

pub fn format_time(time: f64) -> String {
    let millis = ((time + 0.0005) * 1000.0) as u64;
    format!(
        "{}:{:02}.{:03}",
        millis / 60000,
        millis % 60000 / 1000,
        millis % 1000
    )
}
