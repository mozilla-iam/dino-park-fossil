use crate::storage::saver::Saver;
use data_url::DataUrl;
use failure::Error;
use image::DynamicImage;
use image::FilterType;
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
    x264: Vec<u8>,
    x100: Vec<u8>,
    x40: Vec<u8>,
}

impl Avatars {
    pub fn new(buf: &[u8]) -> Result<Self, Error> {
        let img = image::load_from_memory(buf)?;
        let (w, h) = img.dimensions();
        let ratio = f64::from(w) / f64::from(h);
        if ratio < 0.95 || ratio > 1.05 {
            return Err(format_err!("wrong ascpect ratio: {}", ratio));
        }
        Ok(Avatars {
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

pub fn delete(name: &str, bucket: &str, saver: &impl Saver) -> Result<(), Error> {
    saver.delete(name, "264", bucket)?;
    saver.delete(name, "100", bucket)?;
    saver.delete(name, "40", bucket)
}

pub fn save(avatars: Avatars, name: &str, bucket: &str, saver: &impl Saver) -> Result<(), Error> {
    let Avatars { x264, x100, x40 } = avatars;
    saver.save(name, "264", bucket, x264)?;
    saver.save(name, "100", bucket, x100)?;
    saver.save(name, "40", bucket, x40)
}

fn downsize(size: u32, img: &DynamicImage) -> Result<Vec<u8>, Error> {
    let down_sized = img.resize_to_fill(size, size, FilterType::CatmullRom);
    let mut buf: Vec<u8> = Vec::new();
    down_sized.write_to(&mut buf, image::ImageOutputFormat::PNG)?;
    Ok(buf)
}
