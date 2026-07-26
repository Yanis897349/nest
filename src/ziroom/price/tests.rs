use base64::{Engine, engine::general_purpose::STANDARD};
use image::DynamicImage;
use url::Url;

use super::{
    CANONICAL_SPRITE_B64, PriceDecoder,
    download::{MAX_SPRITE_BYTES, extend_limited},
    recognition::{classify_sprite, decode_positions},
};
use crate::model::PriceGlyph;

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
        classify_sprite(&decoder.templates, &bytes).unwrap(),
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
    assert!(classify_sprite(&decoder.templates, &bytes).is_err());
}

#[test]
fn sprite_body_limit_is_enforced_without_declared_length() {
    let mut bytes = vec![0; MAX_SPRITE_BYTES - 2];
    assert!(extend_limited(&mut bytes, &[1, 2]).is_ok());
    assert!(extend_limited(&mut bytes, &[3]).is_err());
}
