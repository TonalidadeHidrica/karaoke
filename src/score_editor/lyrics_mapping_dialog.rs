use std::{cell::RefCell, rc::Rc};

use druid::{
    keyboard_types::Key,
    kurbo::{Line, RoundedRect},
    piet::{Image, InterpolationMode},
    Color, Event, EventCtx, KeyEvent, Modifiers, Rect, RenderContext, Size, Widget,
};
use itertools::Itertools;

use super::ScoreEditorData;
use crate::{
    fonts::{render_text, FontLoader, RenderedText},
    linest::map_f64,
    schema::Lyrics,
};

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
        data: &mut ScoreEditorData,
        _env: &druid::Env,
    ) {
        match event {
            Event::WindowConnected => {
                ctx.request_focus();
            }
            Event::KeyDown(KeyEvent { key, mods, .. }) => match key {
                Key::ArrowLeft => self.move_cursor(ctx, Decrement, mods),
                Key::ArrowRight => self.move_cursor(ctx, Increment, mods),
                Key::ArrowUp => self.modify_mapping(ctx, Increment, data),
                Key::ArrowDown => self.modify_mapping(ctx, Decrement, data),
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
        if let Some(lyrics) = selected_lyrics(data) {
            let Self {
                text_cache,
                font_loader,
                ..
            } = self;
            let text = match text_cache {
                Some((text, cache)) if text == &lyrics.text => cache,
                _ => {
                    &self
                        .text_cache
                        .insert((
                            lyrics.text.to_owned(),
                            render_text(
                                (*font_loader).borrow_mut(),
                                data.score.font_file.clone(),
                                ctx,
                                &lyrics.text,
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

            let x2 = get_x(self.select_pos);
            let rect = Rect::new(x, text_rect.y0, x2, text_rect.y1);
            ctx.fill(rect, &Color::GREEN.with_alpha(0.3));

            let top_y = top_y + image_size.height + 10.0;

            for (&(s, t), &v) in &lyrics.mappings {
                for (x1, x2) in (0..=v)
                    .map(|i| map_f64(i as f64 / v as f64, 0.0..1., get_x(s)..get_x(t)))
                    .tuple_windows()
                {
                    let rect = RoundedRect::new(x1, top_y, x2, top_y + 10.0, 5.0);
                    ctx.fill(rect, &Color::rgb8(172, 255, 84));
                }
            }
        }
    }
}

fn selected_lyrics(data: &ScoreEditorData) -> Option<&Lyrics> {
    if let Some(track_id) = data.selected_track {
        let track = &data.score.tracks[track_id];
        track.lyrics.as_ref()
    } else {
        None
    }
}

fn selected_lyrics_mut(data: &mut ScoreEditorData) -> Option<&mut Lyrics> {
    if let Some(track_id) = data.selected_track {
        let track = &mut data.score.tracks[track_id];
        track.lyrics.as_mut()
    } else {
        None
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
            let is_boundary = |&x: &usize| text.is_boundary(x);
            self.cursor_pos = match dir {
                Increment => (self.cursor_pos + 1..len).find(is_boundary).unwrap_or(len),
                Decrement => (0..self.cursor_pos).rev().find(is_boundary).unwrap_or(0),
            };
            if !mods.contains(Modifiers::SHIFT) {
                self.select_pos = self.cursor_pos;
            }
        }
        ctx.request_paint();
    }

    fn modify_mapping(
        &mut self,
        ctx: &mut EventCtx,
        dir: IncrementOrDecrement,
        data: &mut ScoreEditorData,
    ) {
        if let (Some((_, text)), Some(lyrics)) = (&self.text_cache, selected_lyrics_mut(data)) {
            let (s, t) = match (self.cursor_pos, self.select_pos) {
                (s, t) if s < t => (s, t),
                (s, t) if s > t => (t, s),
                _ => return,
            };
            // TODO naive implementation
            let mappings = &mut lyrics.mappings;
            if mappings
                .keys()
                .any(|&(ss, tt)| s < tt && ss < t && (ss, tt) != (s, t))
            {
                return;
            }
            if !text.is_boundary(s) || !text.is_boundary(t) {
                return;
            }
            match (mappings.get(&(s, t)).copied(), dir) {
                (Some(1), Decrement) => {
                    mappings.remove(&(s, t));
                }
                (x, Increment) => {
                    mappings.insert((s, t), x.unwrap_or(0) + 1);
                }
                (Some(x), Decrement) if x > 1 => {
                    mappings.insert((s, t), x - 1);
                }
                _ => {}
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
