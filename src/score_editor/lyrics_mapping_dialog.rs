use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};

use druid::{RenderContext, Size, Widget, piet::{ImageFormat, InterpolationMode}};
use freetype::{face::LoadFlag, Bitmap, Library, RenderMode};
use rustybuzz::UnicodeBuffer;

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

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &ScoreEditorData, _env: &druid::Env) {
        // TODO remove unwraps!

        let mut font_loader = (&*self.font_loader).borrow_mut();
        let font_data = font_loader
            .get(data.score.font_file.clone(), ForceLoad::False)
            .unwrap();
        let face_index = 0;
        let hb_face = rustybuzz::Face::from_slice(font_data, face_index).unwrap();

        let mut buffer = UnicodeBuffer::new();
        // buffer.set_direction(rustybuzz::Direction::RightToLeft);
        buffer.push_str("Я говорю\u{0301} по-ру\u{0301}сски немно\u{0301}го.");
        let shape = rustybuzz::shape(&hb_face, &[], buffer);

        let ft_lib = Library::init().unwrap();
        let ft_face = ft_lib
            .new_memory_face(font_data.to_owned(), face_index as isize) // TODO unnecessary copy?
            .unwrap();
        let font_size = 60;
        let resolution = 50;
        ft_face
            .set_char_size(font_size * 64, 0, resolution, 0)
            .unwrap();

        // TODO The buffer is too big

        let size = ctx.size();
        let mut text_pixels = vec![0u8; (4. * size.width * size.height) as usize];

        let (mut x, mut y) = (100, font_size as i32);
        let font_size_in_pixels = font_size as f64 * resolution as f64 / 72.0;
        let hb_scale = font_size_in_pixels / ft_face.em_size() as f64;

        for (pos, info) in shape.glyph_positions().iter().zip(shape.glyph_infos()) {
            ft_face
                .load_glyph(info.codepoint, LoadFlag::DEFAULT)
                .unwrap();
            let glyph_slot = ft_face.glyph();
            let glyph = glyph_slot.get_glyph().unwrap();
            let inner_bitmap = glyph.to_bitmap(RenderMode::Normal, None).unwrap();
            let inner_bitmap = inner_bitmap.bitmap();

            let draw_x = x + glyph_slot.bitmap_left() + (pos.x_offset as f64 * hb_scale) as i32;
            let draw_y = y - glyph_slot.bitmap_top() - (pos.y_offset as f64 * hb_scale) as i32;

            blend_bitmap(
                &mut text_pixels,
                size.width as usize,
                size.height as usize,
                size.width as usize,
                &inner_bitmap,
                draw_x as usize,
                draw_y as usize,
                [255, 255, 255],
            );

            x += (pos.x_advance as f64 * hb_scale) as i32;
            y += (pos.y_advance as f64 * hb_scale) as i32;
        }

        // RgbaPremul seems correct, as RgbaSeparate generates kinda jaggy image
        let image = ctx.make_image(size.width as usize, size.height as usize, &text_pixels, ImageFormat::RgbaPremul).unwrap();
        ctx.draw_image(&image, size.to_rect(), InterpolationMode::Bilinear);
    }
}

#[allow(clippy::too_many_arguments)]
fn blend_bitmap(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    pitch: usize,
    bitmap: &Bitmap,
    x: usize,
    y: usize,
    color: [u8; 3],
) {
    for i in 0..bitmap.rows() as usize {
        let y = match y.checked_add(i) {
            Some(y) if (0..height).contains(&y) => y,
            _ => continue,
        };
        for j in 0..bitmap.width() as usize {
            let x = match x.checked_add(j) {
                Some(x) if (0..width).contains(&x) => x,
                _ => continue,
            };
            let src_a = bitmap.buffer()[i * bitmap.pitch() as usize + j] as f64;
            let k = y * pitch + x;
            let pixels = &mut pixels[k * 4..][..4];
            for (p, c) in pixels[..3].iter_mut().zip(color.iter()) {
                let res = *c as f64 * src_a + *p as f64 * (255.0 - src_a);
                *p = (res / 255.0) as _;
            }
            let dst_a = &mut pixels[3];
            let dst_aft = src_a + *dst_a as f64 * (255.0 - src_a);
            *dst_a = dst_aft as _;
            // if i == 0
            //     || i == bitmap.rows() as usize - 1
            //     || j == 0
            //     || j == bitmap.width() as usize - 1
            // {
            //     pixels
            //         .iter_mut()
            //         .zip([255, 0, 0, 255].iter())
            //         .for_each(|(p, &x)| *p = x);
            // }
        }
    }
}

pub fn build_lyrics_mapping_dialog(
    font_loader: Rc<RefCell<FontLoader>>,
) -> impl Widget<ScoreEditorData> {
    LyricsMappingEditor { font_loader }
}
