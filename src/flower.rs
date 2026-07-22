use image::{
    codecs::png::PngEncoder,
    ExtendedColorType, ImageEncoder, Rgba, RgbaImage,
};
use std::collections::HashMap;

const LOGICAL_SIZE: i32 = 16;
const OUTPUT_SIZE: u32 = 32;
const PIXEL_SCALE: u32 = OUTPUT_SIZE / LOGICAL_SIZE as u32;

const PETAL_PALETTES: [[[u8; 4]; 3]; 7] = [
    [rgba(0xff, 0x6b, 0x8a), rgba(0xd9, 0x36, 0x63), rgba(0xff, 0x9d, 0xb2)],
    [rgba(0xa7, 0x8b, 0xfa), rgba(0x75, 0x57, 0xd8), rgba(0xc4, 0xb5, 0xfd)],
    [rgba(0x60, 0xa5, 0xfa), rgba(0x25, 0x63, 0xeb), rgba(0x93, 0xc5, 0xfd)],
    [rgba(0xfb, 0xbf, 0x24), rgba(0xd9, 0x77, 0x06), rgba(0xfd, 0xe6, 0x8a)],
    [rgba(0xfb, 0x71, 0x85), rgba(0xe1, 0x1d, 0x48), rgba(0xfd, 0xa4, 0xaf)],
    [rgba(0x2d, 0xd4, 0xbf), rgba(0x0f, 0x9f, 0x91), rgba(0x99, 0xf6, 0xe4)],
    [rgba(0xf4, 0x72, 0xb6), rgba(0xc0, 0x26, 0x7d), rgba(0xf9, 0xa8, 0xd4)],
];

const CENTER_COLORS: [[u8; 4]; 4] = [
    rgba(0xfa, 0xcc, 0x15),
    rgba(0xf5, 0x9e, 0x0b),
    rgba(0xfd, 0xe0, 0x47),
    rgba(0xfb, 0x92, 0x3c),
];

const STEM_COLORS: [[u8; 4]; 4] = [
    rgba(0x15, 0x80, 0x3d),
    rgba(0x16, 0xa3, 0x4a),
    rgba(0x4d, 0x7c, 0x0f),
    rgba(0x05, 0x96, 0x69),
];

const BACKGROUNDS: [[u8; 4]; 6] = [
    rgba(0xf4, 0xf1, 0xe8),
    rgba(0xe8, 0xf0, 0xff),
    rgba(0xfc, 0xe8, 0xee),
    rgba(0xe8, 0xf8, 0xef),
    rgba(0xff, 0xf3, 0xd9),
    rgba(0xee, 0xe8, 0xff),
];

const PETAL_SHAPE: [(i32, i32); 14] = [
    (0, -3),
    (-1, -3),
    (0, -2),
    (-1, -2),
    (-2, -2),
    (-3, -1),
    (-2, -1),
    (-3, 0),
    (-2, 0),
    (-3, 1),
    (-2, 1),
    (-2, 2),
    (-1, 2),
    (0, 2),
];

