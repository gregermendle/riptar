use image::{imageops::FilterType, GrayImage, ImageFormat, Luma};
use resvg::{
    tiny_skia::Pixmap,
    usvg::{self, Options, Transform, TreeParsing},
    Tree,
};
use std::collections::HashMap;
use std::io::Cursor;
use url::Url;
use worker::*;

fn cache_headers() -> Headers {
    let h = Headers::new();
    h.set("Cache-Control", "public, immutable, no-transform, max-age=31536000")
        .unwrap();
    h
}

#[event(fetch, respond_with_errors)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        .get_async("/dither", |req, _ctx| async move {
            let url = req.url()?;
            let query: HashMap<String, String> =
                url.query_pairs().into_owned().collect();
            let url_param = query.get("url");
            let width = query.get("width").and_then(|s| s.parse::<u32>().ok());
            let height = query.get("height").and_then(|s| s.parse::<u32>().ok());

            if let Some(image_url) = url_param {
                let bytes = get_image_bytes(image_url).await;
                match bytes {
                    Ok(bytes) => match dither_from_bytes(&bytes, width, height) {
                        Ok(dithered) => {
                            let h = cache_headers();
                            h.set("Content-Type", "image/png").unwrap();
                            return Ok(Response::from_bytes(dithered)?
                                .with_headers(h));
                        }
                        Err(_) => return Response::error("Internal error", 500),
                    },
                    Err(_) => return Response::error("Internal error", 500),
                }
            }
            Response::error("Bad request", 400)
        })
        .get_async("/riptar/:name", |req, ctx| async move {
            let name = ctx
                .param("name")
                .map(|s| s.as_str())
                .unwrap_or("riptar");
            let url = req.url()?;
            let query: HashMap<String, String> =
                url.query_pairs().into_owned().collect();
            let format_key = query.get("format");
            let color_key = query.get("color");

            let hash = djb2(name);
            let size = 128;
            let svg = riptar_svg(
                size,
                hash,
                color_key.is_some_and(|c| c == "on"),
            );

            if format_key.is_some_and(|s| s == "png") {
                let image = render_svg(&svg, size);
                let h = cache_headers();
                h.set("Content-Type", "image/png").unwrap();
                return Ok(Response::from_bytes(image)?.with_headers(h));
            }

            let h = cache_headers();
            h.set("Content-Type", "image/svg+xml").unwrap();
            Ok(Response::ok(svg)?.with_headers(h))
        })
        .get_async("/riptar", |req, _ctx| async move {
            let mut u = req.url()?.clone();
            u.set_path("/riptar/riptar");
            u.set_query(None);
            Response::redirect(u)
        })
        .get_async("/", |req, ctx| async move {
            let assets = ctx.env.assets("ASSETS")?;
            assets.fetch_request(req).await
        })
        .get_async("/*path", |req, ctx| async move {
            let assets = ctx.env.assets("ASSETS")?;
            assets.fetch_request(req).await
        })
        .run(req, env)
        .await
}

async fn get_image_bytes(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if let Some(bytes) = try_riptar_bytes(url) {
        return Ok(bytes);
    }
    fetch_image(url).await
}

fn try_riptar_bytes(url: &str) -> Option<Vec<u8>> {
    let parsed = Url::parse(url).ok()?;
    let path = parsed.path();
    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let name = match path_segments.as_slice() {
        ["riptar"] => "riptar",
        ["riptar", name] => name,
        _ => return None,
    };
    let format = parsed.query_pairs().find(|(k, _)| k == "format");
    if format.as_ref().map(|(_, v)| v.as_ref()) != Some("png") {
        return None;
    }
    let color = parsed
        .query_pairs()
        .find(|(k, _)| k == "color")
        .map(|(_, v)| v.as_ref() == "on")
        .unwrap_or(false);
    let hash = djb2(name);
    let svg = riptar_svg(128, hash, color);
    Some(render_svg(&svg, 128))
}

async fn fetch_image(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let resp = reqwest::get(url).await?;
    let bytes = resp.bytes().await?.to_vec();
    Ok(bytes)
}

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

fn dither_from_bytes(
    bytes: &[u8],
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut img = image::load_from_memory(bytes)?;

    if let (Some(w), Some(h)) = (width, height) {
        img = img.resize(w, h, FilterType::Triangle);
    }

    let grayscale = img.to_luma8();
    let mut dithered = grayscale.clone();
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

            for (dx, dy, weight) in [(1, 0, 7), (-1, 1, 3), (0, 1, 5), (1, 1, 1)]
            {
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
