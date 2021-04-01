use std::cmp::Reverse;
use std::collections::binary_heap::PeekMut;
use std::collections::BinaryHeap;
use std::mem::replace;
use std::sync::mpsc;

use crate::audio::AudioCommand;
use crate::audio::AudioManager;
use crate::audio::SoundEffectSchedule;
use crate::bpm_dialog::build_bpm_dialog;
use crate::measure_dialog::build_measure_dialog;
use crate::schema::iterate_beat_times;
use crate::schema::iterate_measures;
use crate::schema::BeatLength;
use crate::schema::BeatPosition;
use crate::schema::Bpm;
use crate::schema::MeasureLength;
use crate::schema::Score;
use crate::schema::ScoreElement;
use crate::schema::ScoreElementKind;
use crate::schema::Track;
use druid::keyboard_types::Key;
use druid::kurbo::Line;
use druid::piet::Text;
use druid::piet::TextLayoutBuilder;
use druid::text::format::ParseFormatter;
use druid::theme::LABEL_COLOR;
use druid::widget::Flex;
use druid::widget::Label;
use druid::widget::Slider;
use druid::widget::TextBox;
use druid::Color;
use druid::Data;
use druid::Env;
use druid::Event;
use druid::EventCtx;
use druid::Insets;
use druid::KeyEvent;
use druid::Lens;
use druid::LifeCycle;
use druid::Modifiers;
use druid::PaintCtx;
use druid::Rect;
use druid::RenderContext;
use druid::Selector;
use druid::SingleUse;
use druid::Widget;
use druid::WidgetExt;
use druid::WindowDesc;
use itertools::iterate;
use itertools::Itertools;
use num::BigRational;
use num::ToPrimitive;
use num::Zero;

use self::layouts::*;

pub fn build_toplevel_widget(audio_manager: AudioManager) -> impl Widget<ScoreEditorData> {
    let status_bar = Flex::row()
        .with_child(
            Label::dynamic(|data: &ScoreEditorData, _| beat_label_string(data)).fix_width(50.0),
        )
        .with_spacer(20.0)
        .with_child(
            Label::dynamic(|len: &BeatLength, _| {
                format!("{}-th note", BigRational::from_integer(4.into()) / &len.0)
            })
            .fix_width(80.0)
            .lens(ScoreEditorData::cursor_delta),
        )
        .with_spacer(20.0)
        .with_child(
            Label::dynamic(|data: &ScoreEditorData, _| {
                let display_time = match &data.music_playback_position {
                    Some(pos) => pos.time,
                    None => data.score.beat_to_time(&data.cursor_position),
                };
                format_time(display_time)
            })
            .fix_width(80.0),
        )
        .with_spacer(20.0)
        .with_child(Label::new("Offset:"))
        .with_child(
            TextBox::new()
                .with_formatter(ParseFormatter::new())
                .update_data_while_editing(true)
                .lens(Score::offset)
                .lens(ScoreEditorData::score),
        )
        .with_spacer(20.0)
        .with_child(Label::new("Music vol:"))
        .with_child(Slider::new().lens(ScoreEditorData::music_volume))
        .with_spacer(5.0)
        .with_child(Label::new("Metronome vol:"))
        .with_child(Slider::new().lens(ScoreEditorData::metronome_volume))
        .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
        .must_fill_main_axis(true)
        .padding(5.0);

    Flex::column()
        .with_child(status_bar)
        .with_child(ScoreEditor {
            audio_manager,
            layout_cache: Vec::new(),
        })
}

