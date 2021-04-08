use std::mem::replace;

use crate::schema::iterate_measures;
use crate::schema::BeatLength;
use crate::schema::BeatPosition;
use crate::schema::ScoreElement;
use crate::schema::ScoreElementKind;
use itertools::iterate;
use itertools::Itertools;
use num::BigRational;

use super::data::ScoreEditorData;
use super::score_editor_widget::ScoreRow;

pub fn cursor_delta_candidates() -> impl DoubleEndedIterator<Item = BeatLength> {
    vec![4, 8, 12, 16, 24, 32]
        .into_iter()
        .map(|x| BeatLength::from(BigRational::new(4.into(), x.into())))
}

pub fn split_into_rows(
    data: &ScoreEditorData,
    max_beat_length_in_row: &BeatLength,
    display_end_beat: &BeatPosition,
) -> Vec<ScoreRow> {
    let mut rows = Vec::new();
    let mut bar_lines = Vec::new();
    let mut row_start_beat = BeatPosition::zero();
    'outer_loop: for (measure_start_beat, measure_end_beat) in
        iterate_measures(data.score.measure_lengths.iter())
    {
        bar_lines.push(measure_start_beat.clone());
        let request_newline = &(&measure_end_beat - &row_start_beat) > max_beat_length_in_row;
        let request_finish = display_end_beat <= &measure_start_beat;
        // If neither of the two condiditions hold, we do not have to do anything now
        if !(request_newline || request_finish) {
            continue;
        }
        // Otherwise, we should output anything before the current measure
        if row_start_beat != measure_start_beat {
            rows.push(ScoreRow::new(
                row_start_beat.clone(),
                measure_start_beat.clone(),
                bar_lines,
            ));
            bar_lines = vec![measure_start_beat.clone()];
        }
        if request_finish {
            break;
        }
        row_start_beat = if &(&measure_end_beat - &measure_start_beat) > max_beat_length_in_row {
            // If the current measure is longer than the upper bound,
            // split the measure into chunks.
            // In this branch, request_newline is always true, so the previous lines has been
            // already flushed.
            for (chunk_start, chunk_end) in
                iterate(measure_start_beat, |beat| beat + &max_beat_length_in_row)
                    .tuple_windows()
                    .take_while(|(start, _)| start <= &measure_end_beat)
            {
                let chunk_end = (&chunk_end).min(&measure_end_beat);
                let mut chunk_bar_lines = replace(&mut bar_lines, Vec::new());
                if chunk_end == &measure_end_beat {
                    chunk_bar_lines.push(measure_end_beat.clone());
                }
                rows.push(ScoreRow::new(
                    chunk_start.clone(),
                    chunk_end.clone(),
                    chunk_bar_lines,
                ));
                if display_end_beat <= chunk_end {
                    break 'outer_loop;
                }
            }
            // Current measure shuold NOT be output in the following process.
            measure_end_beat
        } else {
            // Current measure should be output in the following process.
            measure_start_beat
        }
    }
    rows
}

pub fn append_element(data: &mut ScoreEditorData, kind: ScoreElementKind) {
    let length = data.cursor_delta.to_owned();
    if let Some(track) = data
        .selected_track
        .and_then(|i| data.score.tracks.get_mut(i))
    {
        track.elements.push_back(ScoreElement { kind, length });
        data.cursor_position += &data.cursor_delta;
    }
}
