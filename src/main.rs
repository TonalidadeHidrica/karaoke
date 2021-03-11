use druid::WidgetPod;
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
use druid::PlatformError;
use druid::RenderContext;
use druid::Size;
use druid::Widget;
use druid::WindowDesc;
use itertools::iterate;
use karaoke::schema::iterate_elements;
use karaoke::schema::iterate_measures;
use karaoke::schema::BeatLength;
use karaoke::schema::BeatPosition;
use karaoke::schema::Score;
use karaoke::schema::ScoreElement;
use karaoke::schema::ScoreElementKind;
use num::BigRational;
use num::ToPrimitive;
use thiserror::Error;

fn main() -> Result<(), EditorError> {
    let score = Score::default();
    let window = WindowDesc::new(|| ScoreEditor::default());
    AppLauncher::with_window(window).launch(score)?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("Error while initializing druid")]
    DruidError(#[from] PlatformError),
}

struct ScoreEditor {
    cursor_position: BeatPosition,
    cursor_delta: BeatLength,

    // status_bar: WidgetPod<StatusBarData, Box<dyn Widget<StatusBarData>>>,
}

impl Default for ScoreEditor {
    fn default() -> Self {
        ScoreEditor {
            cursor_position: BeatPosition::zero(),
            cursor_delta: BeatLength::one(),

            // status_bar: WidgetPod::new(Box::new(build_status_bar())),
        }
    }
}

// #[derive(Clone, Data)]
// struct StatusBarData {
//     cursor_position: String,
//     cursor_delta: String,
// }
// 
// fn build_status_bar() -> impl Widget<StatusBarData> {
//     Flex::row().with_child(Label::new("Hoge"))
// }

fn cursor_delta_candidates() -> impl DoubleEndedIterator<Item = BeatLength> {
    vec![4, 8, 12, 16, 24, 32]
        .into_iter()
        .map(|x| BeatLength::from(BigRational::new(4.into(), x.into())))
}

impl Widget<Score> for ScoreEditor {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        score: &mut Score,
        _env: &druid::Env,
    ) {
        match event {
            Event::WindowConnected => {
                ctx.request_focus();
            }
            Event::KeyDown(KeyEvent { key, .. }) => match key {
                Key::Character(s) => match s.as_str() {
                    "1" => {
                        score.elements.push_back(ScoreElement {
                            kind: ScoreElementKind::Start,
                        });
                        ctx.request_paint();
                    }
                    "2" => {
                        score.elements.push_back(ScoreElement {
                            kind: ScoreElementKind::Continued,
                        });
                        ctx.request_paint();
                    }
                    "0" => {
                        score.elements.push_back(ScoreElement {
                            kind: ScoreElementKind::Empty,
                        });
                        ctx.request_paint();
                    }
                    _ => {}
                },
                Key::Backspace => {
                    score.elements.pop_back();
                    ctx.request_paint();
                }
                Key::ArrowLeft => {
                    self.cursor_position -= &self.cursor_delta;
                    if self.cursor_position < BeatPosition::zero() {
                        self.cursor_position = BeatPosition::zero();
                    }
                    ctx.request_paint();
                }
                Key::ArrowRight => {
                    self.cursor_position += &self.cursor_delta;
                    ctx.request_paint();
                }
                Key::ArrowUp => {
                    let mut it = cursor_delta_candidates();
                    let first = it.next().unwrap();
                    self.cursor_delta = it
                        .take_while(|x| x > &self.cursor_delta)
                        .last()
                        .unwrap_or(first);
                }
                Key::ArrowDown => {
                    let mut it = cursor_delta_candidates().rev();
                    let first = it.next().unwrap();
                    self.cursor_delta = it
                        .take_while(|x| x < &self.cursor_delta)
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
        _data: &Score,
        _env: &druid::Env,
    ) {
    }

    fn update(
        &mut self,
        _ctx: &mut druid::UpdateCtx,
        _old_data: &Score,
        _data: &Score,
        _env: &druid::Env,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        _data: &Score,
        _env: &druid::Env,
    ) -> druid::Size {
        // TODO example says that we have to check if constraints is bounded
        bc.max()
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &Score, _env: &druid::Env) {
        let insets = Insets::uniform(-8.0);
        let status_bar_height = 24.0;
        let status_bar_inset = Insets::new(0.0, -status_bar_height, 0.0, 0.0);
        let draw_rect = ctx.size().to_rect().inset(insets).inset(status_bar_inset);
        let beat_width = 60.0;
        let line_height = 15.0;
        let note_height = 12.0;
        let line_margin = 5.0;

        let rect = {
            let mut r = ctx.size();
            r.height = status_bar_height;
            r.to_rect()
        };
        ctx.fill(&rect, &Color::BLACK);

        {
            let mut y = draw_rect.min_y();
            let mut left_beat: BeatPosition = BeatPosition::zero();
            let get_x =
                |length: BeatLength| draw_rect.min_x() + length.0.to_f64().unwrap() * beat_width;
            for (beat_start, beat_end) in iterate_measures(&data.measure_lengths) {
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

                if &beat_start < &self.cursor_position {
                    for b in iterate(beat_start.clone(), |x| x + &BeatLength::one())
                        .skip(1)
                        .take_while(|b| b < &beat_end)
                    {
                        let x = get_x(&b - &left_beat);
                        let line = Line::new((x, y + 2.0), (x, y + line_height));
                        ctx.stroke(line, &Color::GRAY, 1.0);
                    }
                }

                if (&beat_start..&beat_end).contains(&&self.cursor_position) {
                    let x = get_x(&self.cursor_position - &left_beat);
                    let line = Line::new((x, y), (x, y + line_height));
                    ctx.stroke(line, &Color::GREEN, 3.0);
                }

                if &self.cursor_position <= &beat_start {
                    break;
                }
            }
        }

        for (i, j) in iterate_elements(data.elements.iter()) {
            let rect = Size::new((j - i) as f64 * beat_width, note_height)
                .to_rect()
                .with_origin((
                    draw_rect.min_x() + beat_width * i as f64,
                    draw_rect.min_y() + line_height - note_height,
                ))
                .to_rounded_rect(3.0);
            ctx.fill(rect, &Color::OLIVE);
        }
    }
}