fn beat_label_string(data: &ScoreEditorData) -> String {
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

fn format_beat_position(pos: &BeatPosition) -> String {
    let fract = match pos.0.fract() {
        a if a == BigRational::zero() => String::new(),
        a => format!("+{}", a),
    };
    format!("{}{}", pos.0.trunc(), fract)
}

fn format_time(time: f64) -> String {
    let millis = ((time + 0.0005) * 1000.0) as u64;
    format!(
        "{}:{:02}.{:03}",
        millis / 60000,
        millis % 60000 / 1000,
        millis % 1000
    )
}

#[derive(Clone, Debug, Data, Lens)]
pub struct ScoreEditorData {
    score: Score,

    cursor_position: BeatPosition,
    cursor_delta: BeatLength,
    selected_track: Option<usize>,
    playing_music: bool,
    music_volume: f64,
    metronome_volume: f64,

    music_playback_position: Option<MusicPlaybackPositionData>,
}

#[derive(Clone, Debug, Data)]
pub struct MusicPlaybackPositionData {
    time: f64,
    beat: BeatPosition,
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

            music_playback_position: None,
        }
    }
}

struct ScoreEditor {
    audio_manager: AudioManager,
    layout_cache: Vec<ScoreRow>,
}

struct ScoreRow {
    beat_start: BeatPosition,
    beat_end: BeatPosition,
    bar_lines: Vec<BeatPosition>,
    y: f64,
    tracks: Vec<TrackView>,
}

impl ScoreRow {
    fn beat_delta(&self, pos: &BeatPosition) -> BeatLength {
        pos - &self.beat_start
    }

    fn contains_beat(&self, pos: &BeatPosition) -> bool {
        (&self.beat_start..&self.beat_end).contains(&pos)
    }
}

struct TrackView {
    index: usize,
    y: f64,
    beat_start: BeatPosition,
    beat_end: BeatPosition,
}

impl ScoreRow {
    fn new(beat_start: BeatPosition, beat_end: BeatPosition, bar_lines: Vec<BeatPosition>) -> Self {
        Self {
            beat_start,
            beat_end,
            bar_lines,
            y: 0.0,
            tracks: Vec::new(),
        }
    }
}

fn cursor_delta_candidates() -> impl DoubleEndedIterator<Item = BeatLength> {
    vec![4, 8, 12, 16, 24, 32]
        .into_iter()
        .map(|x| BeatLength::from(BigRational::new(4.into(), x.into())))
}

#[derive(Debug)]
pub struct SetMeasureLengthCommand {
    pub position: BeatPosition,
    pub measure_length: Option<MeasureLength>,
}

selector! { EDIT_MEAUSRE_LENGTH_SELECTOR: SingleUse<SetMeasureLengthCommand> }

pub struct SetBpmCommand {
    pub position: BeatPosition,
    pub bpm: Option<Bpm>,
}

selector! { EDIT_BPM_SELECTOR: SingleUse<SetBpmCommand> }

mod layouts {
    use druid::Insets;

    pub(crate) const BEAT_WIDTH: f64 = 60.0;
    pub(crate) const LINE_HEIGHT: f64 = 15.0;
    pub(crate) const NOTE_HEIGHT: f64 = 24.0;
    pub(crate) const NOTE_FULL_HEIGHT: f64 = 32.0;
    pub(crate) const LINE_MARGIN: f64 = 5.0;
    pub(crate) const SCORE_EDITOR_INSETS: Insets = Insets::uniform(-8.0);
}

