//! A picture as a PNG file, as the server sends it to the web client.

// The web server of a later step of the web client sends each picture
// with this. Until it does, only the tests call it. The expectation fails
// once the server calls it: then remove it.
#![cfg_attr(not(test), expect(dead_code))]

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use uoterm_view::art::Picture;

const RGBA_BYTES: usize = 4;

/// The PNG bytes of a picture, eight bits for each channel with alpha.
/// None when its bytes do not fill the picture.
pub fn encode(picture: &Picture) -> Option<Vec<u8>> {
    if picture.rgba.len() != picture.width * picture.height * RGBA_BYTES {
        return None;
    }
    let width = u32::try_from(picture.width).ok()?;
    let height = u32::try_from(picture.height).ok()?;
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&picture.rgba, width, height, ExtendedColorType::Rgba8)
        .ok()?;
    Some(png)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_view::geom::Vector;

    const RED: [u8; 4] = [u8::MAX, 0, 0, u8::MAX];
    const CLEAR: [u8; 4] = [0; 4];

    #[test]
    fn a_picture_comes_back_from_its_png_pixel_for_pixel() {
        let picture = Picture {
            width: 2,
            height: 1,
            rgba: [RED, CLEAR].concat(),
            anchor: Vector::ZERO,
        };
        let png = encode(&picture).unwrap();
        let back = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!((back.width(), back.height()), (2, 1));
        assert_eq!(back.into_raw(), picture.rgba);
    }

    #[test]
    fn bytes_that_do_not_fill_the_picture_give_none() {
        let short = Picture {
            width: 2,
            height: 2,
            rgba: RED.to_vec(),
            anchor: Vector::ZERO,
        };
        assert!(encode(&short).is_none());
    }
}
