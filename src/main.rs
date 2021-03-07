use druid::kurbo::Line;
use druid::AppLauncher;
use druid::Color;
use druid::PlatformError;
use druid::RenderContext;
use druid::Widget;
use druid::WindowDesc;
use thiserror::Error;

fn main() -> Result<(), EditorError> {
    let window = WindowDesc::new(|| ScoreEditor);
    AppLauncher::with_window(window).launch(())?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("Error while initializing druid")]
    DruidError(#[from] PlatformError),
}

struct ScoreEditor;

impl Widget<()> for ScoreEditor {
    fn event(
        &mut self,
        _ctx: &mut druid::EventCtx,
        _event: &druid::Event,
        _data: &mut (),
        _env: &druid::Env,
    ) {
        // dbg!(event);
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut druid::LifeCycleCtx,
        _event: &druid::LifeCycle,
        _data: &(),
        _env: &druid::Env,
    ) {
    }

    fn update(
        &mut self,
        _ctx: &mut druid::UpdateCtx,
        _old_data: &(),
        _data: &(),
        _env: &druid::Env,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        _data: &(),
        _env: &druid::Env,
    ) -> druid::Size {
        // TODO example says that we have to check if constraints is bounded
        bc.max()
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, _data: &(), _env: &druid::Env) {
        let line = Line::new((10.0, 10.0), (20.0, 20.0));
        ctx.stroke(line, &Color::WHITE, 1.0);
    }
}