impl Widget<ScoreEditorData> for ScoreEditor {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut ScoreEditorData, _env: &Env) {
        match event {
            Event::WindowConnected => {
                ctx.request_focus();
            }
            Event::KeyDown(KeyEvent { key, mods, .. }) => match key {
                Key::Character(s) => match s.as_str() {
                    "1" => {
                        append_element(data, ScoreElementKind::Start);
                    }
                    "2" => {
                        append_element(data, ScoreElementKind::Stop);
                    }
                    " " => {
                        if mods.contains(Modifiers::SHIFT) {
                            self.toggle_music_play(ctx, data).unwrap();
                        } else {
                            append_element(data, ScoreElementKind::Skip);
                        }
                    }
                    "a" => {
                        data.score.tracks.push_back(Track {
                            start_beat: data.cursor_position.to_owned(),
                            elements: Default::default(),
                        });
                        data.selected_track = Some(data.score.tracks.len() - 1);
                    }
                    "t" => {
                        let mut candidates = data
                            .score
                            .tracks
                            .iter()
                            .enumerate()
                            .filter_map(|(i, x)| {
                                (x.start_beat()..=&x.end_beat())
                                    .contains(&&data.cursor_position)
                                    .then(|| i)
                            })
                            .peekable();
                        let first = candidates.peek().copied();
                        data.selected_track = if let Some(index) = data.selected_track {
                            if candidates.by_ref().any(|i| i == index) {
                                candidates.next()
                            } else {
                                first
                            }
                        } else {
                            first
                        }
                    }
                    "x" => {
                        data.selected_track.map(|i| data.score.tracks.remove(i));
                    }
                    "m" => self.edit_measure_length(ctx, data),
                    "b" => self.edit_bpm(ctx, data),
                    _ => {}
                },
                Key::Backspace => {
                    if let Some(track) = data
                        .selected_track
                        .and_then(|i| data.score.tracks.get_mut(i))
                    {
                        track.elements.pop_back();
                        data.cursor_position -= &data.cursor_delta;
                    }
                }
                Key::ArrowLeft => {
                    data.cursor_position -= &data.cursor_delta;
                    if data.cursor_position < BeatPosition::zero() {
                        data.cursor_position = BeatPosition::zero();
                    }
                }
                Key::ArrowRight => {
                    data.cursor_position += &data.cursor_delta;
                }
                Key::ArrowUp => {
                    let mut it = cursor_delta_candidates();
                    let first = it.next().unwrap();
                    data.cursor_delta = it
                        .take_while(|x| x > &data.cursor_delta)
                        .last()
                        .unwrap_or(first);
                }
                Key::ArrowDown => {
                    let mut it = cursor_delta_candidates().rev();
                    let first = it.next().unwrap();
                    data.cursor_delta = it
                        .take_while(|x| x < &data.cursor_delta)
                        .last()
                        .unwrap_or(first);
                }
                _ => {}
            },
            Event::MouseDown(..) => ctx.request_focus(),
            Event::Command(command) => {
                if let Some(command) = command
                    .get(EDIT_MEAUSRE_LENGTH_SELECTOR)
                    .and_then(SingleUse::take)
                {
                    match command.measure_length {
                        Some(measure_length) => data
                            .score
                            .measure_lengths
                            .insert(command.position, measure_length),
                        None => data.score.measure_lengths.remove(&command.position),
                    };
                } else if let Some(command) =
                    command.get(EDIT_BPM_SELECTOR).and_then(SingleUse::take)
                {
                    match command.bpm {
                        Some(bpm) => data.score.bpms.insert(command.position, bpm),
                        None => data.score.bpms.remove(&command.position),
                    };
                }
            }
            Event::AnimFrame(..) => {
                if data.playing_music {
                    if let Some(time) = self.audio_manager.playback_position() {
                        if let Some(beat) = BigRational::from_float(data.score.time_to_beat(time)) {
                            let pos = MusicPlaybackPositionData {
                                time,
                                beat: beat.into(),
                            };
                            data.music_playback_position = Some(pos);
                        }
                    }
                    ctx.request_anim_frame();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut druid::LifeCycleCtx,
        event: &LifeCycle,
        data: &ScoreEditorData,
        _env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            self.send_volume(data);
        }
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        old_data: &ScoreEditorData,
        data: &ScoreEditorData,
        _env: &druid::Env,
    ) {
        if !old_data.same(data) {
            ctx.request_layout();
            ctx.request_paint();
        }
        if old_data.music_volume != data.music_volume
            || old_data.metronome_volume != data.metronome_volume
        {
            self.send_volume(data);
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &ScoreEditorData,
        _env: &druid::Env,
    ) -> druid::Size {
        let max_beat_length_in_row = BigRational::from_float(
            ((bc.max().width + SCORE_EDITOR_INSETS.x_value()) / BEAT_WIDTH).trunc(),
        )
        .map(BeatLength::from)
        .max(Some(BeatLength::one()))
        .unwrap();

        let display_end_beat = data
            .score
            .tracks
            .iter()
            .map(|x| x.end_beat())
            .max()
            .as_ref()
            .max(data.music_playback_position.as_ref().map(|p| &p.beat))
            .unwrap_or_else(|| &data.cursor_position)
            .max(&data.cursor_position)
            + &BeatLength::one();

        self.layout_cache = split_into_rows(data, max_beat_length_in_row, display_end_beat);

        struct Wrap<'a>(usize, &'a Track);
        impl PartialEq for Wrap<'_> {
            fn eq(&self, other: &Self) -> bool {
                self.cmp(other) == std::cmp::Ordering::Equal
            }
        }
        impl PartialOrd for Wrap<'_> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Eq for Wrap<'_> {}
        impl Ord for Wrap<'_> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.1.start_beat().cmp(other.1.start_beat()).reverse()
            }
        }
        let mut track_queue: BinaryHeap<_> = data
            .score
            .tracks
            .iter()
            .enumerate()
            .map(|(a, b)| Wrap(a, b))
            .collect();
        let mut shown_track = Vec::new();

        let mut y = 0.0;

        for row in self.layout_cache.iter_mut() {
            row.y = y;
            y += LINE_HEIGHT;

            while let Some(track) = track_queue
                .peek_mut()
                .and_then(|t| (t.1.start_beat() < &row.beat_end).then(|| PeekMut::pop(t)))
            {
                shown_track.push(track);
            }
            shown_track.retain(|t| row.beat_start < t.1.end_beat());

            let mut available_slots = BinaryHeap::<Reverse<usize>>::new();
            let mut end_queue = BinaryHeap::<Reverse<(BeatPosition, usize)>>::new();
            for &Wrap(index, track) in shown_track.iter() {
                let beat_start = track.start_beat().clone().max(row.beat_start.clone());
                let beat_end = track.end_beat().min(row.beat_end.clone());
                while let Some(popped) = end_queue
                    .peek_mut()
                    .and_then(|p| (p.0 .0 <= beat_start).then(|| PeekMut::pop(p)))
                {
                    available_slots.push(Reverse(popped.0 .1));
                }
                let slot = available_slots.pop().map_or(end_queue.len(), |x| x.0);
                end_queue.push(Reverse((beat_end.clone(), slot)));
                row.tracks.push(TrackView {
                    index,
                    y: y + slot as f64 * NOTE_FULL_HEIGHT,
                    beat_start,
                    beat_end,
                });
            }
            y += (available_slots.len() + end_queue.len()) as f64 * NOTE_FULL_HEIGHT + LINE_MARGIN;
        }
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &ScoreEditorData, env: &druid::Env) {
        let draw_rect = ctx.size().to_rect().inset(SCORE_EDITOR_INSETS); // .inset(status_bar_inset);
        let get_x =
            |length: BeatLength| draw_rect.min_x() + length.0.to_f64().unwrap() * BEAT_WIDTH;

        let mut measure_lengths = data.score.measure_lengths.iter().peekable();
        let mut bpms = data.score.bpms.iter().peekable();

        for row in self.layout_cache.iter() {
            let get_x = |pos: &BeatPosition| get_x(row.beat_delta(pos));

            // Draw bar lines at the first of each measure
            for beat in row.bar_lines.iter() {
                let x = get_x(&beat);
                let line = Line::new((x, row.y), (x, row.y + LINE_HEIGHT));
                ctx.stroke(line, &Color::GRAY, 2.0);
            }

            // Draw the other bar lines
            for (start, end) in row.bar_lines.iter().tuple_windows() {
                for beat in iterate(start.clone(), |x| x + &BeatLength::one())
                    .skip(1)
                    .take_while(|b| b < &end)
                {
                    let x = get_x(&beat);
                    let line = Line::new((x, row.y + 2.0), (x, row.y + LINE_HEIGHT));
                    ctx.stroke(line, &Color::GRAY, 1.0);
                }
            }

            // Draw curosr
            if row.contains_beat(&data.cursor_position) {
                draw_cursor(ctx, get_x, &data.cursor_position, row.y);
            }
            // Draw music playback cursor
            if let Some(beat) = data.music_playback_position.as_ref().map(|p| &p.beat) {
                if row.contains_beat(&beat) {
                    draw_cursor(ctx, get_x, &beat, row.y);
                }
            }

            // Draw tracks
            for track_view in row.tracks.iter() {
                let track = &data.score.tracks[track_view.index];
                draw_track(
                    ctx,
                    get_x,
                    row,
                    track_view,
                    track,
                    data.selected_track.map_or(false, |j| track_view.index == j),
                    &draw_rect,
                );
            }

            // draw measure labels
            for (beat, measure) in measure_lengths.peeking_take_while(|(b, _)| row.contains_beat(b))
            {
                let layout = ctx
                    .text()
                    .new_text_layout(format!("{}", measure))
                    .text_color(env.get(LABEL_COLOR))
                    .build();
                match layout {
                    Ok(layout) => ctx.draw_text(&layout, (get_x(beat), row.y)),
                    Err(e) => eprintln!("{}", e),
                }
            }

            // draw beat labels
            for (beat, bpm) in bpms.peeking_take_while(|(b, _)| row.contains_beat(b)) {
                // TODO duplicates?
                let layout = ctx
                    .text()
                    .new_text_layout(format!("{:.2}", bpm.0))
                    .text_color(env.get(LABEL_COLOR))
                    .build();
                match layout {
                    Ok(layout) => ctx.draw_text(&layout, (get_x(beat), row.y)),
                    Err(e) => eprintln!("{}", e),
                }
            }
        }
    }
}

