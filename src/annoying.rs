use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::imageops::FilterType;
use std::io::Write;

const HLG_A: f64 = 0.17883277;
const HLG_B: f64 = 0.28466892;
const HLG_C: f64 = 0.55991073;

pub fn annoying_from_bytes(
    bytes: &[u8],
    width: Option<u32>,
    height: Option<u32>,
    amount: f64,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let amount = amount.clamp(0.0, 1.0);
    let peak_linear = highlight_cap(amount);
    let peak_signal = hlg_oetf(peak_linear);

    let mut img = image::load_from_memory(bytes)?;

    if let (Some(w), Some(h)) = (width, height) {
        img = img.resize(w, h, FilterType::Triangle);
    }

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut rgb16 = Vec::with_capacity((width * height * 3) as usize);

    for y in 0..height {
        for x in 0..width {
            let [r, g, b, a] = rgba.get_pixel(x, y).0;
            if a == 0 {
                rgb16.extend_from_slice(&[0, 0, 0]);
                continue;
            }
            let (hr, hg, hb) = to_hdr_hlg_channels(r, g, b, amount, peak_linear);
            rgb16.push(hlg_to_u16(hr, peak_signal));
            rgb16.push(hlg_to_u16(hg, peak_signal));
            rgb16.push(hlg_to_u16(hb, peak_signal));
        }
    }

    Ok(encode_hdr_png(width, height, &rgb16, amount))
}

fn highlight_cap(amount: f64) -> f64 {
    1.0 + amount * amount * 2.5
}

