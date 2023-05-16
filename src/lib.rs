use worker::*;

use std::collections::HashMap;

use anyhow::Context;

use image::{DynamicImage, GenericImageView};

enum ImageSize {
    Preview,   // 64x64
    Thumbnail, // 110x155
    Medium,    // 230x325
    Large,     // 450x635,
}

async fn fetch_image(req: &Request) -> anyhow::Result<image::DynamicImage> {
    let url = urlencoding::decode(&req.path()[1..])
        .context("failed to decode url")?
        .to_string();

    let response = reqwest::get(url).await.context("failed to fetch image")?;

    let image_data = response
        .bytes()
        .await
        .context("failed to get image bytes")?;

    let image = image::load_from_memory(&image_data).context("failed to decode image")?;

    Ok(image)
}

fn resize_image_cover(
    image: DynamicImage,
    desired_width: u32,
    desired_height: u32,
) -> DynamicImage {
    // TODO

    image
}

async fn fetch_image_resize(req: &Request, size: ImageSize) -> anyhow::Result<image::RgbaImage> {
    let image = fetch_image(&req).await?;

    let (width, height) = match size {
        ImageSize::Preview => (64, 64),
        ImageSize::Thumbnail => (110, 155),
        ImageSize::Medium => (230, 325),
        ImageSize::Large => (450, 635),
    };

    let resized_image = resize_image_cover(image, width, height);

    Ok(resized_image.into_rgba8())
}

fn respond_with_image(image: image::RgbaImage) -> Response {
    let mut buf = std::io::Cursor::new(Vec::new());

    let mut headers = Headers::new();

    image
        .write_to(&mut buf, image::ImageOutputFormat::Png)
        .unwrap();

    let data = buf.get_ref().clone();

    headers.set("content-type", "image/png").unwrap();

    headers
        .set("content-length", &data.len().to_string())
        .unwrap();

    Response::from_body(ResponseBody::Body(data))
        .unwrap()
        .with_headers(headers)
}

#[event(fetch)]
async fn main(req: Request, _env: Env, _ctx: worker::Context) -> Result<Response> {
    let hash_query = req
        .url()?
        .query_pairs()
        .into_owned()
        .collect::<HashMap<String, String>>();

    let _blur = hash_query.contains_key("blur");

    let size = match hash_query.get("size").and_then(|x| Some(x.as_str())) {
        Some("preview") => ImageSize::Preview,
        Some("thumbnail") => ImageSize::Thumbnail,
        Some("medium") => ImageSize::Medium,
        _ => ImageSize::Large,
    };

    match fetch_image_resize(&req, size).await {
        Ok(image) => Ok(respond_with_image(image)),
        Err(_) => {
            // TODO
            let image = image::RgbaImage::new(256, 256);

            Ok(respond_with_image(image))
        }
    }
}