fn split_into_rows(
    data: &ScoreEditorData,
    max_beat_length_in_row: BeatLength,
    display_end_beat: BeatPosition,
) -> Vec<ScoreRow> {
    let mut rows = Vec::new();
    let mut bar_lines = Vec::new();
    let mut row_start_beat = BeatPosition::zero();
    'outer_loop: for (measure_start_beat, measure_end_beat) in
        iterate_measures(data.score.measure_lengths.iter())
    {
        bar_lines.push(measure_start_beat.clone());
        let request_newline = &measure_end_beat - &row_start_beat > max_beat_length_in_row;
        let request_finish = &display_end_beat <= &measure_start_beat;
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
        row_start_beat = if &measure_end_beat - &measure_start_beat > max_beat_length_in_row {
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
                if &display_end_beat <= chunk_end {
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

impl ScoreEditor {
    fn send_volume(&self, data: &ScoreEditorData) {
        self.audio_manager
            .command_sender()
            .send(AudioCommand::SetVolume(data.music_volume))
            .unwrap();
        self.audio_manager
            .command_sender()
            .send(AudioCommand::SetSoundEffectVolume(data.metronome_volume))
            .unwrap();
    }

    fn edit_measure_length(&self, ctx: &mut EventCtx, data: &ScoreEditorData) {
        let cursor_position = data.cursor_position.to_owned();
        let (already_exsits, current_measure_length) =
            match data.score.measure_lengths.range(..=&cursor_position).last() {
                None => (false, BeatLength::four()),
                Some((k, v)) => (k == &cursor_position, v.to_owned().into()),
            };
        let widget_id = ctx.widget_id();
        let window_desc = WindowDesc::new(move || {
            build_measure_dialog::<ScoreEditorData>(
                widget_id,
                cursor_position.to_owned(),
                current_measure_length.into(),
                already_exsits,
            )
        });
        ctx.new_window(window_desc);
    }

    fn edit_bpm(&self, ctx: &mut EventCtx, data: &ScoreEditorData) {
        let cursor_position = data.cursor_position.to_owned();
        let (already_exsits, current_bpm) = match data.score.bpms.range(..=&cursor_position).last()
        {
            None => (false, Bpm::default()),
            Some((k, v)) => (k == &cursor_position, *v),
        };
        let widget_id = ctx.widget_id();
        let window_desc = WindowDesc::new(move || {
            build_bpm_dialog::<ScoreEditorData>(
                widget_id,
                cursor_position.to_owned(),
                current_bpm,
                already_exsits,
            )
        });
        ctx.new_window(window_desc);
    }

    fn toggle_music_play(
        &self,
        ctx: &mut EventCtx,
        data: &mut ScoreEditorData,
    ) -> Result<(), mpsc::SendError<impl std::any::Any>> {
        let sender = self.audio_manager.command_sender();
        if data.playing_music {
            sender.send(AudioCommand::Pause)?;
            data.playing_music = false;
            data.music_playback_position = None;
        } else {
            let pos = data.score.beat_to_time(&data.cursor_position);
            sender.send(AudioCommand::Seek(pos))?;
            sender.send(AudioCommand::SetSoundEffectSchedules(Box::new(
                iterate_beat_times(
                    data.score.offset,
                    data.score.measure_lengths.clone(),
                    data.score.bpms.clone(),
                    data.cursor_position.clone(),
                )
                .map(|(first, time)| SoundEffectSchedule {
                    time,
                    frequency: if first { 1244.51 } else { 739.99 },
                }),
            )))?;
            sender.send(AudioCommand::Play)?;
            data.playing_music = true;
            ctx.request_anim_frame();
        }
        Ok(())
    }
}

fn append_element(data: &mut ScoreEditorData, kind: ScoreElementKind) {
    let length = data.cursor_delta.to_owned();
    if let Some(track) = data
        .selected_track
        .and_then(|i| data.score.tracks.get_mut(i))
    {
        track.elements.push_back(ScoreElement { kind, length });
        data.cursor_position += &data.cursor_delta;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_track(
    ctx: &mut PaintCtx,
    get_x: impl Fn(&BeatPosition) -> f64,
    row: &ScoreRow,
    track_view: &TrackView,
    track: &Track,
    selected: bool,
    draw_rect: &Rect,
) {
    let track_end_beat = track.end_beat();
    ctx.with_save(|ctx| {
        let rect = Rect::new(
            draw_rect.min_x(),
            track_view.y,
            get_x(&row.beat_end),
            track_view.y + NOTE_FULL_HEIGHT,
        );
        ctx.clip(rect);
        let rect = rect.inset(Insets::uniform_xy(
            0.0,
            (NOTE_HEIGHT - NOTE_FULL_HEIGHT) / 2.0,
        ));

        {
            let min_x = if track.start_beat() < &row.beat_start {
                rect.min_x() - 20.0
            } else {
                get_x(track.start_beat())
            };
            let max_x = if &row.beat_end < &track_end_beat {
                rect.max_x() + 20.0
            } else {
                get_x(&track_end_beat)
            };
            let rect = Rect::new(min_x, rect.min_y(), max_x, rect.max_y()).to_rounded_rect(4.0);
            let (fill_brush, stroke_brush) = match selected {
                false => (Color::rgb8(0, 66, 19), Color::rgb8(0, 46, 13)),
                true => (Color::rgb8(66, 0, 69), Color::rgb8(32, 0, 46)),
            };
            ctx.fill(rect, &fill_brush);
            ctx.stroke(rect, &stroke_brush, 3.0);
        }

        let rect = rect.inset(Insets::uniform_xy(0.0, -6.0));

        for (note_start_beat, note_end_beat, _) in track.iterate_notes() {
            if &note_end_beat < &row.beat_start || &row.beat_end < &note_start_beat {
                continue;
            }
            let rect = Rect::new(
                get_x(&note_start_beat),
                rect.min_y(),
                get_x(&note_end_beat),
                rect.max_y(),
            )
            .to_rounded_rect(5.0);
            ctx.fill(rect, &Color::rgb8(172, 255, 84));
        }
    });
}

fn draw_cursor(
    ctx: &mut PaintCtx,
    get_x: impl Fn(&BeatPosition) -> f64,
    cursor_position: &BeatPosition,
    min_y: f64,
) {
    let x = get_x(cursor_position);
    let line = Line::new((x, min_y), (x, min_y + LINE_HEIGHT));
    ctx.stroke(line, &Color::GREEN, 3.0);
}
