use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::binary_heap::PeekMut;
use std::collections::BinaryHeap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::mpsc;

use crate::audio::AudioCommand;
use crate::audio::AudioManager;
use crate::audio::SoundEffectSchedule;
use crate::fonts::FontLoader;
use crate::schema::iterate_beat_times;
use crate::schema::BeatLength;
use crate::schema::BeatPosition;
use crate::schema::Bpm;
use crate::schema::Lyrics;
use crate::schema::MeasureLength;
use crate::schema::ScoreElementKind;
use crate::schema::Track;
use druid::im::OrdMap;
use druid::im::Vector;
use druid::keyboard_types::Key;
use druid::kurbo::Line;
use druid::piet::IntoBrush;
use druid::piet::Piet;
use druid::piet::Text;
use druid::piet::TextLayoutBuilder;
use druid::theme::TEXT_COLOR;
use druid::Color;
use druid::Data;
use druid::Env;
use druid::Event;
use druid::EventCtx;
use druid::Insets;
use druid::KeyEvent;
use druid::LifeCycle;
use druid::Modifiers;
use druid::MouseEvent;
use druid::PaintCtx;
use druid::Rect;
use druid::RenderContext;
use druid::SingleUse;
use druid::Size;
use druid::Widget;
use druid::WidgetExt;
use druid::WindowDesc;
use itertools::iterate;
use itertools::Itertools;
use num::BigRational;
use num::ToPrimitive;

use super::bpm_detector::build_bpm_detector_widget;
use super::bpm_dialog::build_bpm_dialog;
use super::commands::EDIT_BPM_SELECTOR;
use super::commands::EDIT_MEAUSRE_LENGTH_SELECTOR;
use super::data::MusicPlaybackPositionData;
use super::data::ScoreEditorData;
use super::layouts::*;
use super::lyrics_editor::SET_LYRICS_RANGE;
use super::lyrics_editor::UPDATE_SELECTION_SELECTOR;
use super::lyrics_mapping_dialog::build_lyrics_mapping_dialog;
use super::measure_dialog::build_measure_dialog;
use super::misc::append_element;
use super::misc::cursor_delta_candidates;
use super::misc::split_into_rows;

pub struct ScoreEditor {
    pub(super) audio_manager: AudioManager,
    pub font_loader: Rc<RefCell<FontLoader>>,
    pub(super) layout_cache: Vec<ScoreRow>,
    pub(super) hover_cursor: Option<BeatPosition>,
}

pub struct ScoreRow {
    pub beat_start: BeatPosition,
    pub beat_end: BeatPosition,
    pub bar_lines: Vec<BeatPosition>,
    pub y: f64,
    pub y_max: f64,
    pub tracks: Vec<TrackView>,
}

impl ScoreRow {
    pub fn new(
        beat_start: BeatPosition,
        beat_end: BeatPosition,
        bar_lines: Vec<BeatPosition>,
    ) -> Self {
        Self {
            beat_start,
            beat_end,
            bar_lines,
            y: 0.0,
            y_max: 0.0,
            tracks: Vec::new(),
        }
    }

    pub fn beat_delta(&self, pos: &BeatPosition) -> BeatLength {
        pos - &self.beat_start
    }

    pub fn contains_beat(&self, pos: &BeatPosition) -> bool {
        (&self.beat_start..&self.beat_end).contains(&pos)
    }

    pub fn y_range(&self) -> Range<f64> {
        self.y..self.y_max
    }
}

