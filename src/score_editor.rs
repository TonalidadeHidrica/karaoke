use crate::audio::AudioCommand;
use crate::audio::AudioManager;
use crate::bpm_dialog::build_bpm_dialog;
use crate::measure_dialog::build_measure_dialog;
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
        .with_child(Label::new("Volume:"))
        .with_child(Slider::new().lens(ScoreEditorData::music_volume))
        .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
        .must_fill_main_axis(true)
        .padding(5.0);

    Flex::column()
        .with_child(status_bar)
        .with_child(ScoreEditor { audio_manager })
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

            music_playback_position: None,
        }
    }
}

struct ScoreEditor {
    audio_manager: AudioManager,
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
                            let sender = self.audio_manager.command_sender();
                            if data.playing_music {
                                sender.send(AudioCommand::Pause).unwrap();
                                data.playing_music = false;
                                data.music_playback_position = None;
                            } else {
                                let pos = data.score.beat_to_time(&data.cursor_position);
                                sender.send(AudioCommand::Seek(pos)).unwrap();
                                sender.send(AudioCommand::Play).unwrap();
                                data.playing_music = true;
                                ctx.request_anim_frame();
                            }
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
            self.send_music_volume(data);
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
            ctx.request_paint();
        }
        if old_data.music_volume != data.music_volume {
            self.send_music_volume(data);
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        _data: &ScoreEditorData,
        _env: &druid::Env,
    ) -> druid::Size {
        // TODO example says that we have to check if constraints is bounded
        bc.max()
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &ScoreEditorData, env: &druid::Env) {
        let insets = Insets::uniform(-8.0);
        let draw_rect = ctx.size().to_rect().inset(insets); // .inset(status_bar_inset);
        let beat_width = 60.0;
        let line_height = 15.0;
        let line_margin = 5.0;

        let mut y = draw_rect.min_y();
        let mut left_beat: BeatPosition = BeatPosition::zero();
        let get_x =
            |length: BeatLength| draw_rect.min_x() + length.0.to_f64().unwrap() * beat_width;
        let display_end_beat = data
            .score
            .tracks
            .iter()
            .map(|x| x.end_beat())
            .max()
            .unwrap_or_else(|| data.cursor_position.to_owned())
            .max(data.cursor_position.to_owned());
        let display_end_beat = display_end_beat + BeatLength::four();

        let mut measure_lengths = data.score.measure_lengths.iter().peekable();
        let mut bpms = data.score.bpms.iter().peekable();

        for (beat_start, beat_end) in iterate_measures(&data.score.measure_lengths) {
            if &beat_end - &left_beat > BeatLength::from(BigRational::from_integer(16.into())) {
                let x = get_x(&beat_start - &left_beat);
                let line = Line::new((x, y), (x, y + line_height));
                ctx.stroke(line, &Color::GRAY, 2.0);

                let start_y = y;

                y += line_height;
                // FIXME: Naive and inefficient!
                for (i, track) in data.score.tracks.iter().enumerate() {
                    let selected = data.selected_track.map_or(false, |j| i == j);
                    y += draw_track(
                        ctx,
                        get_x,
                        &track,
                        selected,
                        &draw_rect,
                        y,
                        &left_beat,
                        &beat_start,
                    );
                }

                if (&left_beat..&beat_start).contains(&&data.cursor_position) {
                    draw_cursor(ctx, get_x, &data.cursor_position, start_y, y, &left_beat);
                }
                if let Some(beat) = data.music_playback_position.as_ref().map(|p| &p.beat) {
                    if (&left_beat..&beat_start).contains(&&beat) {
                        draw_cursor(ctx, get_x, &beat, start_y, y, &left_beat);
                    }
                }

                y += line_margin;
                left_beat = beat_start.clone();
            }

            let x = get_x(&beat_start - &left_beat);
            let line = Line::new((x, y), (x, y + line_height));
            ctx.stroke(line, &Color::GRAY, 2.0);

            if beat_start < display_end_beat {
                for b in iterate(beat_start.clone(), |x| x + &BeatLength::one())
                    .skip(1)
                    .take_while(|b| b < &beat_end)
                {
                    let x = get_x(&b - &left_beat);
                    let line = Line::new((x, y + 2.0), (x, y + line_height));
                    ctx.stroke(line, &Color::GRAY, 1.0);
                }

                for (beat, measure) in measure_lengths
                    .peeking_take_while(|(b, _)| (&beat_start..&beat_end).contains(b))
                {
                    let x = get_x(beat - &left_beat);
                    let layout = ctx
                        .text()
                        .new_text_layout(format!("{}", measure))
                        .text_color(env.get(LABEL_COLOR))
                        .build();
                    match layout {
                        Ok(layout) => ctx.draw_text(&layout, (x, y)),
                        Err(e) => eprintln!("{}", e),
                    }
                }

                for (beat, bpm) in
                    bpms.peeking_take_while(|(b, _)| (&beat_start..&beat_end).contains(b))
                {
                    // TODO duplicates?
                    let x = get_x(beat - &left_beat);
                    let layout = ctx
                        .text()
                        .new_text_layout(format!("{:.2}", bpm.0))
                        .text_color(env.get(LABEL_COLOR))
                        .build();
                    match layout {
                        Ok(layout) => ctx.draw_text(&layout, (x, y)),
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }

            if display_end_beat <= beat_start {
                let start_y = y;
                y += line_height;
                for (i, track) in data.score.tracks.iter().enumerate() {
                    let selected = data.selected_track.map_or(false, |j| i == j);
                    y += draw_track(
                        ctx,
                        get_x,
                        &track,
                        selected,
                        &draw_rect,
                        y,
                        &left_beat,
                        &beat_start,
                    );
                }
                if (&left_beat..=&beat_start).contains(&&data.cursor_position) {
                    draw_cursor(ctx, get_x, &data.cursor_position, start_y, y, &left_beat);
                }
                if let Some(beat) = data.music_playback_position.as_ref().map(|p| &p.beat) {
                    if (&left_beat..&beat_start).contains(&&beat) {
                        draw_cursor(ctx, get_x, &beat, start_y, y, &left_beat);
                    }
                }

                break;
            }
        }
    }
}

impl ScoreEditor {
    fn send_music_volume(&self, data: &ScoreEditorData) {
        self.audio_manager
            .command_sender()
            .send(AudioCommand::SetVolume(data.music_volume))
            .unwrap()
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
    ctx: &mut druid::PaintCtx,
    get_x: impl Fn(BeatLength) -> f64,
    track: &crate::schema::Track,
    selected: bool,
    draw_rect: &Rect,
    y: f64,
    beat_left: &BeatPosition,
    beat_right: &BeatPosition,
) -> f64 {
    let note_height = 24.0;
    let note_full_height = 32.0;

    let track_end_beat = track.end_beat();
    if beat_right <= track.start_beat() || &track_end_beat <= beat_left {
        return 0.0;
    }
    ctx.with_save(|ctx| {
        let rect = Rect::new(
            draw_rect.min_x(),
            y,
            get_x(beat_right - beat_left),
            y + note_full_height,
        );
        ctx.clip(rect);
        let rect = rect.inset(Insets::uniform_xy(
            0.0,
            (note_height - note_full_height) / 2.0,
        ));

        {
            let min_x = if track.start_beat() < beat_left {
                rect.min_x() - 20.0
            } else {
                get_x(track.start_beat() - beat_left)
            };
            let max_x = if beat_right < &track_end_beat {
                rect.max_x() + 20.0
            } else {
                get_x(&track_end_beat - beat_left)
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
            if &note_end_beat < beat_left || beat_right < &note_start_beat {
                continue;
            }
            let rect = Rect::new(
                get_x(&note_start_beat - beat_left),
                rect.min_y(),
                get_x(&note_end_beat - beat_left),
                rect.max_y(),
            )
            .to_rounded_rect(5.0);
            ctx.fill(rect, &Color::rgb8(172, 255, 84));
        }
    });

    note_full_height
}

fn draw_cursor(
    ctx: &mut druid::PaintCtx,
    get_x: impl Fn(BeatLength) -> f64,
    cursor_position: &BeatPosition,
    min_y: f64,
    max_y: f64,
    beat_left: &BeatPosition,
) {
    let x = get_x(cursor_position - beat_left);
    let line = Line::new((x, min_y), (x, max_y));
    ctx.stroke(line, &Color::GREEN, 3.0);
}
