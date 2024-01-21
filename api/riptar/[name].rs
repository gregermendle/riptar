use riptar::{djb2, riptar_svg, render_svg};
use serde::Serialize;
use std::collections::HashMap;
use url::Url;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};

#[derive(Serialize)]
pub struct APIError {
    pub message: &'static str,
    pub code: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    let parsed_url = Url::parse(&req.uri().to_string()).unwrap();
    let hash_query: HashMap<String, String> = parsed_url.query_pairs().into_owned().collect();
    let name_key = hash_query.get("name");
    let format_key = hash_query.get("format");
    let color_key = hash_query.get("color");

    let hash = match name_key {
        Some(name) => djb2(name),
        None => djb2("riptar"),
    };

    let size = 128;
    let svg = riptar_svg(size, hash, color_key.is_some_and(|color| color == "on"));

    if format_key.is_some_and(|s| s == "png") {
        let image = render_svg(&svg, size);
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "image/png")
            .header(
                "Cache-Control",
                "public, immutable, no-transform, max-age=31536000",
            )
            .body(vercel_runtime::Body::Binary(image))?);
    }
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/svg+xml")
        .header(
            "Cache-Control",
            "public, immutable, no-transform, max-age=31536000",
        )
        .body(vercel_runtime::Body::Text(svg))?)
}
