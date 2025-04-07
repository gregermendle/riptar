use resvg::{
    tiny_skia::Pixmap,
    usvg::{self, Options, Transform, TreeParsing},
    Tree,
};
use reqwest::blocking::get;
use image::{GrayImage, ImageFormat, Luma};
use std::io::Cursor;

pub fn riptar_svg(size: u32, hash: u32, is_color: bool) -> String {
    let l = hash % 360;
    let mut color = "#111111ff".to_string();

    if is_color {
        color = format!("hsl({l}, 35%, 30%)");
    }

    format!(
        "<svg
        xmlns=\"http://www.w3.org/2000/svg\"
        version=\"1.1\"
        viewBox=\"0 0 48 48\"
        width=\"{size}\"
        height=\"{size}\"
    >
        <defs>
        <filter
            id=\"nnnoise-filter\"
            x=\"0%\"
            y=\"0%\"
            width=\"100%\"
            height=\"100%\"
            filterUnits=\"objectBoundingBox\"
            primitiveUnits=\"userSpaceOnUse\"
            colorInterpolationFilters=\"linearRGB\"
        >
            <feTurbulence
                type=\"fractalNoise\"
                baseFrequency=\"0.05\"
                numOctaves=\"2\"
                seed=\"{hash}\"
                stitchTiles=\"stitch\"
                x=\"0%\"
                y=\"0%\"
                width=\"100%\"
                height=\"100%\"
                result=\"turbulence\"
                ></feTurbulence>
                <feSpecularLighting
                surfaceScale=\"15\"
                specularConstant=\"0.75\"
                specularExponent=\"20\"
                lightingColor=\"#fff\"
                x=\"0%\"
                y=\"0%\"
                width=\"100%\"
                height=\"100%\"
                in=\"turbulence\"
                result=\"specularLighting\"
            >
            <feDistantLight azimuth=\"3\" elevation=\"100\"></feDistantLight>
            </feSpecularLighting>
        </filter>
        </defs>
        <rect width=\"48\" height=\"48\" fill=\"{color}\"></rect>
        <rect
            width=\"48\"
            height=\"48\"
            fill=\"#ffffff\"
            filter=\"url(#nnnoise-filter)\"
        ></rect>
    </svg>"
    )
}

pub fn render_svg(svg: &str, size: u32) -> Vec<u8> {
    let usvg_tree = usvg::Tree::from_str(svg, &Options::default()).unwrap();
    let tree = Tree::from_usvg(&usvg_tree);
    let mut pixmap = Pixmap::new(size, size).unwrap();
    tree.render(Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().unwrap()
}

pub fn djb2(str: &str) -> u32 {
    str.chars()
        .map(|c| c.to_digit(16).unwrap_or(1))
        .fold(5381, |hash, c| ((hash << 5) + hash) + c)
}

pub fn dither(url: &String) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let response = get(url)?;
    let bytes = response.bytes()?;
    let img = image::load_from_memory(&bytes)?.to_luma8();

    let mut dithered = img.clone();
    floyd_steinberg_dither(&mut dithered);

    let mut buf = Cursor::new(Vec::new());
    dithered.write_to(&mut buf, ImageFormat::Png)?;
    Ok(buf.into_inner())
}

fn floyd_steinberg_dither(img: &mut GrayImage) {
    let (width, height) = img.dimensions();

    for y in 0..height {
        for x in 0..width {
            let old_pixel = img.get_pixel(x, y)[0];
            let new_pixel = if old_pixel < 128 { 0 } else { 255 };
            let quant_error = old_pixel as i16 - new_pixel as i16;

            img.put_pixel(x, y, Luma([new_pixel]));

            for (dx, dy, weight) in [
                (1, 0, 7),
                (-1, 1, 3),
                (0, 1, 5),
                (1, 1, 1),
            ] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                    let neighbor = img.get_pixel(nx as u32, ny as u32)[0];
                    let adjusted = neighbor as i16 + quant_error * weight / 16;
                    let clamped = adjusted.clamp(0, 255) as u8;
                    img.put_pixel(nx as u32, ny as u32, Luma([clamped]));
                }
            }
        }
    }
}