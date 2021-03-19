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
use druid::RenderContext;
// use druid::Size;
use druid::Widget;
use druid::WidgetExt;
use druid::WindowDesc;
use itertools::iterate;
// use karaoke::schema::iterate_elements;
use karaoke::schema::iterate_measures;
use karaoke::schema::BeatLength;
use karaoke::schema::BeatPosition;
use karaoke::schema::Score;
// use karaoke::schema::ScoreElement;
// use karaoke::schema::ScoreElementKind;
use num::BigRational;
use num::ToPrimitive;
use num::Zero;
use thiserror::Error;

fn main() -> Result<(), EditorError> {
    let score = ScoreEditorData::default();
    let window = WindowDesc::new(build_toplevel_widget).window_size((1440.0, 810.0));
    AppLauncher::with_window(window).launch(score)?;

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
    cursor_position: BeatPosition,
    cursor_delta: BeatLength,
    score: Score,
}

impl Default for ScoreEditorData {
    fn default() -> Self {
        ScoreEditorData {
            cursor_position: BeatPosition::zero(),
            cursor_delta: BeatLength::one(),
            score: Score::default(),
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
                    // "1" => {
                    //     data.score.elements.push_back(ScoreElement {
                    //         kind: ScoreElementKind::Start,
                    //     });
                    //     ctx.request_paint();
                    // }
                    // "2" => {
                    //     data.score.elements.push_back(ScoreElement {
                    //         kind: ScoreElementKind::Continued,
                    //     });
                    //     ctx.request_paint();
                    // }
                    // "0" => {
                    //     data.score.elements.push_back(ScoreElement {
                    //         kind: ScoreElementKind::Empty,
                    //     });
                    //     ctx.request_paint();
                    // }
                    _ => {}
                },
                // Key::Backspace => {
                //     data.score.elements.pop_back();
                //     ctx.request_paint();
                // }
                Key::ArrowLeft => {
                    data.cursor_position -= &data.cursor_delta;
                    if data.cursor_position < BeatPosition::zero() {
                        data.cursor_position = BeatPosition::zero();
                    }
                    ctx.request_paint();
                }
                Key::ArrowRight => {
                    data.cursor_position += &data.cursor_delta;
                    ctx.request_paint();
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
        _ctx: &mut druid::UpdateCtx,
        _old_data: &ScoreEditorData,
        _data: &ScoreEditorData,
        _env: &druid::Env,
    ) {
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
        // let note_height = 12.0;
        let line_margin = 5.0;

        let mut y = draw_rect.min_y();
        let mut left_beat: BeatPosition = BeatPosition::zero();
        let get_x =
            |length: BeatLength| draw_rect.min_x() + length.0.to_f64().unwrap() * beat_width;
        for (beat_start, beat_end) in iterate_measures(&data.score.measure_lengths) {
            if &beat_end - &left_beat > BeatLength::from(BigRational::from_integer(16.into())) {
                let x = get_x(&beat_start - &left_beat);
                let line = Line::new((x, y), (x, y + line_height));
                ctx.stroke(line, &Color::GRAY, 2.0);

                left_beat = beat_start.clone();
                y += line_height + line_margin;
            }

            let x = get_x(&beat_start - &left_beat);
            let line = Line::new((x, y), (x, y + line_height));
            ctx.stroke(line, &Color::GRAY, 2.0);

            if &beat_start < &data.cursor_position {
                for b in iterate(beat_start.clone(), |x| x + &BeatLength::one())
                    .skip(1)
                    .take_while(|b| b < &beat_end)
                {
                    let x = get_x(&b - &left_beat);
                    let line = Line::new((x, y + 2.0), (x, y + line_height));
                    ctx.stroke(line, &Color::GRAY, 1.0);
                }
            }

            if (&beat_start..&beat_end).contains(&&data.cursor_position) {
                let x = get_x(&data.cursor_position - &left_beat);
                let line = Line::new((x, y), (x, y + line_height));
                ctx.stroke(line, &Color::GREEN, 3.0);
            }

            if &data.cursor_position <= &beat_start {
                break;
            }
        }

        // for (i, j) in iterate_elements(data.score.elements.iter()) {
        //     let rect = Size::new((j - i) as f64 * beat_width, note_height)
        //         .to_rect()
        //         .with_origin((
        //             draw_rect.min_x() + beat_width * i as f64,
        //             draw_rect.min_y() + line_height - note_height,
        //         ))
        //         .to_rounded_rect(3.0);
        //     ctx.fill(rect, &Color::OLIVE);
        // }
    }
}
