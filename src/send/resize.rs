use byteorder::WriteBytesExt;
use failure::format_err;
use failure::Error;
use image::imageops::FilterType;
use image::DynamicImage;
use image::GenericImageView;
use log::info;

pub struct Avatars {
    pub raw: Vec<u8>,
    pub x528: Vec<u8>,
    pub x264: Vec<u8>,
    pub x100: Vec<u8>,
    pub x40: Vec<u8>,
}

impl Avatars {
    pub fn new(buf: Vec<u8>) -> Result<Self, Error> {
        match image::guess_format(&buf) {
            Ok(image::ImageFormat::Png) => (),
            _ => return Err(format_err!("invalid image supplied, only png is supported")),
        }

        let img = image::load_from_memory_with_format(&buf, image::ImageFormat::Png)?;
        let (w, h) = img.dimensions();
        let ratio = f64::from(w) / f64::from(h);
        if ratio < 0.95 || ratio > 1.05 {
            return Err(format_err!("wrong aspect ratio: {}", ratio));
        }

        // Copy the necessary data from the original image the image crate does not pick up manually
        let metadata_to_add = Avatars::maybe_extract_png_color_metadata(&buf)?;

        Ok(Avatars {
            raw: buf,
            x528: downsize(528, &img, &metadata_to_add)?,
            x264: downsize(264, &img, &metadata_to_add)?,
            x100: downsize(100, &img, &metadata_to_add)?,
            x40: downsize(40, &img, &metadata_to_add)?,
        })
    }

    /// Returns png chunks that needs to be copied to keep color information
    /// related things intact (i.e. copy chunks that the image crate does not pick up)
    fn maybe_extract_png_color_metadata(buf: &[u8]) -> Result<Vec<u8>, Error> {
        let mut metadata_to_add = Vec::new();

        let mut png_decoder = lodepng::Decoder::new();
        // required so the chunks are remembered (by default, it only remembers the mandatory chunks)
        png_decoder.remember_unknown_chunks(true);
        png_decoder.decode(&buf)?;

        let mut added_chunks = Vec::new();

        for chunk_to_copy in &["cHRM", "gAMA", "sRGB", "iCCP", "eXIf"] {
            if let Some(data_chunk) = png_decoder.info_png().get(chunk_to_copy) {
                // chunk_length = padding (2 bytes) chunk_len (4 bytes) + chunk type (4 bytes) + data_len + crc (8 bytes) = data_len + 18
                metadata_to_add.reserve(data_chunk.len() + 18);

                // write two bytes of padding
                metadata_to_add.extend(&[0x00, 0x00]);
                metadata_to_add
                    .write_u16::<byteorder::BE>(data_chunk.len() as u16)
                    .unwrap();
                metadata_to_add.extend(&data_chunk.name());
                metadata_to_add.extend(data_chunk.data());

                metadata_to_add
                    .write_u32::<byteorder::BE>(data_chunk.crc())
                    .unwrap();

                added_chunks.push(chunk_to_copy);
            }
        }

        info!(
            "Will manually transfer the following png metadata chunks into the downsized images: {:?}",
            added_chunks
        );

        Ok(metadata_to_add)
    }
}

fn downsize(size: u32, img: &DynamicImage, metadata_to_add: &[u8]) -> Result<Vec<u8>, Error> {
    let down_sized = img.resize_to_fill(size, size, FilterType::Lanczos3);
    let mut buf: Vec<u8> = Vec::new();
    down_sized.write_to(&mut buf, image::ImageOutputFormat::Png)?;

    if !metadata_to_add.is_empty() {
        const IMAGE_START_OFFSET_TO_ADD_DATA: usize = 33;

        // temporarily cut off the image bytes after the (fixed) header
        // as most of the metadata we need to add must appear before any other tag
        let image_data = &buf[IMAGE_START_OFFSET_TO_ADD_DATA..]
            .iter()
            .copied()
            .collect::<Vec<u8>>();

        buf.truncate(IMAGE_START_OFFSET_TO_ADD_DATA);

        buf.extend(metadata_to_add);
        buf.extend(image_data);
    }

    Ok(buf)
}
