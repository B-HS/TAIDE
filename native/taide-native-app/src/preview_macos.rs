use objc2_core_foundation::{
    CFBoolean, CFData, CFDictionary, CFNumber, CFString, CFType, CGPoint, CGRect, CGSize,
};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGDataProvider, CGImage, CGImageAlphaInfo,
    CGImageByteOrderInfo, kCGColorSpaceSRGB,
};
use objc2_image_io::{
    CGImageSource, CGImageSourceStatus, kCGImagePropertyPixelHeight, kCGImagePropertyPixelWidth,
    kCGImageSourceCreateThumbnailFromImageAlways, kCGImageSourceCreateThumbnailWithTransform,
    kCGImageSourceShouldAllowFloat, kCGImageSourceShouldCache, kCGImageSourceThumbnailMaxPixelSize,
};
use taide_model::error::AppResult;

use crate::preview::{Raster, invalid, rgba_bytes};

const CHANNELS: usize = 4;
const ALPHA_CHANNEL: usize = 3;
const BITS_PER_COMPONENT: usize = 8;
const MAX_COMPONENT: u16 = u8::MAX as u16;

fn dimension(properties: &CFDictionary, key: &CFString) -> AppResult<usize> {
    let properties = unsafe { properties.cast_unchecked::<CFString, CFType>() };
    properties
        .get(key)
        .and_then(|value| value.downcast_ref::<CFNumber>().and_then(CFNumber::as_i64))
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| invalid("ImageIO returned an invalid image dimension"))
}

fn rasterize(image: &CGImage, max_side: usize) -> AppResult<Raster> {
    let size = [CGImage::width(Some(image)), CGImage::height(Some(image))];
    rgba_bytes(size, max_side)?;
    let provider = CGImage::data_provider(Some(image))
        .ok_or_else(|| invalid("ImageIO could not materialize decoded pixels"))?;
    let decoded = CGDataProvider::data(Some(&provider))
        .ok_or_else(|| invalid("ImageIO could not materialize decoded pixels"))?;
    let minimum = CGImage::bytes_per_row(Some(image))
        .checked_mul(size[1])
        .filter(|bytes| *bytes > 0)
        .ok_or_else(|| invalid("ImageIO returned an invalid decoded pixel stride"))?;
    if decoded.len() < minimum {
        return Err(invalid("ImageIO returned incomplete decoded pixels"));
    }
    drop(decoded);
    render(size, max_side, |context| {
        CGContext::draw_image(
            Some(context),
            CGRect {
                origin: CGPoint { x: 0.0, y: 0.0 },
                size: CGSize {
                    width: size[0] as f64,
                    height: size[1] as f64,
                },
            },
            Some(image),
        );
    })
}

pub(crate) fn render(
    size: [usize; 2],
    max_side: usize,
    draw: impl FnOnce(&CGContext),
) -> AppResult<Raster> {
    let count = rgba_bytes(size, max_side)?;
    let color_space = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }))
        .ok_or_else(|| invalid("ImageIO could not create its sRGB output color space"))?;
    let mut rgba = vec![0; count];
    let context = unsafe {
        CGBitmapContextCreate(
            rgba.as_mut_ptr().cast(),
            size[0],
            size[1],
            BITS_PER_COMPONENT,
            size[0] * CHANNELS,
            Some(&color_space),
            CGImageByteOrderInfo::Order32Big.0 | CGImageAlphaInfo::PremultipliedLast.0,
        )
    }
    .ok_or_else(|| invalid("ImageIO could not allocate its output bitmap context"))?;
    draw(&context);
    drop(context);
    for pixel in rgba.as_chunks_mut::<CHANNELS>().0 {
        let alpha = u16::from(pixel[ALPHA_CHANNEL]);
        if alpha == 0 {
            pixel.fill(0);
            continue;
        }
        for channel in &mut pixel[..ALPHA_CHANNEL] {
            *channel = ((u16::from(*channel) * MAX_COMPONENT + alpha / 2) / alpha)
                .min(MAX_COMPONENT) as u8;
        }
    }
    Ok(Raster {
        size,
        rgba,
        animation: None,
    })
}

pub(crate) fn decode(bytes: &[u8], max_side: usize) -> AppResult<Raster> {
    let data = CFData::from_bytes(bytes);
    let disabled = CFBoolean::new(false);
    let source_options = CFDictionary::<CFString, CFType>::from_slices(
        &[unsafe { kCGImageSourceShouldCache }],
        &[disabled.as_ref()],
    );
    let source = unsafe { CGImageSource::with_data(&data, Some(source_options.as_opaque())) }
        .ok_or_else(|| invalid("ImageIO could not read this image format"))?;
    if unsafe { source.status() } != CGImageSourceStatus::StatusComplete {
        return Err(invalid("ImageIO rejected an incomplete image"));
    }
    if unsafe { source.count() } != 1 {
        return Err(invalid(
            "ImageIO animated or multi-image preview is not connected yet",
        ));
    }
    let index = unsafe { source.primary_image_index() };
    let properties = unsafe { source.properties_at_index(index, Some(source_options.as_opaque())) }
        .ok_or_else(|| invalid("ImageIO could not read image dimensions"))?;
    let size = [
        dimension(&properties, unsafe { kCGImagePropertyPixelWidth })?,
        dimension(&properties, unsafe { kCGImagePropertyPixelHeight })?,
    ];
    rgba_bytes(size, max_side)?;
    let max_size = CFNumber::new_isize(
        isize::try_from(size[0].max(size[1]))
            .map_err(|_| invalid("ImageIO image dimension overflow"))?,
    );
    let enabled = CFBoolean::new(true);
    let options = CFDictionary::<CFString, CFType>::from_slices(
        &[
            unsafe { kCGImageSourceCreateThumbnailFromImageAlways },
            unsafe { kCGImageSourceCreateThumbnailWithTransform },
            unsafe { kCGImageSourceThumbnailMaxPixelSize },
            unsafe { kCGImageSourceShouldCache },
            unsafe { kCGImageSourceShouldAllowFloat },
        ],
        &[
            enabled.as_ref(),
            enabled.as_ref(),
            max_size.as_ref(),
            disabled.as_ref(),
            disabled.as_ref(),
        ],
    );
    let image = unsafe { source.thumbnail_at_index(index, Some(options.as_opaque())) }
        .ok_or_else(|| invalid("ImageIO could not decode the image"))?;
    rasterize(&image, max_side)
}
