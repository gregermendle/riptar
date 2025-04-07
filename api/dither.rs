use riptar::dither;
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
    let url_key = hash_query.get("url");

    if let Some(url) = url_key {
        if let Ok(dithered) = dither(url) {
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "image/png")
                .header(
                    "Cache-Control",
                    "public, immutable, no-transform, max-age=31536000",
                )
                .body(vercel_runtime::Body::Binary(dithered))?);
        }

        return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(vercel_runtime::Body::Empty)?);
    }

    return Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(vercel_runtime::Body::Empty)?);
}
