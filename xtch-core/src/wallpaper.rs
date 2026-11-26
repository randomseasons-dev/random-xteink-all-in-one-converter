//! Wallpaper (sleep-screen) maker: one picture -> one device-size, always-portrait
//! 2-bit BMP that CrossPoint draws as-is. Steps, in order: ratio -> fill/crop
//! (+ border colour) -> 4-shade quantization (dithered or rounded) aimed at the shades
//! the X4 Pro's UC8179 screen really shows.

use crate::pipeline::Device;
use image::imageops::FilterType;
use image::{DynamicImage, GrayImage, ImageDecoder, Luma};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub struct WallpaperSettings {
    pub device: Device,
    /// Keep the picture's own ratio. false = stretch it to the device ratio (height kept).
    pub preserve_ratio: bool,
    /// Scale up or down until the picture covers the screen, then centre-crop (no borders).
    /// false = no scaling: centre-crop at native size, sides shorter than the screen get borders.
    pub fill_screen: bool,
    /// Border colour when borders occur: black (true) or white (false).
    pub black_border: bool,
    /// Floyd–Steinberg dither to the 4 screen shades. false = round each pixel to the nearest shade.
    pub dither: bool,
}

/// How CrossPoint's own tuning says the 4 shades look on the X4 Pro's UC8179 screen
/// (black, dark gray, light gray, white): both grays darker than evenly spaced.
fn shade_levels(_s: &WallpaperSettings) -> [f32; 4] {
    [15.0, 35.0, 90.0, 210.0]
}

/// Picture gray (0..255) -> palette index 0..3 per pixel. The picture's black..white is first
/// mapped onto the screen's black..white shades, then dithered (or rounded) against the
/// screen's real shades, so mid-tones land where they look right on the panel.
pub fn quantize(g: &GrayImage, s: &WallpaperSettings) -> Vec<u8> {
    let lv = shade_levels(s);
    let (lo, hi) = (lv[0], lv[3]);
    let (w, h) = (g.width() as usize, g.height() as usize);
    let mut buf: Vec<f32> = g.pixels().map(|p| lo + p.0[0] as f32 * (hi - lo) / 255.0).collect();
    let nearest = |v: f32| -> usize {
        let mut best = 0;
        for k in 1..4 {
            if (v - lv[k]).abs() < (v - lv[best]).abs() {
                best = k;
            }
        }
        best
    };
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let old = buf[i].clamp(lo, hi);
            let k = nearest(old);
            out[i] = k as u8;
            if !s.dither {
                continue;
            }
            let err = old - lv[k];
            if x + 1 < w {
                buf[i + 1] += err * 7.0 / 16.0;
            }
            if y + 1 < h {
                if x > 0 {
                    buf[i + w - 1] += err * 3.0 / 16.0;
                }
                buf[i + w] += err * 5.0 / 16.0;
                if x + 1 < w {
                    buf[i + w + 1] += err * 1.0 / 16.0;
                }
            }
        }
    }
    out
}

/// What the preview shows for each palette index: the screen's shades, stretched so black is
/// black and white is white on the PC monitor (keeps how dark the grays look on the panel).
fn preview_shades(s: &WallpaperSettings) -> [u8; 4] {
    let lv = shade_levels(s);
    let (lo, hi) = (lv[0], lv[3]);
    let mut o = [0u8; 4];
    for k in 0..4 {
        o[k] = ((lv[k] - lo) * 255.0 / (hi - lo)).round() as u8;
    }
    o
}

impl Default for WallpaperSettings {
    fn default() -> Self {
        WallpaperSettings {
            device: Device::X4Pro,
            preserve_ratio: true,
            fill_screen: true,
            black_border: false,
            dither: true,
        }
    }
}

fn resize(g: &GrayImage, w: u32, h: u32) -> GrayImage {
    image::imageops::resize(g, w.max(1), h.max(1), FilterType::Lanczos3)
}

/// Place one grayscale picture on the device-size canvas (before the 4-shade step).
pub fn make_wallpaper(img: &GrayImage, s: &WallpaperSettings) -> GrayImage {
    let (pw, ph) = s.device.portrait();
    let (w, h) = (img.width().max(1), img.height().max(1));

    let placed: GrayImage = match (s.preserve_ratio, s.fill_screen) {
        // Stretched to the device ratio, then filled: that is exactly the screen size.
        (false, true) => resize(img, pw, ph),
        // Stretched to the device ratio at the picture's own height, no scaling after.
        (false, false) => {
            let sw = (h as f64 * pw as f64 / ph as f64).round() as u32;
            resize(img, sw, h)
        }
        // Cover the screen: the tighter side matches, the other overflows and is cropped.
        (true, true) => {
            let scale = (pw as f64 / w as f64).max(ph as f64 / h as f64);
            let nw = ((w as f64 * scale).round() as u32).max(pw);
            let nh = ((h as f64 * scale).round() as u32).max(ph);
            resize(img, nw, nh)
        }
        (true, false) => img.clone(),
    };

    // Centre on the screen canvas: overflow is cropped, short sides become borders.
    let bg = if s.black_border { 0 } else { 255 };
    let mut canvas = GrayImage::from_pixel(pw, ph, Luma([bg]));
    let ox = (pw as i64 - placed.width() as i64) / 2;
    let oy = (ph as i64 - placed.height() as i64) / 2;
    image::imageops::overlay(&mut canvas, &placed, ox, oy);

    canvas
}

