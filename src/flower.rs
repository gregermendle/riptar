use image::{
    codecs::{
        gif::{GifEncoder, Repeat},
        png::PngEncoder,
    },
    Delay, ExtendedColorType, Frame, ImageEncoder, Rgba, RgbaImage,
};
use std::collections::HashMap;

const LOGICAL_SIZE: i32 = 16;
const DEFAULT_OUTPUT_SIZE: u32 = 32;
const MIN_OUTPUT_SIZE: u32 = 16;
const MAX_OUTPUT_SIZE: u32 = 256;

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

pub const FLOWER_VARIANT_COUNT: u8 = 5;
pub const FLOWER_ANIMATION_FRAME_COUNT: usize = 8;
pub const DEFAULT_FRAME_DELAY_MS: u32 = 250;

const SWAY_FRAMES: [i32; FLOWER_ANIMATION_FRAME_COUNT] = [-2, -1, 0, 1, 2, 1, 0, -1];

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

pub fn normalize_flower_size(size: Option<u32>) -> u32 {
    let size = size.unwrap_or(DEFAULT_OUTPUT_SIZE).clamp(MIN_OUTPUT_SIZE, MAX_OUTPUT_SIZE);
    ((size + 8) / 16) * 16
}

pub fn normalize_flower_variant(variant: Option<u32>) -> u8 {
    (variant.unwrap_or(0) % FLOWER_VARIANT_COUNT as u32) as u8
}

pub fn generate_flower_image(
    seed: &str,
    style: FlowerStyle,
    size: Option<u32>,
    variant: Option<u32>,
) -> GeneratedFlower {
    let output_size = normalize_flower_size(size);
    let pixel_scale = output_size / LOGICAL_SIZE as u32;
    let include_stem = style == FlowerStyle::FlowerWithStem;
    let generated = generate_flower_pixels(seed, style, variant);
    let mut image = RgbaImage::from_pixel(output_size, output_size, Rgba(generated.background));

    draw_pixels(&mut image, &generated.pixels, pixel_scale, 0, include_stem);

    GeneratedFlower {
        image,
        background: generated.background,
    }
}

fn generate_flower_pixels(
    seed: &str,
    style: FlowerStyle,
    variant: Option<u32>,
) -> GeneratedFlowerPixels {
    let variant = normalize_flower_variant(variant);
    let include_stem = style == FlowerStyle::FlowerWithStem;

    let mut identity_rng = SeededRandom::new(seed);

    let variation_seed = format!("{seed}:variant:{variant}");
    let mut shape_rng = SeededRandom::new(&variation_seed);

    let palette = PETAL_PALETTES[identity_rng.index(PETAL_PALETTES.len())];
    let center_color = CENTER_COLORS[identity_rng.index(CENTER_COLORS.len())];
    let stem_color = STEM_COLORS[identity_rng.index(STEM_COLORS.len())];
    let leaf_color = STEM_COLORS[identity_rng.index(STEM_COLORS.len())];
    let background = BACKGROUNDS[identity_rng.index(BACKGROUNDS.len())];

    let center_x = 8;
    let center_y = if include_stem {
        6 + shape_rng.index(2) as i32
    } else {
        8
    };
    let stem_lean = if shape_rng.next_f64() > 0.5 { 1 } else { -1 };

    let mut pixels = PixelGrid::default();

    for &(offset_x, offset_y) in &PETAL_SHAPE {
        let distance = offset_x.abs() + offset_y.abs();
        let color = if distance >= 4 {
            palette[1]
        } else if shape_rng.next_f64() > 0.72 {
            palette[2]
        } else {
            palette[0]
        };

        pixels.add_mirrored(center_x, center_y, offset_x, offset_y, color);
    }

    if shape_rng.next_f64() > 0.45 {
        pixels.add_mirrored(center_x, center_y, 3, 0, palette[1]);
    }
    if shape_rng.next_f64() > 0.50 {
        pixels.add_mirrored(center_x, center_y, 1, -4, palette[1]);
    }
    if shape_rng.next_f64() > 0.55 {
        pixels.add_mirrored(center_x, center_y, 3, 1, palette[0]);
    }
    if shape_rng.next_f64() > 0.60 {
        pixels.add_mirrored(center_x, center_y, 1, 3, palette[1]);
    }

    if !include_stem {
        let blossom_variant = shape_rng.index(FLOWER_VARIANT_COUNT as usize) as u8;
        add_flower_variant(
            &mut pixels,
            &mut shape_rng,
            center_x,
            center_y,
            palette,
            blossom_variant,
        );

        if shape_rng.next_f64() > 0.55 {
            pixels.add_mirrored(center_x, center_y, 2, -3, palette[1]);
        }
        if shape_rng.next_f64() > 0.60 {
            pixels.add_mirrored(center_x, center_y, 3, -1, palette[2]);
        }
        if shape_rng.next_f64() > 0.65 {
            pixels.add_mirrored(center_x, center_y, 2, 3, palette[1]);
        }
        if shape_rng.next_f64() > 0.70 {
            pixels.add_mirrored(center_x, center_y, 4, 0, palette[1]);
        }
    }

    pixels.add(center_x, center_y, center_color);
    pixels.add(center_x - 1, center_y, center_color);
    pixels.add(center_x, center_y - 1, center_color);
    pixels.add(center_x - 1, center_y - 1, center_color);

    if shape_rng.next_f64() > 0.4 {
        pixels.add(center_x - 1, center_y - 1, CENTER_HIGHLIGHT);
    }

    if include_stem {
        add_stem(
            &mut pixels,
            &mut shape_rng,
            center_x,
            center_y,
            stem_lean,
            stem_color,
            leaf_color,
        );
    } else {
        pixels.recenter();
    }

    GeneratedFlowerPixels { pixels, background }
}