const CENTER_HIGHLIGHT: [u8; 4] = rgba(0xff, 0xf3, 0xa3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowerStyle {
    FlowerWithStem,
    FlowerOnly,
}

#[derive(Debug, Clone)]
pub struct GeneratedFlower {
    pub image: RgbaImage,
    pub background: [u8; 4],
}

pub fn generate_flower_image(seed: &str, style: FlowerStyle) -> GeneratedFlower {
    let include_stem = style == FlowerStyle::FlowerWithStem;
    let mut random = SeededRandom::new(seed);

    let palette = PETAL_PALETTES[random.index(PETAL_PALETTES.len())];
    let center_color = CENTER_COLORS[random.index(CENTER_COLORS.len())];
    let stem_color = STEM_COLORS[random.index(STEM_COLORS.len())];
    let leaf_color = STEM_COLORS[random.index(STEM_COLORS.len())];
    let background = BACKGROUNDS[random.index(BACKGROUNDS.len())];

    let center_x = 8;
    let center_y = if include_stem {
        6 + random.index(2) as i32
    } else {
        8
    };
    let stem_lean = if random.next_f64() > 0.5 { 1 } else { -1 };

    let mut pixels = PixelGrid::default();

    for &(offset_x, offset_y) in &PETAL_SHAPE {
        let distance = offset_x.abs() + offset_y.abs();
        let color = if distance >= 4 {
            palette[1]
        } else if random.next_f64() > 0.72 {
            palette[2]
        } else {
            palette[0]
        };

        pixels.add_mirrored(center_x, center_y, offset_x, offset_y, color);
    }

    if random.next_f64() > 0.45 {
        pixels.add_mirrored(center_x, center_y, 3, 0, palette[1]);
    }
    if random.next_f64() > 0.50 {
        pixels.add_mirrored(center_x, center_y, 1, -4, palette[1]);
    }
    if random.next_f64() > 0.55 {
        pixels.add_mirrored(center_x, center_y, 3, 1, palette[0]);
    }
    if random.next_f64() > 0.60 {
        pixels.add_mirrored(center_x, center_y, 1, 3, palette[1]);
    }

    if !include_stem {
        add_flower_only_variation(&mut pixels, &mut random, center_x, center_y, palette);
    }

    pixels.add(center_x, center_y, center_color);
    pixels.add(center_x - 1, center_y, center_color);
    pixels.add(center_x, center_y - 1, center_color);
    pixels.add(center_x - 1, center_y - 1, center_color);

    if random.next_f64() > 0.4 {
        pixels.add(center_x - 1, center_y - 1, CENTER_HIGHLIGHT);
    }

    if include_stem {
        add_stem(
            &mut pixels,
            &mut random,
            center_x,
            center_y,
            stem_lean,
            stem_color,
            leaf_color,
        );
    } else {
        pixels.recenter();
    }

    let mut image = RgbaImage::from_pixel(OUTPUT_SIZE, OUTPUT_SIZE, Rgba(background));

    for ((x, y), color) in pixels.items {
        let start_x = x as u32 * PIXEL_SCALE;
        let start_y = y as u32 * PIXEL_SCALE;

        for dy in 0..PIXEL_SCALE {
            for dx in 0..PIXEL_SCALE {
                image.put_pixel(start_x + dx, start_y + dy, Rgba(color));
            }
        }
    }

    GeneratedFlower { image, background }
}

pub fn generate_flower_png(
    seed: &str,
    style: FlowerStyle,
) -> Result<Vec<u8>, image::ImageError> {
    let generated = generate_flower_image(seed, style);
    encode_png(&generated.image)
}

pub fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, image::ImageError> {
    let mut bytes = Vec::new();

    PngEncoder::new(&mut bytes).write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        ExtendedColorType::Rgba8,
    )?;

    Ok(bytes)
}

fn add_flower_only_variation(
    pixels: &mut PixelGrid,
    random: &mut SeededRandom,
    center_x: i32,
    center_y: i32,
    palette: [[u8; 4]; 3],
) {
    match random.index(5) {
        0 => {
            pixels.add_mirrored(center_x, center_y, 3, -1, palette[0]);
            pixels.add_mirrored(center_x, center_y, 3, 0, palette[1]);
            pixels.add_mirrored(center_x, center_y, 3, 1, palette[0]);
            pixels.add_mirrored(center_x, center_y, 2, 2, palette[1]);
        }
        1 => {
            pixels.add_mirrored(center_x, center_y, 0, -4, palette[1]);
            pixels.add_mirrored(center_x, center_y, 1, -4, palette[1]);
            pixels.add_mirrored(center_x, center_y, 0, 3, palette[0]);
            pixels.add_mirrored(center_x, center_y, 1, 3, palette[1]);
        }
        2 => {
            pixels.add_mirrored(center_x, center_y, 4, 0, palette[1]);
            pixels.add_mirrored(center_x, center_y, 4, 1, palette[1]);
            pixels.add_mirrored(center_x, center_y, 3, -2, palette[0]);
            pixels.add_mirrored(center_x, center_y, 3, 2, palette[0]);
        }
        3 => {
            pixels.add_mirrored(center_x, center_y, 1, -3, palette[2]);
            pixels.add_mirrored(center_x, center_y, 2, -2, palette[2]);
            pixels.add_mirrored(center_x, center_y, 2, 1, palette[2]);
            pixels.add_mirrored(center_x, center_y, 1, 2, palette[2]);
        }
        _ => {
            pixels.add_mirrored(center_x, center_y, 2, -3, palette[1]);
            pixels.add_mirrored(center_x, center_y, 3, -2, palette[0]);
            pixels.add_mirrored(center_x, center_y, 3, 2, palette[1]);

            let x = if random.next_f64() > 0.5 {
                center_x - 1
            } else {
                center_x + 1
            };
            pixels.add(x, center_y + 3, palette[0]);
        }
    }

    if random.next_f64() > 0.55 {
        pixels.add_mirrored(center_x, center_y, 2, -3, palette[1]);
    }
    if random.next_f64() > 0.60 {
        pixels.add_mirrored(center_x, center_y, 3, -1, palette[2]);
    }
    if random.next_f64() > 0.65 {
        pixels.add_mirrored(center_x, center_y, 2, 3, palette[1]);
    }
    if random.next_f64() > 0.70 {
        pixels.add_mirrored(center_x, center_y, 4, 0, palette[1]);
    }
}

