use std::{collections::HashMap, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, GrayImage, ImageReader, Limits, imageops::FilterType};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::model::PriceGlyph;

use super::trusted_url::TrustedZiroomUrl;

const MAX_SPRITE_BYTES: usize = 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 4_096;
const MAX_IMAGE_ALLOCATION: u64 = 16 * 1024 * 1024;

const CANONICAL_SPRITE_SEQUENCE: &str = "6291078453";
const CANONICAL_SPRITE_B64: &str = "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";

#[derive(Debug, Clone)]
struct GlyphMask(GrayImage);

#[derive(Debug)]
pub struct PriceDecoder {
    client: reqwest::Client,
    templates: Vec<GlyphMask>,
    mappings: HashMap<Url, Vec<u8>>,
}

impl PriceDecoder {
    pub fn new() -> Result<Self> {
        let bytes = STANDARD
            .decode(CANONICAL_SPRITE_B64)
            .context("invalid embedded price template")?;
        let image = decode_png(&bytes)?;
        let cells = split_cells(&image)?;
        let mut templates: Vec<Option<GlyphMask>> = vec![None; 10];
        for (index, digit) in CANONICAL_SPRITE_SEQUENCE.bytes().enumerate() {
            templates[usize::from(digit - b'0')] = Some(GlyphMask(cells[index].clone()));
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent("nest/0.1 (+https://www.ziroom.com/)")
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(15))
                .build()?,
            templates: templates
                .into_iter()
                .map(|template| template.expect("all ten templates are present"))
                .collect(),
            mappings: HashMap::new(),
        })
    }

    pub async fn decode(
        &mut self,
        glyphs: &[PriceGlyph],
        cancel: &CancellationToken,
    ) -> Result<u32> {
        let first = glyphs
            .first()
            .ok_or_else(|| anyhow!("price has no glyphs"))?;
        if glyphs
            .iter()
            .any(|glyph| glyph.sprite_url != first.sprite_url)
        {
            bail!("price uses more than one sprite");
        }
        if !self.mappings.contains_key(&first.sprite_url) {
            let trusted = TrustedZiroomUrl::parse(first.sprite_url.clone())?;
            let bytes = self.download_sprite(&trusted, cancel).await?;
            let mapping = self.classify_sprite(&bytes)?;
            self.mappings.insert(first.sprite_url.clone(), mapping);
        }
        let mapping = self
            .mappings
            .get(&first.sprite_url)
            .ok_or_else(|| anyhow!("sprite mapping disappeared"))?;
        decode_positions(glyphs, mapping)
    }

    async fn download_sprite(
        &self,
        url: &TrustedZiroomUrl,
        cancel: &CancellationToken,
    ) -> Result<Vec<u8>> {
        let request = self
            .client
            .get(url.as_url().clone())
            .header("Referer", "https://www.ziroom.com/")
            .send();
        let mut response = tokio::select! {
            () = cancel.cancelled() => bail!("price sprite download cancelled"),
            response = request => response?,
        }
        .error_for_status()?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_SPRITE_BYTES as u64)
        {
            bail!("price sprite exceeds the {MAX_SPRITE_BYTES}-byte limit");
        }
        if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE)
            && !content_type
                .to_str()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .starts_with("image/")
        {
            bail!("price sprite response was not an image");
        }
        let mut bytes = Vec::new();
        loop {
            let chunk = tokio::select! {
                () = cancel.cancelled() => bail!("price sprite download cancelled"),
                chunk = response.chunk() => chunk?,
            };
            let Some(chunk) = chunk else {
                break;
            };
            extend_limited(&mut bytes, &chunk)?;
        }
        Ok(bytes)
    }

    fn classify_sprite(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let image = decode_png(bytes)?;
        let cells = split_cells(&image)?;
        let mut costs = [[0.0_f32; 10]; 10];
        for (cell_index, cell) in cells.iter().enumerate() {
            for (digit, template) in self.templates.iter().enumerate() {
                costs[cell_index][digit] = mask_distance(cell, &template.0);
            }
        }
        let (mapping, average_cost) = minimum_cost_assignment(&costs);
        if average_cost > 0.08 {
            bail!("price sprite differs from known glyphs (distance {average_cost:.3})");
        }
        Ok(mapping)
    }
}

fn decode_png(bytes: &[u8]) -> Result<DynamicImage> {
    let mut reader = ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_IMAGE_ALLOCATION);
    reader.limits(limits);
    reader.decode().context("failed to decode price sprite")
}

fn extend_limited(buffer: &mut Vec<u8>, chunk: &[u8]) -> Result<()> {
    if buffer.len().saturating_add(chunk.len()) > MAX_SPRITE_BYTES {
        bail!("price sprite exceeds the {MAX_SPRITE_BYTES}-byte limit");
    }
    buffer.extend_from_slice(chunk);
    Ok(())
}

fn split_cells(image: &DynamicImage) -> Result<Vec<GrayImage>> {
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

fn decode_positions(glyphs: &[PriceGlyph], mapping: &[u8]) -> Result<u32> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(position: f32) -> PriceGlyph {
        PriceGlyph {
            sprite_url: Url::parse("https://static8.ziroom.com/sprite.png").unwrap(),
            background_x_px: position,
            background_size_px: Some(214.0),
        }
    }

    #[test]
    fn canonical_sprite_classifies_all_ten_digits() {
        let decoder = PriceDecoder::new().unwrap();
        let bytes = STANDARD.decode(CANONICAL_SPRITE_B64).unwrap();
        assert_eq!(
            decoder.classify_sprite(&bytes).unwrap(),
            vec![6, 2, 9, 1, 0, 7, 8, 4, 5, 3]
        );
    }

    #[test]
    fn positions_decode_8190() {
        let mapping = vec![6, 2, 9, 1, 0, 7, 8, 4, 5, 3];
        let glyphs = vec![glyph(-128.4), glyph(-64.2), glyph(-42.8), glyph(-85.6)];
        assert_eq!(decode_positions(&glyphs, &mapping).unwrap(), 8190);
    }

    #[test]
    fn invalid_sprite_is_not_guessed() {
        let decoder = PriceDecoder::new().unwrap();
        let blank = DynamicImage::new_rgba8(300, 28);
        let mut bytes = Vec::new();
        blank
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        assert!(decoder.classify_sprite(&bytes).is_err());
    }

    #[test]
    fn sprite_body_limit_is_enforced_without_declared_length() {
        let mut bytes = vec![0; MAX_SPRITE_BYTES - 2];
        assert!(extend_limited(&mut bytes, &[1, 2]).is_ok());
        assert!(extend_limited(&mut bytes, &[3]).is_err());
    }
}
