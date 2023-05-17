use fast_image_resize as fr;
use fastblur::gaussian_blur;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb, RgbaImage};
use std::{collections::HashMap, num::NonZeroU32};
use worker::*;

#[derive(Debug)]
enum ImageSize {
    Preview,   // 64x64
    Thumbnail, // 110x155
    Medium,    // 230x325
    Large,     // 450x635,
}

async fn fetch_image(req: &Request) -> anyhow::Result<DynamicImage> {
    let url = urlencoding::decode(&req.path()[1..])?.to_string();

    // TODO cache and load from cache

    let response = reqwest::get(url).await?;

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

    let image_data = response.bytes().await?;

    let image = image::load_from_memory(&image_data)?;

    Ok(image)
}

fn resize_fit_cover(
    image: DynamicImage,
    desired_width: u32,
    desired_height: u32,
) -> anyhow::Result<RgbaImage> {
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

    let mut src_image = fr::Image::from_vec_u8(
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
        image.to_rgba8().into_raw(),
        fr::PixelType::U8x4,
    )?;

    let alpha_mul_div = fr::MulDiv::default();

    alpha_mul_div.multiply_alpha_inplace(&mut src_image.view_mut())?;

    let mut dst_image = fr::Image::new(
        NonZeroU32::new(new_width).unwrap(),
        NonZeroU32::new(new_height).unwrap(),
        src_image.pixel_type(),
    );

    let mut dst_view = dst_image.view_mut();

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(fr::FilterType::Hamming));

    resizer.resize(&src_image.view(), &mut dst_view)?;

    alpha_mul_div.divide_alpha_inplace(&mut dst_view)?;

    let image_buffer =
        RgbaImage::from_raw(new_width, new_height, dst_image.buffer().to_vec()).unwrap();

    let mut resized_dynamic_image = DynamicImage::ImageRgba8(image_buffer);

    Ok(image::imageops::crop(
        &mut resized_dynamic_image,
        crop_x,
        crop_y,
        desired_width,
        desired_height,
    )
    .to_image())
}

async fn fetch_image_resize(
    req: &Request,
    size: &ImageSize,
    blur: bool,
) -> anyhow::Result<DynamicImage> {
    let image = fetch_image(&req).await?;

    let (width, height) = match size {
        ImageSize::Preview => (64, 64),
        ImageSize::Thumbnail => (110, 155),
        ImageSize::Medium => (230, 325),
        ImageSize::Large => (450, 635),
    };

    let img = DynamicImage::ImageRgba8(resize_fit_cover(image, width, height)?);

    if blur {
        let (width, height) = img.dimensions();

        let mut data: Vec<[u8; 3]> = img
            .pixels()
            .map(|(_, _, pixel)| {
                let [r, g, b, _] = pixel.0;
                [r, g, b]
            })
            .collect();

        gaussian_blur(&mut data, width as usize, height as usize, 10.0);

        let blurred_image: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(width, height, |x, y| {
                let pixel = data[(y * width + x) as usize];
                Rgb([pixel[0], pixel[1], pixel[2]])
            });

        Ok(DynamicImage::ImageRgb8(blurred_image))
    } else {
        Ok(img)
    }
}

fn respond_with_image(image: DynamicImage) -> anyhow::Result<Response> {
    let mut buf = std::io::Cursor::new(Vec::new());

    let mut headers = Headers::new();

    image.write_to(&mut buf, image::ImageOutputFormat::Png)?;

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

            let image = image::load_from_memory(default_image).unwrap();

            Ok(respond_with_image(image).unwrap())
        }
    }
}
