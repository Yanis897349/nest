use anyhow::{Context, Result, anyhow, bail};
use image::{DynamicImage, GrayImage, ImageReader, Limits, imageops::FilterType};

use crate::model::PriceGlyph;

const MAX_IMAGE_DIMENSION: u32 = 4_096;
const MAX_IMAGE_ALLOCATION: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(super) struct GlyphMask(pub(super) GrayImage);

pub(super) fn classify_sprite(templates: &[GlyphMask], bytes: &[u8]) -> Result<Vec<u8>> {
    let image = decode_png(bytes)?;
    let cells = split_cells(&image)?;
    let mut costs = [[0.0_f32; 10]; 10];
    for (cell_index, cell) in cells.iter().enumerate() {
        for (digit, template) in templates.iter().enumerate() {
            costs[cell_index][digit] = mask_distance(cell, &template.0);
        }
    }
    let (mapping, average_cost) = minimum_cost_assignment(&costs);
    if average_cost > 0.08 {
        bail!("price sprite differs from known glyphs (distance {average_cost:.3})");
    }
    Ok(mapping)
}

pub(super) fn decode_png(bytes: &[u8]) -> Result<DynamicImage> {
    let mut reader = ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_IMAGE_ALLOCATION);
    reader.limits(limits);
    reader.decode().context("failed to decode price sprite")
}

pub(super) fn split_cells(image: &DynamicImage) -> Result<Vec<GrayImage>> {
    let gray = image.to_luma_alpha8();
    let width = gray.width();
    let height = gray.height();
    if width < 10 || !width.is_multiple_of(10) || height == 0 {
        bail!("unexpected price sprite dimensions {width}x{height}");
    }
    let cell_width = width / 10;
    let cells = (0..10)
        .map(|index| {
            let rgba = image.crop_imm(index * cell_width, 0, cell_width, height);
            let alpha = rgba.to_rgba8();
            let mut mask = GrayImage::new(cell_width, height);
            for (x, y, pixel) in alpha.enumerate_pixels() {
                mask.put_pixel(x, y, image::Luma([pixel.0[3]]));
            }
            image::imageops::resize(&mask, 30, 28, FilterType::Triangle)
        })
        .collect();
    Ok(cells)
}

fn mask_distance(left: &GrayImage, right: &GrayImage) -> f32 {
    let total = left.width() * left.height();
    left.pixels()
        .zip(right.pixels())
        .map(|(a, b)| (f32::from(a.0[0]) - f32::from(b.0[0])).abs() / 255.0)
        .sum::<f32>()
        / total as f32
}

fn minimum_cost_assignment(costs: &[[f32; 10]; 10]) -> (Vec<u8>, f32) {
    let mut dp = vec![f32::INFINITY; 1 << 10];
    let mut parent = vec![None; 1 << 10];
    dp[0] = 0.0;
    for mask in 0_usize..(1 << 10) {
        let cell = mask.count_ones() as usize;
        if cell == 10 || !dp[mask].is_finite() {
            continue;
        }
        for digit in 0..10 {
            if mask & (1 << digit) != 0 {
                continue;
            }
            let next = mask | (1 << digit);
            let candidate = dp[mask] + costs[cell][digit];
            if candidate < dp[next] {
                dp[next] = candidate;
                parent[next] = Some((mask, digit));
            }
        }
    }
    let mut mapping = vec![0_u8; 10];
    let mut mask = (1 << 10) - 1;
    for cell in (0..10).rev() {
        let (previous, digit) = parent[mask].expect("a complete assignment exists");
        mapping[cell] = digit as u8;
        mask = previous;
    }
    (mapping, dp[(1 << 10) - 1] / 10.0)
}

pub(super) fn decode_positions(glyphs: &[PriceGlyph], mapping: &[u8]) -> Result<u32> {
    let mut price = 0_u32;
    for glyph in glyphs {
        let sprite_width = glyph
            .background_size_px
            .filter(|value| *value > 0.0)
            .unwrap_or(214.0);
        let cell_width = sprite_width / 10.0;
        let index = (glyph.background_x_px.abs() / cell_width).round() as usize;
        let digit = mapping
            .get(index)
            .copied()
            .ok_or_else(|| anyhow!("price background offset points outside sprite"))?;
        price = price
            .checked_mul(10)
            .and_then(|value| value.checked_add(u32::from(digit)))
            .ok_or_else(|| anyhow!("price overflow"))?;
    }
    Ok(price)
}
