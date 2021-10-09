use std::{cell::RefCell, rc::Rc};

use druid::{
    piet::{Image, InterpolationMode},
    RenderContext, Size, Widget,
};

use super::ScoreEditorData;
use crate::fonts::{render_text, FontLoader};

#[derive(Default)]
struct LyricsMappingEditor {
    font_loader: Rc<RefCell<FontLoader>>,
}

impl Widget<ScoreEditorData> for LyricsMappingEditor {
    fn event(
        &mut self,
        _ctx: &mut druid::EventCtx,
        _event: &druid::Event,
        _data: &mut ScoreEditorData,
        _env: &druid::Env,
    ) {
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
        if bc.is_width_bounded() && bc.is_height_bounded() {
            bc.max()
        } else {
            let size = Size::new(100.0, 100.0);
            bc.constrain(size)
        }
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &ScoreEditorData, _env: &druid::Env) {
        let image = render_text(
            (&*self.font_loader).borrow_mut(),
            data.score.font_file.clone(),
            ctx,
        );
        ctx.draw_image(&image, image.size().to_rect(), InterpolationMode::Bilinear);
    }
}

pub fn build_lyrics_mapping_dialog(
    font_loader: Rc<RefCell<FontLoader>>,
) -> impl Widget<ScoreEditorData> {
    LyricsMappingEditor { font_loader }
}
