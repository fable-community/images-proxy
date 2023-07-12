use anyhow::Context;
use fast_image_resize as fr;
use image::{DynamicImage, GenericImageView, RgbaImage};
use js_sys::Uint8Array;
use std::{cmp::max, num::NonZeroU32, str::FromStr};
use url::Url;
use wasm_bindgen::prelude::*;

#[derive(Debug)]
enum ImageSize {
    Preview,   // 32x32
    Thumbnail, // 110x155
    Medium,    // 230x325
    Large,     // 450x635,
}

#[derive(Debug)]
enum ImageFormat {
    Png,
    Jpeg,
    WebP,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

fn trim_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

async fn fetch_image(url: &str) -> anyhow::Result<(DynamicImage, ImageFormat)> {
    let mut response = reqwest::get(url)
        .await
        .context(format!("failed to fetch {}", url))?;

    let mut content_type = response
        .headers()
        .get("Content-Type")
        .and_then(|x| x.to_str().ok());

    // if response is a html page
    // check tags for a og:image url
    if content_type.map(|s| s.starts_with("text/html")) == Some(true) {
        let xml = trim_text(&response.text().await?);

        let re = regex::Regex::new(r#"<meta.*?property="og:image".*?content="(.*?)""#)?;

        if let Some(cap) = re.captures(&xml) {
            let mut url = Url::from_str(&cap[1])?;

            // imgur has ?fb query that cuts images
            // it's save to removed to retrieve the original uncut image
            if url.host_str() == Some("i.imgur.com") {
                url.set_query(None);
            }

            response = reqwest::get(url).await?;

            content_type = response
                .headers()
                .get("Content-Type")
                .and_then(|x| x.to_str().ok());
        } else {
            return Err(anyhow::anyhow!(format!("html page has no og:image")));
        }
    }

    let format = match content_type {
        Some("image/jpeg") => ImageFormat::Jpeg,
        Some("image/png") => ImageFormat::Png,
        Some("image/webp") => ImageFormat::WebP,
        Some(s) => return Err(anyhow::anyhow!(format!("image type {} no allowed", s))),
        None => return Err(anyhow::anyhow!("No content-type header")),
    };

    console_log!(
        "{}, {:?}, {:?}, {}",
        response.status(),
        content_type,
        format,
        response.url()
    );

    let image_data = response
        .bytes()
        .await
        .context("failed to convent response body to bytes")?;

    let img =
        image::load_from_memory(&image_data).context("failed to load image from response body")?;

    Ok((img, format))
}

fn resize_to_fit(
    image: DynamicImage,
    desired_width: u32,
    desired_height: u32,
) -> anyhow::Result<DynamicImage> {
    let (width, height) = image.dimensions();

    let wratio = desired_width as f64 / width as f64;
    let hratio = desired_height as f64 / height as f64;

    let ratio = f64::max(wratio, hratio);

    let nw = max((width as f64 * ratio).round() as u64, 1);
    let nh = max((height as f64 * ratio).round() as u64, 1);

    let (new_width, new_height) = if nw > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / width as f64;
        (u32::MAX, max((height as f64 * ratio).round() as u32, 1))
    } else if nh > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / height as f64;
        (max((width as f64 * ratio).round() as u32, 1), u32::MAX)
    } else {
        (nw as u32, nh as u32)
    };

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

    let buffer = RgbaImage::from_raw(new_width, new_height, dst_image.buffer().to_vec()).unwrap();

    let resized_img = DynamicImage::ImageRgba8(buffer);

    let (iwidth, iheight) = resized_img.dimensions();

    let ratio = u64::from(iwidth) * u64::from(desired_height);
    let nratio = u64::from(desired_width) * u64::from(iheight);

    if nratio > ratio {
        Ok(resized_img.crop_imm(
            0,
            (iheight - desired_height) / 2,
            desired_width,
            desired_height,
        ))
    } else {
        Ok(resized_img.crop_imm(
            (iwidth - desired_width) / 2,
            0,
            desired_width,
            desired_height,
        ))
    }
}

async fn fetch_image_resize(
    url: &str,
    size: &ImageSize,
) -> anyhow::Result<(DynamicImage, ImageFormat)> {
    let (img, format) = fetch_image(url).await?;

    let (width, height) = match size {
        ImageSize::Preview => (32, 32),
        ImageSize::Thumbnail => (110, 155),
        ImageSize::Medium => (230, 325),
        ImageSize::Large => (450, 635),
    };

    let img = resize_to_fit(img, width, height).context("failed to resize image")?;

    Ok((img, format))
}

fn respond_with_image(image: DynamicImage, format: ImageFormat) -> anyhow::Result<Uint8Array> {
    let mut buf = std::io::Cursor::new(Vec::new());

    image
        .write_to(
            &mut buf,
            match format {
                // TODO webp encoding is not supported while targeting wasm
                // all webp images will be convented to png
                ImageFormat::WebP | ImageFormat::Png => image::ImageOutputFormat::Png,
                ImageFormat::Jpeg => image::ImageOutputFormat::Jpeg(80),
            },
        )
        .context("failed to encode resized image")?;

    Ok(Uint8Array::from(buf.get_ref().clone().as_ref()))
}

#[wasm_bindgen]
pub async fn proxy(url: &str, size: Option<String>) -> Uint8Array {
    console_error_panic_hook::set_once();

    let size = match size.as_ref().map(|x| x.as_str()) {
        Some("preview") => ImageSize::Preview,
        Some("thumbnail") => ImageSize::Thumbnail,
        Some("medium") => ImageSize::Medium,
        _ => ImageSize::Large,
    };

    match fetch_image_resize(&url, &size).await {
        Ok((img, f)) => respond_with_image(img, f).unwrap(),
        Err(_err) => {
            // console_log!("{:?}", _err);

            let default_image: &[u8] = match &size {
                ImageSize::Preview => include_bytes!("../default/preview.png"),
                ImageSize::Thumbnail => include_bytes!("../default/thumbnail.png"),
                _ => include_bytes!("../default/medium.png"),
            };

            Uint8Array::from(default_image)
        }
    }
}
