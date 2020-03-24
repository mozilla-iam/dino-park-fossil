use data_url::DataUrl;
use failure::format_err;
use failure::Error;
use image::imageops::FilterType;
use image::DynamicImage;
use image::GenericImageView;

#[derive(Debug, Fail)]
pub enum ImageProcessingError {
    #[fail(display = "invalid data uri")]
    InvalidDataUri,
    #[fail(display = "invalid image format")]
    InvalidFormat,
    #[fail(display = "invalid base 64")]
    InvalidBase64,
}

pub struct Avatars {
    pub raw: Vec<u8>,
    pub x528: Vec<u8>,
    pub x264: Vec<u8>,
    pub x100: Vec<u8>,
    pub x40: Vec<u8>,
}

impl Avatars {
    pub fn new(buf: Vec<u8>) -> Result<Self, Error> {
        let img = image::load_from_memory(&buf)?;
        let (w, h) = img.dimensions();
        let ratio = f64::from(w) / f64::from(h);
        if ratio < 0.95 || ratio > 1.05 {
            return Err(format_err!("wrong ascpect ratio: {}", ratio));
        }
        Ok(Avatars {
            raw: buf,
            x528: downsize(528, &img)?,
            x264: downsize(264, &img)?,
            x100: downsize(100, &img)?,
            x40: downsize(40, &img)?,
        })
    }
}

pub fn png_from_data_uri(data_uri: &str) -> Result<Vec<u8>, Error> {
    let data = DataUrl::process(data_uri).map_err(|_| ImageProcessingError::InvalidDataUri)?;
    if data.mime_type().type_ != "image" || data.mime_type().subtype != "png" {
        return Err(ImageProcessingError::InvalidFormat.into());
    }
    let (buf, _) = data
        .decode_to_vec()
        .map_err(|_| ImageProcessingError::InvalidBase64)?;
    Ok(buf)
}

fn downsize(size: u32, img: &DynamicImage) -> Result<Vec<u8>, Error> {
    let down_sized = img.resize_to_fill(size, size, FilterType::Lanczos3);
    let mut buf: Vec<u8> = Vec::new();
    down_sized.write_to(&mut buf, image::ImageOutputFormat::Png)?;
    Ok(buf)
}
