use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};

use druid::{Size, Widget};

use crate::fonts::{FontLoader, ForceLoad};

use super::ScoreEditorData;

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

    fn paint(&mut self, _ctx: &mut druid::PaintCtx, data: &ScoreEditorData, _env: &druid::Env) {
        let mut a = (&*self.font_loader).borrow_mut();
        let font = a.get(data.score.font_file.clone(), ForceLoad::False).unwrap();
    }
}

pub fn build_lyrics_mapping_dialog(
    font_loader: Rc<RefCell<FontLoader>>,
) -> impl Widget<ScoreEditorData> {
    LyricsMappingEditor { font_loader }
}
