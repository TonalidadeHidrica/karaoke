use std::{cell::RefCell, rc::Rc};

use druid::{
    keyboard_types::Key,
    kurbo::Line,
    piet::{Image, InterpolationMode},
    Color, Event, EventCtx, KeyEvent, Modifiers, Rect, RenderContext, Size, Widget,
};

use super::ScoreEditorData;
use crate::fonts::{render_text, FontLoader, RenderedText};

#[derive(Default)]
struct LyricsMappingEditor {
    font_loader: Rc<RefCell<FontLoader>>,
    cursor_pos: usize,
    select_pos: usize,
    text_cache: Option<(String, RenderedText)>,
}

impl Widget<ScoreEditorData> for LyricsMappingEditor {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        _data: &mut ScoreEditorData,
        _env: &druid::Env,
    ) {
        match event {
            Event::WindowConnected => {
                ctx.request_focus();
            }
            Event::KeyDown(KeyEvent { key, mods, .. }) => match key {
                Key::ArrowLeft => self.move_cursor(ctx, Decrement, mods),
                Key::ArrowRight => self.move_cursor(ctx, Increment, mods),
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
        if bc.is_width_bounded() && bc.is_height_bounded() {
            bc.max()
        } else {
            let size = Size::new(100.0, 100.0);
            bc.constrain(size)
        }
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &ScoreEditorData, _env: &druid::Env) {
        // TODO cache the rendered text
        let Self {
            text_cache,
            font_loader,
            ..
        } = self;
        let text = &text_cache
            .get_or_insert_with(|| {
                (
                    String::new(),
                    render_text(
                        (&*font_loader).borrow_mut(),
                        data.score.font_file.clone(),
                        ctx,
                    ),
                )
            })
            .1;

        let top_y = 30.0;
        let image_size = text.image.size();
        let ctx_size = ctx.size();
        let text_rect = image_size
            .to_rect()
            .with_origin(((ctx_size.width - image_size.width) / 2., top_y));
        ctx.draw_image(&text.image, text_rect, InterpolationMode::Bilinear);

        let get_x = |pos: usize| {
            text_rect.x0
                + text
                    .glyphs
                    .get(pos)
                    .map_or(image_size.width, |g| g.cursor_pos.0 as f64)
        };
        let x = get_x(self.cursor_pos);
        let line = Line::new((x, text_rect.y0), (x, text_rect.y1));
        ctx.stroke(line, &Color::GREEN, 1.);

        let x2 = get_x(self.select_pos);
        let rect = Rect::new(x, text_rect.y0, x2, text_rect.y1);
        ctx.fill(rect, &Color::GREEN.with_alpha(0.3));
    }
}

#[derive(Clone, Copy)]
enum IncrementOrDecrement {
    Increment,
    Decrement,
}
use IncrementOrDecrement::*;

impl LyricsMappingEditor {
    fn move_cursor(&mut self, ctx: &mut EventCtx, dir: IncrementOrDecrement, mods: &Modifiers) {
        if let Some((_, text)) = &self.text_cache {
            match dir {
                Increment => self.cursor_pos += 1,
                Decrement => self.cursor_pos = self.cursor_pos.saturating_sub(1),
            }
            self.cursor_pos = self.cursor_pos.min(text.glyphs.len());

            if !mods.contains(Modifiers::SHIFT) {
                self.select_pos = self.cursor_pos;
            }
        }
        ctx.request_paint();
    }
}

pub fn build_lyrics_mapping_dialog(
    font_loader: Rc<RefCell<FontLoader>>,
) -> impl Widget<ScoreEditorData> {
    LyricsMappingEditor {
        font_loader,
        cursor_pos: 0,
        select_pos: 0,
        text_cache: None,
    }
}
