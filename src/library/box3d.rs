// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

//! A flat box art turned into a box seen at an angle, like GameTDB's `cover3D` pictures of the
//! Wii and GameCube games: the front in perspective, the spine on the left, a transparent
//! background.

use image::{Rgb, RgbImage, Rgba, RgbaImage, imageops::sample_bilinear};

/// The spine's width, of the front's
const SPINE: f32 = 0.13;
/// The front's width after turning, of the flat one's
const FRONT: f32 = 0.92;
/// The height of the front's far (right) edge, of the near one's
const FAR_FRONT: f32 = 0.93;
/// The height of the spine's far (left) edge, of the near one's
const FAR_SPINE: f32 = 0.95;
/// How much of the flat picture's left side the spine shows
const SPINE_SOURCE: f32 = 0.06;
/// The spine's brightness, and the front's at its far edge
const SPINE_LIGHT: f32 = 0.6;
const FAR_LIGHT: f32 = 0.9;

pub fn render(front: &RgbImage) -> RgbaImage {
    let (w, h) = (front.width() as f32, front.height() as f32);
    let spine_w = (w * SPINE).round().max(1.0);
    let front_w = (w * FRONT).round().max(1.0);
    let mut out = RgbaImage::new((spine_w + front_w) as u32, h as u32);

    for x in 0..out.width() {
        let xc = x as f32 + 0.5;
        // where in the flat picture this column is, how tall it is and how bright
        let (u, height, light) = if xc < spine_w {
            let s = xc / spine_w;
            (
                s * SPINE_SOURCE,
                h * (FAR_SPINE + (1.0 - FAR_SPINE) * s),
                SPINE_LIGHT,
            )
        } else {
            let t = (xc - spine_w) / front_w;
            (
                t,
                h * (1.0 - (1.0 - FAR_FRONT) * t),
                1.0 - (1.0 - FAR_LIGHT) * t,
            )
        };
        let top = (h - height) / 2.0;

        for y in 0..out.height() {
            // the part of this pixel inside the box, for smooth top and bottom edges
            let (y0, y1) = (y as f32, y as f32 + 1.0);
            let covered = (y1.min(top + height) - y0.max(top)).clamp(0.0, 1.0);
            if covered == 0.0 {
                continue;
            }
            let v = ((y as f32 + 0.5 - top) / height).clamp(0.0, 1.0);
            let Rgb([r, g, b]) =
                sample_bilinear(front, u.clamp(0.0, 1.0), v).unwrap_or(Rgb([0; 3]));
            let shade = |c: u8| (c as f32 * light).round().min(255.0) as u8;
            out.put_pixel(
                x,
                y,
                Rgba([
                    shade(r),
                    shade(g),
                    shade(b),
                    (covered * 255.0).round() as u8,
                ]),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_has_a_darker_spine_and_a_transparent_background() {
        let flat = RgbImage::from_pixel(100, 140, Rgb([200, 100, 50]));
        let turned = render(&flat);
        assert_eq!((turned.width(), turned.height()), (13 + 92, 140));

        // the middle of the spine is darker, of the front as bright as the picture near the spine
        assert_eq!(turned.get_pixel(6, 70).0, [120, 60, 30, 255]);
        assert_eq!(turned.get_pixel(13, 70).0, [200, 100, 50, 255]);
        // the far corners are outside the box: transparent
        assert_eq!(turned.get_pixel(0, 0).0[3], 0);
        assert_eq!(turned.get_pixel(104, 139).0[3], 0);
        // the near edge is (nearly) the full height
        assert!(turned.get_pixel(13, 0).0[3] > 240);
    }
}