pub fn generate_flower_png(
    seed: &str,
    style: FlowerStyle,
    size: Option<u32>,
    variant: Option<u32>,
) -> Result<Vec<u8>, image::ImageError> {
    let generated = generate_flower_image(seed, style, size, variant);
    encode_png(&generated.image)
}

pub fn generate_flower_animation_frames(
    seed: &str,
    style: FlowerStyle,
    size: Option<u32>,
    variant: Option<u32>,
) -> Vec<GeneratedFlower> {
    let output_size = normalize_flower_size(size);
    let pixel_scale = output_size / LOGICAL_SIZE as u32;
    let include_stem = style == FlowerStyle::FlowerWithStem;

    let generated = generate_flower_pixels(seed, style, variant);

    SWAY_FRAMES
        .iter()
        .map(|&sway| {
            let mut image = RgbaImage::from_pixel(
                output_size,
                output_size,
                Rgba(generated.background),
            );
            draw_pixels(&mut image, &generated.pixels, pixel_scale, sway, include_stem);

            GeneratedFlower {
                image,
                background: generated.background,
            }
        })
        .collect()
}

pub fn generate_flower_gif(
    seed: &str,
    style: FlowerStyle,
    size: Option<u32>,
    variant: Option<u32>,
    frame_delay_ms: Option<u32>,
) -> Result<Vec<u8>, image::ImageError> {
    let frames = generate_flower_animation_frames(seed, style, size, variant);
    let delay_ms = frame_delay_ms.unwrap_or(DEFAULT_FRAME_DELAY_MS).max(20);
    let mut bytes = Vec::new();

    {
        let mut encoder = GifEncoder::new(&mut bytes);
        encoder.set_repeat(Repeat::Infinite)?;

        for generated in frames {
            let delay = Delay::from_numer_denom_ms(delay_ms, 1);
            encoder.encode_frame(Frame::from_parts(generated.image, 0, 0, delay))?;
        }
    }

    Ok(bytes)
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

fn add_flower_variant(
    pixels: &mut PixelGrid,
    rng: &mut SeededRandom,
    center_x: i32,
    center_y: i32,
    palette: [[u8; 4]; 3],
    variant: u8,
) {
    match variant % FLOWER_VARIANT_COUNT {
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

            let x = if rng.next_f64() > 0.5 {
                center_x - 1
            } else {
                center_x + 1
            };
            pixels.add(x, center_y + 3, palette[0]);
        }
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
        pixels.add_part(stem_x, y, stem_color, PixelPart::Stem);
    }

    let leaf_y = center_y + 5 + random.index(3) as i32;
    let leaf_direction = if random.next_f64() > 0.5 { 1 } else { -1 };

    pixels.add_part(stem_x + leaf_direction, leaf_y, leaf_color, PixelPart::Stem);
    pixels.add_part(stem_x + leaf_direction * 2, leaf_y - 1, leaf_color, PixelPart::Stem);
    pixels.add_part(stem_x + leaf_direction * 2, leaf_y, leaf_color, PixelPart::Stem);

    if random.next_f64() > 0.45 {
        let opposite_y = (leaf_y + 2).min(13);
        pixels.add_part(stem_x - leaf_direction, opposite_y, leaf_color, PixelPart::Stem);
        pixels.add_part(
            stem_x - leaf_direction * 2,
            opposite_y - 1,
            leaf_color,
            PixelPart::Stem,
        );
    }

    pixels.add_part(stem_x - 1, 15, stem_color, PixelPart::Base);
    pixels.add_part(stem_x, 15, stem_color, PixelPart::Base);
    pixels.add_part(stem_x + 1, 15, stem_color, PixelPart::Base);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PixelPart {
    Flower,
    Stem,
    Base,
}

#[derive(Debug, Clone, Copy)]
struct Pixel {
    color: [u8; 4],
    part: PixelPart,
}

#[derive(Default)]
struct PixelGrid {
    items: HashMap<(i32, i32), Pixel>,
}

impl PixelGrid {
    fn add(&mut self, x: i32, y: i32, color: [u8; 4]) {
        self.add_part(x, y, color, PixelPart::Flower);
    }

    fn add_part(&mut self, x: i32, y: i32, color: [u8; 4], part: PixelPart) {
        if (0..LOGICAL_SIZE).contains(&x) && (0..LOGICAL_SIZE).contains(&y) {
            self.items.insert((x, y), Pixel { color, part });
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

        for ((x, y), pixel) in old {
            let new_x = (x + dx).clamp(0, LOGICAL_SIZE - 1);
            let new_y = (y + dy).clamp(0, LOGICAL_SIZE - 1);
            self.items.insert((new_x, new_y), pixel);
        }
    }
}

struct GeneratedFlowerPixels {
    pixels: PixelGrid,
    background: [u8; 4],
}

fn transform_pixel(x: i32, y: i32, part: PixelPart, sway: i32, include_stem: bool) -> (i32, i32) {
    if !include_stem {
        return (x + round_js(sway as f64 * 0.35), y);
    }

    let flower_shift_x = sway;
    let flower_shift_y = if sway.abs() == 2 { 1 } else { 0 };

    match part {
        PixelPart::Flower => (x + flower_shift_x, y + flower_shift_y),
        PixelPart::Stem => {
            let progress = ((15 - y) as f64 / 6.0).clamp(0.0, 1.0);
            (
                x + round_js(flower_shift_x as f64 * progress),
                y + round_js(flower_shift_y as f64 * progress),
            )
        }
        PixelPart::Base => (x, y),
    }
}

fn draw_pixels(
    image: &mut RgbaImage,
    pixels: &PixelGrid,
    pixel_scale: u32,
    sway: i32,
    include_stem: bool,
) {
    for (&(x, y), pixel) in &pixels.items {
        let (x, y) = transform_pixel(x, y, pixel.part, sway, include_stem);
        if !(0..LOGICAL_SIZE).contains(&x) || !(0..LOGICAL_SIZE).contains(&y) {
            continue;
        }

        let start_x = x as u32 * pixel_scale;
        let start_y = y as u32 * pixel_scale;

        for dy in 0..pixel_scale {
            for dx in 0..pixel_scale {
                image.put_pixel(start_x + dx, start_y + dy, Rgba(pixel.color));
            }
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
    fn deterministic_for_same_seed_and_variant() {
        let a = generate_flower_png("user-1", FlowerStyle::FlowerOnly, None, Some(2)).unwrap();
        let b = generate_flower_png("user-1", FlowerStyle::FlowerOnly, None, Some(2)).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_variants_produce_different_images() {
        let a = generate_flower_png("user-1", FlowerStyle::FlowerOnly, None, Some(1)).unwrap();
        let b = generate_flower_png("user-1", FlowerStyle::FlowerOnly, None, Some(2)).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn variants_wrap_correctly() {
        assert_eq!(
            generate_flower_png("user-1", FlowerStyle::FlowerOnly, None, Some(0)).unwrap(),
            generate_flower_png(
                "user-1",
                FlowerStyle::FlowerOnly,
                None,
                Some(FLOWER_VARIANT_COUNT as u32),
            )
            .unwrap(),
        );
    }

    #[test]
    fn background_is_seed_driven_not_variant_driven() {
        let a = generate_flower_image("user-1", FlowerStyle::FlowerOnly, None, Some(0));
        let b = generate_flower_image("user-1", FlowerStyle::FlowerOnly, None, Some(3));
        assert_eq!(a.background, b.background);
    }

    #[test]
    fn output_is_32_by_32_by_default() {
        let generated =
            generate_flower_image("user-1", FlowerStyle::FlowerWithStem, None, None);
        assert_eq!(generated.image.dimensions(), (32, 32));
    }

    #[test]
    fn output_respects_custom_size() {
        let generated = generate_flower_image("user-1", FlowerStyle::FlowerOnly, Some(64), None);
        assert_eq!(generated.image.dimensions(), (64, 64));
    }

    #[test]
    fn output_is_a_png() {
        let bytes = generate_flower_png("user-1", FlowerStyle::FlowerOnly, None, None).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn all_variants_generate_within_canvas() {
        for variant in 0..FLOWER_VARIANT_COUNT {
            for style in [FlowerStyle::FlowerOnly, FlowerStyle::FlowerWithStem] {
                let generated =
                    generate_flower_image("user-1", style, None, Some(variant as u32));
                assert_eq!(generated.image.dimensions(), (32, 32));
            }
        }
    }

    #[test]
    fn animation_has_expected_frame_count_and_size() {
        let frames = generate_flower_animation_frames(
            "user-1",
            FlowerStyle::FlowerWithStem,
            Some(64),
            Some(2),
        );

        assert_eq!(frames.len(), FLOWER_ANIMATION_FRAME_COUNT);
        assert!(frames.iter().all(|frame| frame.image.dimensions() == (64, 64)));
    }

    #[test]
    fn animation_is_deterministic() {
        let a = generate_flower_animation_frames(
            "user-1",
            FlowerStyle::FlowerWithStem,
            None,
            Some(2),
        );
        let b = generate_flower_animation_frames(
            "user-1",
            FlowerStyle::FlowerWithStem,
            None,
            Some(2),
        );

        assert_eq!(a.len(), b.len());
        for (left, right) in a.iter().zip(&b) {
            assert_eq!(left.image, right.image);
        }
    }
}
