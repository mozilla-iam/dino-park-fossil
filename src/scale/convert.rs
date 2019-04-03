use actix_web::dev;
use actix_web::error;
use actix_web::multipart;
use actix_web::Error;
use futures::future;
use futures::Future;
use futures::Stream;
use image::FilterType;

fn scale(size: u32, buf: &[u8]) -> Result<Vec<u8>, Error> {
    info!("downsizing to {}", size);
    let img = image::load_from_memory(buf).map_err(error::ErrorInternalServerError)?;
    let down_sized = img.resize_to_fill(size, size, FilterType::CatmullRom);
    let mut buf: Vec<u8> = Vec::new();
    down_sized
        .write_to(&mut buf, image::ImageOutputFormat::PNG)
        .map_err(error::ErrorInternalServerError)?;
    info!("done downsizing to {}", size);
    Ok(buf)
}

pub fn downsize(
    size: u32,
    field: multipart::Field<dev::Payload>,
) -> Box<Future<Item = Vec<u8>, Error = Error>> {
    Box::new(
        field
            .fold(Vec::<u8>::new(), move |mut acc: Vec<u8>, bytes| {
                acc.extend(bytes.into_iter());
                future::result(Ok(acc).map_err(|e| {
                    println!("file.write_all failed: {:?}", e);
                    error::MultipartError::Payload(error::PayloadError::Io(e))
                }))
            })
            .map_err(|e| {
                println!("failed downsizing, {:?}", e);
                error::ErrorInternalServerError(e)
            })
            .and_then(move |buf: Vec<u8>| scale(size, &buf)),
    )
}
pub fn handle_multipart_item(
    size: u32,
    item: multipart::MultipartItem<dev::Payload>,
) -> Box<Stream<Item = Vec<u8>, Error = Error>> {
    info!("incoming");
    match item {
        multipart::MultipartItem::Field(field) => Box::new(downsize(size, field).into_stream()),
        multipart::MultipartItem::Nested(mp) => Box::new(
            mp.map_err(error::ErrorInternalServerError)
                .map(move |p| handle_multipart_item(size, p))
                .flatten(),
        ),
    }
}
