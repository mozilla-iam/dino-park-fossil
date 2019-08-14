use actix_multipart::Field;
use actix_multipart::Multipart;
use actix_multipart::MultipartError;

use actix_web::error;
use actix_web::web::Bytes;
use failure::Error;
use futures::future;
use futures::Future;
use futures::Stream;
use image::FilterType;

#[derive(Debug, Fail)]
pub enum ConverterError {
    #[fail(display = "failed to downsize")]
    Downsize,
}

fn scale(size: u32, buf: &[u8]) -> Result<Vec<u8>, Error> {
    info!("downsizing to {}", size);
    let img = image::load_from_memory(buf)?;
    let down_sized = img.resize_to_fill(size, size, FilterType::CatmullRom);
    let mut buf: Vec<u8> = Vec::new();
    down_sized.write_to(&mut buf, image::ImageOutputFormat::PNG)?;
    info!("done downsizing to {}", size);
    Ok(buf)
}

pub fn downsize(size: u32, field: Field) -> impl Future<Item = Vec<u8>, Error = Error> {
    field
        .fold(Vec::<u8>::new(), move |mut acc: Vec<u8>, bytes: Bytes| {
            acc.extend(bytes.into_iter());
            future::result(Ok(acc).map_err(|e| {
                println!("file.write_all failed: {:?}", e);
                MultipartError::Payload(error::PayloadError::Io(e))
            }))
        })
        .map_err(|e| {
            println!("failed downsizing, {:?}", e);
            ConverterError::Downsize.into()
        })
        .and_then(move |buf: Vec<u8>| scale(size, &buf))
}
pub fn handle_multipart_item(
    size: u32,
    multipart: Multipart,
) -> impl Future<Item = Vec<u8>, Error = Error> {
    info!("incoming");
    multipart
        .map(move |field| downsize(size, field).into_stream())
        .map_err(|_| Error::from(ConverterError::Downsize))
        .flatten()
        .collect()
        .map(|mut v| v.pop().unwrap_or_default())
        .map_err(Into::into)
}
