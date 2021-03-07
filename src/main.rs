use std::borrow::Borrow;

use druid::im::Vector;
use druid::AppLauncher;
use druid::Color;
use druid::Data;
use druid::Insets;
use druid::PlatformError;
use druid::RenderContext;
use druid::Size;
use druid::Widget;
use druid::WindowDesc;
use itertools::Itertools;
use thiserror::Error;

fn main() -> Result<(), EditorError> {
    let mut score = Score::default();
    use ScoreElementKind::*;
    score.elements = vec![
        Start, Continued, Continued, Start, Continued, Continued, Empty, Empty, Start, Empty,
        Start, Empty,
    ]
    .into_iter()
    .map(|kind| ScoreElement { kind })
    .collect();

    let window = WindowDesc::new(|| ScoreEditor {});
    AppLauncher::with_window(window).launch(score)?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("Error while initializing druid")]
    DruidError(#[from] PlatformError),
}

#[derive(Clone, Default, Data)]
struct Score {
    elements: Vector<ScoreElement>,
}

#[derive(Clone, PartialEq, Data)]
struct ScoreElement {
    kind: ScoreElementKind,
}

#[derive(Clone, PartialEq, Data)]
enum ScoreElementKind {
    Start,
    Continued,
    Empty,
}

struct ScoreEditor {}

fn iterate_elements<'a>(
    elements: impl Iterator<Item = impl Borrow<ScoreElement>> + 'a,
) -> impl Iterator<Item = (usize, usize)> + 'a {
    use ScoreElementKind::*;
    let mut elements = elements.enumerate().peekable();
    std::iter::from_fn(move || {
        let (i, _) = elements.find(|(_, e)| matches!(e.borrow().kind, Start))?;
        let j = match elements
            .peeking_take_while(|(_, e)| matches!(e.borrow().kind, Continued))
            .last()
        {
            Some((k, _)) => k + 1,
            None => i + 1,
        };
        Some((i, j))
    })
}

impl Widget<Score> for ScoreEditor {
    fn event(
        &mut self,
        _ctx: &mut druid::EventCtx,
        _event: &druid::Event,
        _data: &mut Score,
        _env: &druid::Env,
    ) {
        // dbg!(event);
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
        let rect = ctx.size().to_rect().inset(insets);
        let beat_width = 15.0;
        let line_height = 20.0;
        let note_height = 12.0;

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

#[cfg(test)]
mod test {
    use super::{iterate_elements, ScoreElement, ScoreElementKind};
    use itertools::Itertools;

    #[test]
    fn test_iterate_elements() {
        use ScoreElementKind::*;
        let elements = vec![
            Start, Continued, Continued, Start, Continued, Continued, Empty, Empty, Start, Empty,
            Start, Empty,
        ]
        .into_iter()
        .map(|kind| ScoreElement { kind });
        assert_eq!(
            iterate_elements(elements).collect_vec(),
            vec![(0, 3), (3, 6), (8, 9), (10, 11),]
        );
    }
}