fn srgb_to_linear(channel: f64) -> f64 {
    let c = channel / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn hlg_oetf(linear: f64) -> f64 {
    let e = linear.max(0.0);
    if e <= 1.0 / 12.0 {
        e * 3.0_f64.sqrt()
    } else {
        HLG_A * (12.0 * e - HLG_B).ln() + HLG_C
    }
}

fn hlg_to_u16(hlg: f64, peak_signal: f64) -> u16 {
    let peak = peak_signal.max(1e-8);
    (hlg / peak * 65535.0).round().clamp(0.0, 65535.0) as u16
}

fn rec709_luminance(r: f64, g: f64, b: f64) -> f64 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn to_hdr_hlg_channels(
    r: u8,
    g: u8,
    b: u8,
    amount: f64,
    peak_linear: f64,
) -> (f64, f64, f64) {
    let lr = srgb_to_linear(r as f64);
    let lg = srgb_to_linear(g as f64);
    let lb = srgb_to_linear(b as f64);

    let lum = rec709_luminance(lr, lg, lb);
    let boosted_lum = boost_luminance(lum, amount);

    let (nr, ng, nb) = if lum > 1e-8 {
        let scale = boosted_lum / lum;
        (
            (lr * scale).clamp(0.0, peak_linear),
            (lg * scale).clamp(0.0, peak_linear),
            (lb * scale).clamp(0.0, peak_linear),
        )
    } else {
        (lr, lg, lb)
    };

    (hlg_oetf(nr), hlg_oetf(ng), hlg_oetf(nb))
}

fn boost_luminance(y: f64, amount: f64) -> f64 {
    if y <= 1e-8 || amount <= 0.0 {
        return y;
    }

    let cap = highlight_cap(amount);
    let lift = 1.0 + amount * 0.04;
    let knee = 0.72;

    let boosted = if y <= knee {
        let t = y / knee;
        y * (1.0 + (lift - 1.0) * t)
    } else {
        let base = knee * (1.0 + (lift - 1.0));
        let t = ((y - knee) / (1.0 - knee)).clamp(0.0, 1.0);
        base + t.powf(2.0) * (cap - base)
    };

    boosted.max(y)
}

fn encode_hdr_png(width: u32, height: u32, rgb16: &[u16], amount: f64) -> Vec<u8> {
    let mut raw = Vec::with_capacity(((width * 3 * 2 + 1) * height) as usize);
    for y in 0..height {
        raw.push(0);
        let row_start = (y * width * 3) as usize;
        for x in 0..(width * 3) as usize {
            let sample = rgb16[row_start + x as usize];
            raw.extend_from_slice(&sample.to_be_bytes());
        }
    }

    let mut compressed = Vec::new();
    {
        let mut encoder = ZlibEncoder::new(&mut compressed, Compression::default());
        encoder.write_all(&raw).unwrap();
        encoder.finish().unwrap();
    }

    let mut png = Vec::new();
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(16);
    ihdr.push(2);
    ihdr.extend_from_slice(&[0, 0, 0]);
    write_chunk(&mut png, b"IHDR", &ihdr);

    write_chunk(&mut png, b"cICP", &[12, 18, 0, 1]);
    write_chunk(&mut png, b"mDCV", &mastering_display());
    write_chunk(&mut png, b"cLLI", &content_light_level(amount));

    write_chunk(&mut png, b"IDAT", &compressed);
    write_chunk(&mut png, b"IEND", &[]);

    png
}

fn mastering_display() -> [u8; 24] {
    let mut data = [0_u8; 24];
    put_u16(&mut data[0..2], 13250);
    put_u16(&mut data[2..4], 34500);
    put_u16(&mut data[4..6], 7500);
    put_u16(&mut data[6..8], 3000);
    put_u16(&mut data[8..10], 34000);
    put_u16(&mut data[10..12], 16000);
    put_u16(&mut data[12..14], 15635);
    put_u16(&mut data[14..16], 16450);
    put_u32(&mut data[16..20], 10_000_000);
    put_u32(&mut data[20..24], 1);
    data
}

fn content_light_level(amount: f64) -> [u8; 8] {
    let max_nits = 100.0 + amount * amount * 900.0;
    let avg_nits = 100.0 + amount * amount * 250.0;
    let mut data = [0_u8; 8];
    put_u32(&mut data[0..4], (max_nits * 10000.0) as u32);
    put_u32(&mut data[4..8], (avg_nits * 10000.0) as u32);
    data
}

fn put_u16(out: &mut [u8], value: u32) {
    out[0..2].copy_from_slice(&(value as u16).to_be_bytes());
}

fn put_u32(out: &mut [u8], value: u32) {
    out[0..4].copy_from_slice(&value.to_be_bytes());
}

fn write_chunk(output: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(chunk_type);
    output.extend_from_slice(data);
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(chunk_type);
    hasher.update(data);
    output.extend_from_slice(&hasher.finalize().to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, ImageFormat, Rgba, RgbaImage};

    #[test]
    fn boost_never_darkens() {
        for i in 1..255 {
            let y = srgb_to_linear(i as f64);
            for amount in [0.25, 0.5, 1.0] {
                assert!(boost_luminance(y, amount) >= y * 0.999);
            }
        }
    }

    #[test]
    fn amount_zero_is_unchanged() {
        assert!((boost_luminance(0.8, 0.0) - 0.8).abs() < 1e-6);
    }

    #[test]
    fn amount_scales_smoothly() {
        let a25 = boost_luminance(1.0, 0.25);
        let a50 = boost_luminance(1.0, 0.5);
        let a100 = boost_luminance(1.0, 1.0);
        assert!(a25 < a50 && a50 < a100);
        assert!(a25 < 1.3);
        assert!(a100 < 4.0);
    }

    #[test]
    fn hlg_encoding_preserves_highlight_detail() {
        let peak = hlg_oetf(highlight_cap(1.0));
        let bright = hlg_to_u16(hlg_oetf(3.0), peak);
        let medium = hlg_to_u16(hlg_oetf(1.5), peak);
        let dim = hlg_to_u16(hlg_oetf(0.5), peak);
        assert!(bright > medium && medium > dim);
        assert!(bright < 65535);
    }

    #[test]
    fn neutral_gray_stays_neutral() {
        let (r, g, b) = to_hdr_hlg_channels(128, 128, 128, 1.0, highlight_cap(1.0));
        assert!((r - g).abs() < 0.001);
        assert!((g - b).abs() < 0.001);
    }

    #[test]
    fn output_is_hdr_png_with_hlg_cicp() {
        let img: RgbaImage = ImageBuffer::from_pixel(2, 2, Rgba([200, 120, 80, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Png).unwrap();

        let out = annoying_from_bytes(&buf.into_inner(), None, None, 1.0).unwrap();
        assert!(out.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(out.windows(4).any(|w| w == b"cICP"));
    }
}
