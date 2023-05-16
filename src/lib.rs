use worker::*;

use std::collections::HashMap;

use anyhow::Context;

use image::{DynamicImage, GenericImageView, RgbaImage};

#[derive(Debug)]
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

    // TODO cache and load from cache

    let response = reqwest::get(url).await.context("failed to fetch image")?;

    let content_type = response
        .headers()
        .get("Content-Type")
        .and_then(|x| x.to_str().ok());

    console_log!("{}, {:?}", response.status(), content_type);

    match content_type {
        Some("image/png") => (),
        Some("image/jpeg") => (),
        Some("image/webp") => (),
        _ => return Err(anyhow::anyhow!("image type no allowed")),
    }

    let image_data = response
        .bytes()
        .await
        .context("failed to get image bytes")?;

    let image = image::load_from_memory(&image_data).context("failed to decode image")?;

    Ok(image)
}

fn resize_fit_cover(image: DynamicImage, desired_width: u32, desired_height: u32) -> RgbaImage {
    let (width, height) = image.dimensions();

    let scale_factor = if desired_width / desired_height > width / height {
        desired_width as f32 / width as f32
    } else {
        desired_height as f32 / height as f32
    };

    let new_width = std::cmp::max((width as f32 * scale_factor) as u32, desired_width);
    let new_height = std::cmp::max((height as f32 * scale_factor) as u32, desired_height);

    let crop_x = (new_width - desired_width) / 2;
    let crop_y = (new_height - desired_height) / 2;

    console_log!(
        "{}, {}, {}, {}",
        new_width,
        new_height,
        new_width as i32 - desired_width as i32,
        new_height as i32 - desired_height as i32
    );

    let mut resized_img = image::imageops::resize(
        &image,
        new_width,
        new_height,
        image::imageops::FilterType::Lanczos3,
    );

    image::imageops::crop(
        &mut resized_img,
        crop_x,
        crop_y,
        desired_width,
        desired_height,
    )
    .to_image()
}

async fn fetch_image_resize(
    req: &Request,
    size: &ImageSize,
    blur: bool,
) -> anyhow::Result<image::RgbaImage> {
    let image = fetch_image(&req).await?;

    let (width, height) = match size {
        ImageSize::Preview => (64, 64),
        ImageSize::Thumbnail => (110, 155),
        ImageSize::Medium => (230, 325),
        ImageSize::Large => (450, 635),
    };

    let resized_image = resize_fit_cover(image, width, height);

    if blur {
        Ok(image::imageops::blur(&resized_image, 15.0))
    } else {
        Ok(resized_image)
    }
}

fn respond_with_image(image: image::RgbaImage) -> anyhow::Result<Response> {
    let mut buf = std::io::Cursor::new(Vec::new());

    let mut headers = Headers::new();

    image
        .write_to(&mut buf, image::ImageOutputFormat::Png)
        .context("failed to encode image to png")?;

    let data = buf.get_ref().clone();

    match headers.set("content-type", "image/png") {
        Ok(_) => (),
        Err(_) => return Err(anyhow::anyhow!("failed to write content-type header")),
    };

    match headers.set("content-length", &data.len().to_string()) {
        Ok(_) => (),
        Err(_) => return Err(anyhow::anyhow!("failed to write content-length header")),
    };

    match Response::from_body(ResponseBody::Body(data)) {
        Ok(response) => Ok(response.with_headers(headers)),
        Err(_) => Err(anyhow::anyhow!("failed to write response body")),
    }
}

#[event(fetch)]
async fn main(req: Request, _env: Env, _ctx: worker::Context) -> Result<Response> {
    let hash_query = req
        .url()?
        .query_pairs()
        .into_owned()
        .collect::<HashMap<String, String>>();

    let blur = hash_query.contains_key("blur");

    let size = match hash_query.get("size").and_then(|x| Some(x.as_str())) {
        Some("preview") => ImageSize::Preview,
        Some("thumbnail") => ImageSize::Thumbnail,
        Some("medium") => ImageSize::Medium,
        _ => ImageSize::Large,
    };

    console_log!("size: {:?}, blur: {}", size, blur);

    match fetch_image_resize(&req, &size, blur).await {
        Ok(image) => Ok(respond_with_image(image).unwrap()),
        Err(_) => {
            let default_image: &[u8] = match &size {
                ImageSize::Preview | ImageSize::Thumbnail => {
                    include_bytes!("../default/thumbnail.png")
                }
                _ => include_bytes!("../default/medium.png"),
            };

            let image = image::load_from_memory(default_image).unwrap().to_rgba8();

            Ok(respond_with_image(image).unwrap())
        }
    }
}
