use anyhow::Context;
use fast_image_resize as fr;
use fastblur::gaussian_blur;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb, RgbaImage};
use std::{collections::HashMap, num::NonZeroU32, str::FromStr};
use url::Url;
use wasm_bindgen::prelude::*;
use web_sys::{Headers, Request, Response, ResponseInit};

#[derive(Debug)]
enum ImageSize {
    Preview,   // 64x64
    Thumbnail, // 110x155
    Medium,    // 230x325
    Large,     // 450x635,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

async fn fetch_image(url: &Url) -> anyhow::Result<DynamicImage> {
    let image_url = urlencoding::decode(&url.path()[1..])
        .context(format!("failed to decode url {}", url.path()))?
        .to_string();

    let response = reqwest::get(image_url)
        .await
        .context(format!("failed to fetch {}", url.path()))?;

    let content_type = response
        .headers()
        .get("Content-Type")
        .and_then(|x| x.to_str().ok());

    console_log!("{}, {:?}, {}", response.status(), content_type, url.path());

    match content_type {
        Some("image/png") => (),
        Some("image/jpeg") => (),
        Some("image/webp") => (),
        Some(s) => return Err(anyhow::anyhow!(format!("image type {} no allowed", s))),
        None => return Err(anyhow::anyhow!("No content-type header")),
    }

    let image_data = response
        .bytes()
        .await
        .context("failed to convent response body to bytes")?;

    let image =
        image::load_from_memory(&image_data).context("failed to laod image from response body")?;

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

    // console_log!(
    //     "{}, {}, {}, {}",
    //     new_width,
    //     new_height,
    //     new_width as i32 - desired_width as i32,
    //     new_height as i32 - desired_height as i32
    // );

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
    url: &Url,
    size: &ImageSize,
    blur: bool,
) -> anyhow::Result<DynamicImage> {
    let image = fetch_image(&url).await?;

    let (width, height) = match size {
        ImageSize::Preview => (64, 64),
        ImageSize::Thumbnail => (110, 155),
        ImageSize::Medium => (230, 325),
        ImageSize::Large => (450, 635),
    };

    let img = DynamicImage::ImageRgba8(
        resize_fit_cover(image, width, height).context("failed to resize image")?,
    );

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
    let headers = match Headers::new() {
        Ok(x) => x,
        Err(_) => return Err(anyhow::anyhow!("failed to set headers")),
    };

    let mut buf = std::io::Cursor::new(Vec::new());

    image
        .write_to(&mut buf, image::ImageOutputFormat::Png)
        .context("failed to encode resized image")?;

    let mut data = buf.get_ref().clone();

    match headers.set("content-type", "image/png") {
        Ok(_) => (),
        Err(_) => return Err(anyhow::anyhow!("failed to set headers")),
    };

    match headers.set("content-length", &data.len().to_string()) {
        Ok(_) => (),
        Err(_) => return Err(anyhow::anyhow!("failed to set headers")),
    };

    let response = match Response::new_with_opt_u8_array_and_init(Some(&mut data), &{
        let mut init = ResponseInit::new();
        init.headers(&headers);
        init
    }) {
        Ok(x) => x,
        Err(_) => return Err(anyhow::anyhow!("failed to create new response object")),
    };

    Ok(response)
}

#[wasm_bindgen]
pub async fn handler(request: Request) -> Response {
    console_error_panic_hook::set_once();

    let url = Url::from_str(&request.url()).unwrap();

    let hash_query = url
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

    // console_log!("size: {:?}, blur: {}", size, blur);

    match fetch_image_resize(&url, &size, blur).await {
        Ok(image) => respond_with_image(image).unwrap(),
        Err(_err) => {
            // console_log!("{:?}", _err);

            let default_image: &[u8] = match &size {
                ImageSize::Preview | ImageSize::Thumbnail => {
                    include_bytes!("../default/thumbnail.png")
                }
                _ => include_bytes!("../default/medium.png"),
            };

            let image = image::load_from_memory(default_image).unwrap();

            respond_with_image(image).unwrap()
        }
    }
}
