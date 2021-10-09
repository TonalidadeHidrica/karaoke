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
        if let Some(track_id) = data.selected_track {
            let track = &data.score.tracks[track_id];
            if let Some(lyrics) = &track.lyrics {
                let Self {
                    text_cache,
                    font_loader,
                    ..
                } = self;
                let text = match text_cache {
                    Some((text, cache)) if text == lyrics => cache,
                    _ => {
                        &self
                            .text_cache
                            .insert((
                                lyrics.to_owned(),
                                render_text(
                                    (&*font_loader).borrow_mut(),
                                    data.score.font_file.clone(),
                                    ctx,
                                    lyrics,
                                ),
                            ))
                            .1
                    }
                };
                // println!("{:?}", text.glyphs);

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
                println!("{}", x);

                let x2 = get_x(self.select_pos);
                let rect = Rect::new(x, text_rect.y0, x2, text_rect.y1);
                ctx.fill(rect, &Color::GREEN.with_alpha(0.3));
            }
        }
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
            let len = text.glyphs.len();
            let is_boundary = |&x: &usize| {
                x > 0
                    && match (text.glyphs.get(x - 1), text.glyphs.get(x)) {
                        (Some(a), Some(b)) => a.glyph_info.cluster != b.glyph_info.cluster,
                        _ => false,
                    }
            };
            self.cursor_pos = match dir {
                Increment => (self.cursor_pos+1..len).find(is_boundary).unwrap_or(len),
                Decrement => (0..self.cursor_pos).rev().find(is_boundary).unwrap_or(0),
            };
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
