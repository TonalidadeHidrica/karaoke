use druid::keyboard_types::Key;
use druid::kurbo::Line;
use druid::AppLauncher;
use druid::Color;
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
use num::Zero;
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
}

impl Default for ScoreEditor {
    fn default() -> Self {
        ScoreEditor {
            cursor_position: BeatPosition::zero(),
        }
    }
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
                    self.cursor_position -= BeatLength::one();
                    if self.cursor_position < BeatPosition::zero() {
                        self.cursor_position = BeatPosition::zero();
                    }
                    ctx.request_paint();
                }
                Key::ArrowRight => {
                    self.cursor_position += BeatLength::one();
                    ctx.request_paint();
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

    #[allow(unreachable_code)]
    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &Score, _env: &druid::Env) {
        let insets = Insets::uniform(-8.0);
        let rect = ctx.size().to_rect().inset(insets);
        let beat_width = 60.0;
        let line_height = 15.0;
        let note_height = 12.0;
        let line_margin = 5.0;

        {
            let mut y = rect.min_y();
            let mut left_beat: BeatPosition = BeatPosition::zero();
            let get_x = |length: BeatLength| rect.min_x() + length.0.to_f64().unwrap() * beat_width;
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
                    dbg!(&self.cursor_position, &left_beat);
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
                    rect.min_x() + beat_width * i as f64,
                    rect.min_y() + line_height - note_height,
                ))
                .to_rounded_rect(3.0);
            ctx.fill(rect, &Color::OLIVE);
        }
    }
}
