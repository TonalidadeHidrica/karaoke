use druid::keyboard_types::Key;
use druid::kurbo::Line;
use druid::widget::Flex;
use druid::widget::Label;
use druid::AppLauncher;
use druid::Color;
use druid::Data;
use druid::Event;
use druid::Insets;
use druid::KeyEvent;
use druid::Lens;
use druid::PlatformError;
use druid::Rect;
use druid::RenderContext;
use druid::Widget;
use druid::WidgetExt;
use druid::WindowDesc;
use itertools::iterate;
use karaoke::schema::iterate_measures;
use karaoke::schema::BeatLength;
use karaoke::schema::BeatPosition;
use karaoke::schema::Score;
use karaoke::schema::ScoreElement;
use karaoke::schema::ScoreElementKind;
use karaoke::schema::Track;
use num::BigRational;
use num::ToPrimitive;
use num::Zero;
use thiserror::Error;

fn main() -> Result<(), EditorError> {
    let data = ScoreEditorData::default();
    let window = WindowDesc::new(build_toplevel_widget).window_size((1440.0, 810.0));
    AppLauncher::with_window(window).launch(data)?;

    Ok(())
}

fn build_toplevel_widget() -> impl Widget<ScoreEditorData> {
    let status_bar = Flex::row()
        .with_child(Label::dynamic(|data: &ScoreEditorData, _| {
            let pos = &data.cursor_position;
            let (start_beat, len) = match data.score.measure_lengths.range(..pos).next_back() {
                Some((a, b)) => (a.to_owned(), b.to_owned()),
                None => (BeatPosition::zero(), BeatLength::four()),
            };
            let delta_beat = (pos - &start_beat).0;
            let index = (&delta_beat / &len.0).trunc();
            let beat = &delta_beat - &index * &len.0;
            format!(
                "{}:{}",
                index,
                format_beat_position(&BeatPosition::from(beat))
            )
        }))
        .with_spacer(20.0)
        .with_child(
            Label::dynamic(|len: &BeatLength, _| {
                format!("{}-th note", BigRational::from_integer(4.into()) / &len.0)
            })
            .lens(ScoreEditorData::cursor_delta),
        )
        .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
        .must_fill_main_axis(true)
        .padding(5.0);

    Flex::column()
        .with_child(status_bar)
        .with_child(ScoreEditor::default())
}

fn format_beat_position(pos: &BeatPosition) -> String {
    let fract = match pos.0.fract() {
        a if a == BigRational::zero() => String::new(),
        a => format!("+{}", a),
    };
    format!("{}{}", pos.0.trunc(), fract)
}

#[derive(Clone, Data, Lens)]
struct ScoreEditorData {
    score: Score,

    cursor_position: BeatPosition,
    cursor_delta: BeatLength,
    selected_track: Option<usize>,
}

impl Default for ScoreEditorData {
    fn default() -> Self {
        ScoreEditorData {
            score: Score::default(),

            cursor_position: BeatPosition::zero(),
            cursor_delta: BeatLength::one(),
            selected_track: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("Error while initializing druid")]
    DruidError(#[from] PlatformError),
}

#[derive(Default)]
struct ScoreEditor {}

fn cursor_delta_candidates() -> impl DoubleEndedIterator<Item = BeatLength> {
    vec![4, 8, 12, 16, 24, 32]
        .into_iter()
        .map(|x| BeatLength::from(BigRational::new(4.into(), x.into())))
}

impl Widget<ScoreEditorData> for ScoreEditor {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut ScoreEditorData,
        _env: &druid::Env,
    ) {
        match event {
            Event::WindowConnected => {
                ctx.request_focus();
            }
            Event::KeyDown(KeyEvent { key, .. }) => match key {
                Key::Character(s) => match s.as_str() {
                    "1" => {
                        append_element(data, ScoreElementKind::Start);
                    }
                    "2" => {
                        append_element(data, ScoreElementKind::Stop);
                    }
                    " " => {
                        append_element(data, ScoreElementKind::Skip);
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
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut druid::LifeCycleCtx,
        _event: &druid::LifeCycle,
        _data: &ScoreEditorData,
        _env: &druid::Env,
    ) {
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

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &ScoreEditorData, _env: &druid::Env) {
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
                break;
            }
        }
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
    track: &karaoke::schema::Track,
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

        let rect = rect.inset(Insets::uniform_xy(0.0, -4.0));

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
