use std::cmp::Eq;
use std::collections::hash_map::Entry;
use std::io::{BufReader, Read};
use std::ops::DerefMut;
use std::{collections::HashMap, path::PathBuf};

use druid::piet::d2d::Bitmap as CoreGraphicsImage;
use druid::PaintCtx;
use druid::{piet::ImageFormat, RenderContext};
use freetype::{face::LoadFlag, Bitmap, Library, RenderMode};
use fs_err::File;
use itertools::{zip, Itertools};
use rustybuzz::{GlyphInfo, GlyphPosition, UnicodeBuffer};
use thiserror::Error;

#[derive(Default)]
pub struct FontLoader {
    faces: HashMap<PathBuf, Result<Vec<u8>, FontLoadError>>,
}

#[derive(PartialEq, Eq)]
pub enum ForceLoad {
    True,
    False,
}

impl FontLoader {
    // TODO Until raw_entry_mut is stabilized, I have to clone pathbuf over and over again.
    pub fn get(&mut self, path: PathBuf, force: ForceLoad) -> Result<&[u8], &FontLoadError> {
        let res = match self.faces.entry(path) {
            Entry::Occupied(mut entry) => {
                if entry.get().is_err() && force == ForceLoad::True {
                    let _ = entry.insert(load_file_into_vec(entry.key()));
                }
                entry.into_mut()
            }
            Entry::Vacant(entry) => {
                let value = load_file_into_vec(entry.key());
                entry.insert(value)
            }
        };
        res.as_ref().map(|x| &x[..])
    }
}

fn load_file_into_vec(path: impl Into<PathBuf>) -> Result<Vec<u8>, FontLoadError> {
    let mut v = Vec::new();
    BufReader::new(File::open(path)?).read_to_end(&mut v)?;
    Ok(v)
}

#[derive(Debug, Error)]
pub enum FontLoadError {
    #[error("{0}")]
    IOError(#[from] std::io::Error),
}

pub struct RenderedText {
    pub glyphs: Vec<RenderedGlyph>,
    pub image: CoreGraphicsImage,
}
#[derive(Debug)]
pub struct RenderedGlyph {
    pub cursor_pos: (usize, usize),
    pub top_left: (usize, usize),
    pub size: (usize, usize),
    pub glyph_pos: GlyphPosition,
    pub glyph_info: GlyphInfo,
}

impl RenderedText {
    pub fn is_boundary(&self, x: usize) -> bool {
        x == 0
            || x == self.glyphs.len()
            || match (self.glyphs.get(x - 1), self.glyphs.get(x)) {
                (Some(a), Some(b)) => a.glyph_info.cluster != b.glyph_info.cluster,
                _ => false,
            }
    }
}

pub fn render_text(
    mut font_loader: impl DerefMut<Target = FontLoader>,
    font_path: PathBuf,
    paint_ctx: &mut PaintCtx,
    text: &str,
) -> RenderedText {
    // TODO remove unwraps!

    let font_data = font_loader.get(font_path, ForceLoad::False).unwrap();
    let face_index = 0;
    let hb_face = rustybuzz::Face::from_slice(font_data, face_index).unwrap();

    let mut buffer = UnicodeBuffer::new();
    // buffer.set_direction(rustybuzz::Direction::RightToLeft);
    buffer.push_str(text);
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

    let (mut x, mut y) = (0, 0);
    let font_size_in_pixels = font_size as f64 * resolution as f64 / 72.0;
    let hb_scale = font_size_in_pixels / ft_face.em_size() as f64;

    let (xys, infos) = shape
        .glyph_positions()
        .iter()
        .zip(shape.glyph_infos())
        .map(|(pos, info)| {
            ft_face
                .load_glyph(info.codepoint, LoadFlag::DEFAULT)
                .unwrap();
            let glyph_slot = ft_face.glyph();
            let bitmap = glyph_slot.bitmap();

            let old_xy = (x, y);
            let draw_x = x + glyph_slot.bitmap_left() + (pos.x_offset as f64 * hb_scale) as i32;
            let draw_y = y - glyph_slot.bitmap_top() - (pos.y_offset as f64 * hb_scale) as i32;

            x += (pos.x_advance as f64 * hb_scale) as i32;
            y += (pos.y_advance as f64 * hb_scale) as i32;

            (
                (old_xy, (draw_x, draw_y)),
                ((bitmap.width(), bitmap.rows()), pos, info),
            )
        })
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let xs = zip(&xys, &infos)
        .flat_map(|((dxy, xy), (wh, ..))| [xy.0, xy.0 + wh.0, dxy.0])
        .chain([0, x]);
    let ys = zip(&xys, &infos)
        .flat_map(|((dxy, xy), (wh, ..))| [xy.1, xy.1 + wh.1, dxy.1])
        .chain([0, y]);
    // println!("{:?} {:?}", xs.clone().collect_vec(), ys.clone().collect_vec());
    let (xs, ys, w, h) = match (xs.minmax().into_option(), ys.minmax().into_option()) {
        (Some((xs, xt)), Some((ys, yt))) => (xs, ys, (xt - xs) as usize, (yt - ys) as usize),
        _ => (0, 0, 0, 0),
    };
    let xys = xys
        .into_iter()
        .map(|((dx, dy), (x, y))| {
            (
                ((dx - xs) as usize, (dy - ys) as usize),
                ((x - xs) as usize, (y - ys) as usize),
            )
        })
        .collect_vec();
    // println!("{:?} {:?}", (xs, ys, w, h), xys);

    let mut text_pixels = vec![0u8; 4 * w * h];

    for (&((_x, _y), (draw_x, draw_y)), (_, _, info)) in zip(&xys, &infos) {
        ft_face
            .load_glyph(info.codepoint, LoadFlag::DEFAULT)
            .unwrap();
        let glyph_slot = ft_face.glyph();
        let glyph = glyph_slot.get_glyph().unwrap();
        let inner_bitmap = glyph.to_bitmap(RenderMode::Normal, None).unwrap();
        let inner_bitmap = inner_bitmap.bitmap();

        blend_bitmap(
            &mut text_pixels,
            w,
            h,
            w,
            &inner_bitmap,
            draw_x,
            draw_y,
            [255, 255, 255],
        );

        // // Draw red lines to the cursor positions
        // for i in 0..10 {
        //     if (0..w).contains(&x) {
        //         if let Some(x) = text_pixels.get_mut((x + (y - i) * w) * 4..) {
        //             x[0] = 255;
        //             x[3] = 255;
        //         }
        //     }
        // }
    }

    let glyphs = zip(xys, infos)
        .map(
            |((cursor_pos, top_left), ((w, h), &glyph_pos, &glyph_info))| RenderedGlyph {
                cursor_pos,
                top_left,
                size: (w as usize, h as usize),
                glyph_pos,
                glyph_info,
            },
        )
        .collect();
    RenderedText {
        image: paint_ctx
            .make_image(w, h, &text_pixels, ImageFormat::RgbaPremul)
            .unwrap(),
        glyphs,
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

            // The following is for for drawing border, debugging purpose
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