#[allow(clippy::too_many_arguments)]
fn add_stem(
    pixels: &mut PixelGrid,
    random: &mut SeededRandom,
    center_x: i32,
    center_y: i32,
    stem_lean: i32,
    stem_color: [u8; 4],
    leaf_color: [u8; 4],
) {
    let mut stem_x = center_x;

    for y in (center_y + 2)..=14 {
        if y > center_y + 4 && random.next_f64() > 0.72 {
            stem_x = (stem_x + stem_lean).clamp(6, 9);
        }
        pixels.add(stem_x, y, stem_color);
    }

    let leaf_y = center_y + 5 + random.index(3) as i32;
    let leaf_direction = if random.next_f64() > 0.5 { 1 } else { -1 };

    pixels.add(stem_x + leaf_direction, leaf_y, leaf_color);
    pixels.add(stem_x + leaf_direction * 2, leaf_y - 1, leaf_color);
    pixels.add(stem_x + leaf_direction * 2, leaf_y, leaf_color);

    if random.next_f64() > 0.45 {
        let opposite_y = (leaf_y + 2).min(13);
        pixels.add(stem_x - leaf_direction, opposite_y, leaf_color);
        pixels.add(
            stem_x - leaf_direction * 2,
            opposite_y - 1,
            leaf_color,
        );
    }

    pixels.add(stem_x - 1, 15, stem_color);
    pixels.add(stem_x, 15, stem_color);
    pixels.add(stem_x + 1, 15, stem_color);
}

#[derive(Default)]
struct PixelGrid {
    items: HashMap<(i32, i32), [u8; 4]>,
}

impl PixelGrid {
    fn add(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if (0..LOGICAL_SIZE).contains(&x) && (0..LOGICAL_SIZE).contains(&y) {
            self.items.insert((x, y), color);
        }
    }

    fn add_mirrored(
        &mut self,
        center_x: i32,
        center_y: i32,
        offset_x: i32,
        offset_y: i32,
        color: [u8; 4],
    ) {
        self.add(center_x + offset_x, center_y + offset_y, color);

        if offset_x != 0 {
            self.add(center_x - offset_x, center_y + offset_y, color);
        }
    }

    fn recenter(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let min_x = self.items.keys().map(|(x, _)| *x).min().unwrap();
        let max_x = self.items.keys().map(|(x, _)| *x).max().unwrap();
        let min_y = self.items.keys().map(|(_, y)| *y).min().unwrap();
        let max_y = self.items.keys().map(|(_, y)| *y).max().unwrap();

        let dx = round_js(7.5 - (min_x + max_x) as f64 / 2.0);
        let dy = round_js(7.5 - (min_y + max_y) as f64 / 2.0);

        if dx == 0 && dy == 0 {
            return;
        }

        let old = std::mem::take(&mut self.items);

        for ((x, y), color) in old {
            self.add(
                (x + dx).clamp(0, LOGICAL_SIZE - 1),
                (y + dy).clamp(0, LOGICAL_SIZE - 1),
                color,
            );
        }
    }
}

struct SeededRandom {
    state: u32,
}

impl SeededRandom {
    fn new(seed: &str) -> Self {
        let state = hash_string(seed);
        Self {
            state: if state == 0 { 1 } else { state },
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);

        let mut value = self.state;
        value = (value ^ (value >> 15)).wrapping_mul(value | 1);
        value ^= value.wrapping_add((value ^ (value >> 7)).wrapping_mul(value | 61));

        value ^ (value >> 14)
    }

    fn next_f64(&mut self) -> f64 {
        self.next_u32() as f64 / 4_294_967_296.0
    }

    fn index(&mut self, len: usize) -> usize {
        debug_assert!(len > 0);
        (self.next_f64() * len as f64).floor() as usize
    }
}

fn hash_string(value: &str) -> u32 {
    let mut hash = 2_166_136_261_u32;

    for unit in value.encode_utf16() {
        hash ^= unit as u32;
        hash = hash.wrapping_mul(16_777_619);
    }

    hash
}

fn round_js(value: f64) -> i32 {
    (value + 0.5).floor() as i32
}

const fn rgba(r: u8, g: u8, b: u8) -> [u8; 4] {
    [r, g, b, 255]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_same_seed() {
        let a = generate_flower_png("user-1", FlowerStyle::FlowerOnly).unwrap();
        let b = generate_flower_png("user-1", FlowerStyle::FlowerOnly).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn output_is_32_by_32() {
        let generated = generate_flower_image("user-1", FlowerStyle::FlowerWithStem);
        assert_eq!(generated.image.dimensions(), (32, 32));
    }

    #[test]
    fn output_is_a_png() {
        let bytes = generate_flower_png("user-1", FlowerStyle::FlowerOnly).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }
}