/// Decode a picture to grayscale, honouring EXIF rotation (phone photos) and
/// flattening any transparency onto white.
pub fn load_picture(path: &Path) -> Result<GrayImage, String> {
    let err = |e: image::ImageError| format!("{}: {}", path.display(), e);
    let mut dec = image::ImageReader::open(path)
        .map_err(|e| format!("{}: {}", path.display(), e))?
        .with_guessed_format()
        .map_err(|e| format!("{}: {}", path.display(), e))?
        .into_decoder()
        .map_err(err)?;
    let orientation = dec.orientation().ok();
    let mut img = DynamicImage::from_decoder(dec).map_err(err)?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }
    if !img.color().has_alpha() {
        return Ok(img.to_luma8());
    }
    let la = img.to_luma_alpha8();
    let mut out = GrayImage::new(la.width(), la.height());
    for (o, p) in out.pixels_mut().zip(la.pixels()) {
        let (v, a) = (p.0[0] as u32, p.0[1] as u32);
        o.0[0] = ((v * a + 255 * (255 - a)) / 255) as u8;
    }
    Ok(out)
}

/// Load + convert one picture file to wallpaper BMP bytes: a screen-size 2-bit BMP, which
/// CrossPoint 2.0 (and 1.6.x) draws exactly as-is.
pub fn render_bmp(path: &Path, s: &WallpaperSettings) -> Result<Vec<u8>, String> {
    let canvas = make_wallpaper(&load_picture(path)?, s);
    Ok(encode_bmp2_4gray(canvas.width(), canvas.height(), &quantize(&canvas, s)))
}

/// Uncompressed 2-bit palettized BMP with a 4-entry palette {0, 85, 170, 255} (CrossPoint maps
/// that palette straight to its 4 screen shades). `idx` = palette index 0..3 per pixel,
/// row-major top-down. Four pixels per byte, leftmost pixel in the high bits; rows padded to
/// 4 bytes; bottom-up.
pub fn encode_bmp2_4gray(w: u32, h: u32, idx: &[u8]) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let row = ((w + 3) / 4 + 3) & !3; // 4 px per byte, rows padded to 4 bytes
    let data_off = 54 + 4 * 4;
    let data_len = row * h;
    let mut out = Vec::with_capacity(data_off + data_len);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((data_off + data_len) as u32).to_le_bytes()); // file size
    out.extend_from_slice(&[0u8; 4]); // reserved
    out.extend_from_slice(&(data_off as u32).to_le_bytes()); // pixel data offset
    out.extend_from_slice(&40u32.to_le_bytes()); // BITMAPINFOHEADER size
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes()); // positive = bottom-up rows
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&2u16.to_le_bytes()); // bits per pixel
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB (no compression)
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes()); // 72 DPI
    out.extend_from_slice(&2835i32.to_le_bytes());
    out.extend_from_slice(&4u32.to_le_bytes()); // palette colours
    out.extend_from_slice(&4u32.to_le_bytes()); // important colours
    for v in [0u8, 85, 170, 255] {
        out.extend_from_slice(&[v, v, v, 0]); // B, G, R, reserved
    }
    let mut line = vec![0u8; row];
    for y in (0..h).rev() {
        line.iter_mut().for_each(|b| *b = 0);
        for x in 0..w {
            line[x / 4] |= (idx[y * w + x] & 3) << (6 - (x % 4) * 2);
        }
        out.extend_from_slice(&line);
    }
    out
}

/// Render many pictures across all CPU cores in bounded batches; `on_each(i, result)`
/// is called in input order.
pub fn render_bmp_par(
    paths: &[PathBuf],
    s: &WallpaperSettings,
    mut on_each: impl FnMut(usize, Result<Vec<u8>, String>),
) {
    use rayon::prelude::*;
    const BATCH: usize = 32; // big photos decode to large buffers; keep a wave small
    let mut start = 0usize;
    while start < paths.len() {
        let end = (start + BATCH).min(paths.len());
        let batch: Vec<Result<Vec<u8>, String>> = paths[start..end]
            .par_iter()
            .map(|p| render_bmp(p, s))
            .collect();
        for (k, res) in batch.into_iter().enumerate() {
            on_each(start + k, res);
        }
        start = end;
    }
}

/// Render a wallpaper to PNG bytes for the UI preview, in the shades the screen shows.
pub fn render_png(path: &Path, s: &WallpaperSettings) -> Result<Vec<u8>, String> {
    let canvas = make_wallpaper(&load_picture(path)?, s);
    let shades = preview_shades(s);
    let idx = quantize(&canvas, s);
    let img = GrayImage::from_fn(canvas.width(), canvas.height(), |x, y| {
        Luma([shades[idx[(y * canvas.width() + x) as usize] as usize]])
    });
    let mut buf = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageLuma8(img)
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(buf.into_inner())
}