pub struct TrackView {
    index: usize,
    y: f64,
    _beat_start: BeatPosition,
    _beat_end: BeatPosition,
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
                            lyrics: None,
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
                    "B" => self.open_bpm_detector(ctx),
                    "/" => {
                        if let Some(time) = self.audio_manager.playback_position() {
                            data.bpm_detector_data.push(time);
                        }
                    }
                    "l" => self.edit_lyrics_mapping(ctx, data),
                    "L" => {
                        // Remove lyrics
                        if let Some(i) = data.selected_track {
                            data.score.tracks[i].lyrics = None;
                        }
                    }
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
            Event::MouseMove(event) => {
                let hover_cursor = self.handle_mouse_move(data, event);
                if self.hover_cursor != hover_cursor {
                    ctx.request_paint();
                }
                self.hover_cursor = hover_cursor;
            }
            Event::MouseDown(..) => {
                ctx.request_focus();
                if let Some(beat) = &self.hover_cursor {
                    data.cursor_position = beat.clone();
                }
            }
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
                } else if let Some(selection) = command.get(UPDATE_SELECTION_SELECTOR) {
                    data.selection = selection.to_owned();
                } else if let Some(()) = command.get(SET_LYRICS_RANGE) {
                    if let (Some(track), Some(selection)) = (data.selected_track, data.selection) {
                        let (s, t) = (selection.anchor, selection.active);
                        let (s, t) = (s.min(t), s.max(t));
                        if s < t {
                            data.score.tracks[track].lyrics = Some(Lyrics {
                                text: data.score.lyrics[s..t].to_owned(),
                                mappings: OrdMap::new(),
                            });
                        }
                    }
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
            self.hover_cursor = None;
            ctx.request_layout();
            ctx.request_paint();
        }
        #[allow(clippy::float_cmp)]
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
        let max_beat_length_in_row = BigRational::from_float((bc.max().width / BEAT_WIDTH).trunc())
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
            .unwrap_or(&data.cursor_position)
            .max(&data.cursor_position)
            + &BeatLength::one();

        self.layout_cache = split_into_rows(data, &max_beat_length_in_row, &display_end_beat);

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

        for (i, row) in self.layout_cache.iter_mut().enumerate() {
            if i > 0 {
                y += LINE_MARGIN;
            }

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
                    _beat_start: beat_start,
                    _beat_end: beat_end,
                });
            }
            y += (available_slots.len() + end_queue.len()) as f64 * NOTE_FULL_HEIGHT;
            row.y_max = y;
        }

        // let width = max_beat_length_in_row.0.to_f64().unwrap() * BEAT_WIDTH;
        let width = bc.max().width;
        let height = y;
        Size::new(width, height)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &ScoreEditorData, env: &druid::Env) {
        let draw_rect = ctx.size().to_rect();
        let get_x =
            |length: BeatLength| draw_rect.min_x() + length.0.to_f64().unwrap() * BEAT_WIDTH;

        let mut measure_lengths = data.score.measure_lengths.iter().peekable();
        let mut bpms = data.score.bpms.iter().peekable();

        for row in self.layout_cache.iter() {
            let get_x = |pos: &BeatPosition| get_x(row.beat_delta(pos));

            // Draw bar lines at the first of each measure
            for beat in row.bar_lines.iter() {
                let x = get_x(beat);
                let line = Line::new((x, row.y), (x, row.y + LINE_HEIGHT));
                ctx.stroke(line, &Color::GRAY, 2.0);
            }

            // Draw the other bar lines
            for (start, end) in row.bar_lines.iter().tuple_windows() {
                for beat in iterate(start.clone(), |x| x + &BeatLength::one())
                    .skip(1)
                    .take_while(|b| b < end)
                {
                    let x = get_x(&beat);
                    let line = Line::new((x, row.y + 2.0), (x, row.y + LINE_HEIGHT));
                    ctx.stroke(line, &Color::GRAY, 1.0);
                }
            }

            // Draw curosr
            if row.contains_beat(&data.cursor_position) {
                draw_cursor(ctx, get_x, &data.cursor_position, row.y, &Color::GREEN, 3.0);
            }
            // Draw music playback cursor
            if let Some(beat) = data.music_playback_position.as_ref().map(|p| &p.beat) {
                if row.contains_beat(beat) {
                    draw_cursor(ctx, get_x, beat, row.y, &Color::NAVY, 3.0);
                }
            }
            // Draw hover cursor
            if let Some(beat) = &self.hover_cursor {
                if row.contains_beat(beat) {
                    draw_cursor(ctx, get_x, beat, row.y, &Color::AQUA, 1.0);
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
                    .text_color(env.get(TEXT_COLOR))
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
                    .text_color(env.get(TEXT_COLOR))
                    .build();
                match layout {
                    Ok(layout) => ctx.draw_text(&layout, (get_x(beat), row.y)),
                    Err(e) => eprintln!("{}", e),
                }
            }
        }
    }
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
                None => (false, MeasureLength::default()),
                Some((k, v)) => (k == &cursor_position, v.to_owned()),
            };
        let widget_id = ctx.widget_id();
        let window_desc = WindowDesc::new(build_measure_dialog::<ScoreEditorData>(
            widget_id,
            cursor_position,
            current_measure_length,
            already_exsits,
        ));
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
        let window_desc = WindowDesc::new(build_bpm_dialog::<ScoreEditorData>(
            widget_id,
            cursor_position,
            current_bpm,
            already_exsits,
        ));
        ctx.new_window(window_desc);
    }

    fn edit_lyrics_mapping(&self, ctx: &mut EventCtx, data: &ScoreEditorData) {
        let i = match data.selected_track {
            Some(i) => i,
            _ => return,
        };
        let window_desc = WindowDesc::new(build_lyrics_mapping_dialog(self.font_loader.clone()));
        ctx.new_window(window_desc);
    }

    fn open_bpm_detector(&self, ctx: &mut EventCtx) {
        let window_desc =
            WindowDesc::new(build_bpm_detector_widget().lens(ScoreEditorData::bpm_detector_data));
        ctx.new_window(window_desc)
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

    fn handle_mouse_move(
        &mut self,
        _data: &mut ScoreEditorData,
        event: &MouseEvent,
    ) -> Option<BeatPosition> {
        let row = self
            .layout_cache
            .iter()
            .find(|row| row.y_range().contains(&event.pos.y))?;
        let length = BeatLength(BigRational::from_float((event.pos.x / BEAT_WIDTH).trunc())?);
        let beat = &row.beat_start + &length;
        row.contains_beat(&beat).then(|| beat)
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
    let is_first = &row.beat_start <= track.start_beat();
    let is_final = track_end_beat <= row.beat_end;

    let lyrics_height = if is_first && track.lyrics.is_some() {
        LYRICS_HEIGHT
    } else {
        0.
    };

    ctx.with_save(|ctx| {
        let rect = Rect::new(
            draw_rect.min_x(),
            track_view.y,
            get_x(&row.beat_end),
            track_view.y + NOTE_FULL_HEIGHT + lyrics_height,
        );
        ctx.clip(rect);
        let track_rect = rect.inset(Insets::uniform_xy(
            0.0,
            (NOTE_HEIGHT - NOTE_FULL_HEIGHT) / 2.0,
        ));

        let min_x = if is_first {
            get_x(track.start_beat())
        } else {
            track_rect.min_x() - 20.0
        };
        let max_x = if is_final {
            get_x(&track_end_beat)
        } else {
            track_rect.max_x() + 20.0
        };

        {
            let rect = Rect::new(min_x, track_rect.min_y(), max_x, track_rect.max_y())
                .to_rounded_rect(4.0);
            let (fill_brush, stroke_brush) = match selected {
                false => (Color::rgb8(0, 66, 19), Color::rgb8(0, 46, 13)),
                true => (Color::rgb8(66, 0, 69), Color::rgb8(32, 0, 46)),
            };
            ctx.fill(rect, &fill_brush);
            ctx.stroke(rect, &stroke_brush, 3.0);
        }

        let note_rect = track_rect.inset(Insets::uniform_xy(0.0, -6.0));
        let note_rect = note_rect.with_size((note_rect.width(), NOTE_HEIGHT - 6.0 * 2.)); // TODO magic number

        for (note_start_beat, note_end_beat, _) in track.iterate_notes() {
            if note_end_beat < row.beat_start || row.beat_end < note_start_beat {
                continue;
            }
            let rect = Rect::new(
                get_x(&note_start_beat),
                note_rect.min_y(),
                get_x(&note_end_beat),
                note_rect.max_y(),
            )
            .to_rounded_rect(5.0);
            ctx.fill(rect, &Color::rgb8(172, 255, 84));
        }

        if let Some(lyrics) = &track.lyrics {
            let layout = ctx
                .text()
                .new_text_layout(lyrics.text.to_owned())
                .text_color(Color::Rgba32(0xffffffff))
                .build();
            match layout {
                Ok(layout) => ctx.draw_text(&layout, (min_x, note_rect.max_y())),
                Err(e) => eprintln!("{}", e),
            };
        }
    });
}

fn draw_cursor<'c>(
    ctx: &mut PaintCtx<'_, '_, 'c>,
    get_x: impl Fn(&BeatPosition) -> f64,
    cursor_position: &BeatPosition,
    min_y: f64,
    brush: &impl IntoBrush<Piet<'c>>,
    width: f64,
) {
    let x = get_x(cursor_position);
    let line = Line::new((x, min_y), (x, min_y + LINE_HEIGHT));
    ctx.stroke(line, brush, width);
}
