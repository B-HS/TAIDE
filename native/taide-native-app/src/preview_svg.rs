use std::sync::atomic::{AtomicUsize, Ordering};

use image::ImageDecoder;
use resvg::{tiny_skia, usvg};
use taide_model::error::AppResult;

use crate::preview::{MAX_RGBA_BYTES, Raster, invalid, raster_decoder, rgba_bytes};

const MAX_SVG_DEPTH: usize = 16;

pub(crate) fn decode(bytes: &[u8], max_side: usize) -> AppResult<Raster> {
    let source = std::str::from_utf8(bytes).map_err(|error| invalid(error.to_string()))?;
    let xml = usvg::roxmltree::Document::parse_with_options(
        source,
        usvg::roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .map_err(|error| invalid(error.to_string()))?;
    let embedded_bytes = AtomicUsize::new(0);
    let embedded_encoded = AtomicUsize::new(0);
    let embedded_depth = AtomicUsize::new(0);
    let peak_depth = AtomicUsize::new(0);
    let options = usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_string: Box::new(|_, _| None),
            resolve_data: Box::new(|mime, data, options| {
                embedded_encoded
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                        used.checked_add(data.len()).filter(|total| {
                            *total as u64 <= taide_model::file::READ_ONLY_FILE_BYTES
                        })
                    })
                    .ok()?;
                let (kind, dimensions) = match image::guess_format(&data) {
                    Ok(format) => {
                        let decoder = raster_decoder(&data, max_side).ok()?;
                        let (width, height) = decoder.dimensions();
                        drop(decoder);
                        let kind = match format {
                            image::ImageFormat::Png => usvg::ImageKind::PNG(data),
                            image::ImageFormat::Jpeg => usvg::ImageKind::JPEG(data),
                            image::ImageFormat::Gif => usvg::ImageKind::GIF(data),
                            image::ImageFormat::WebP => usvg::ImageKind::WEBP(data),
                            _ => return None,
                        };
                        (kind, [width as usize, height as usize])
                    }
                    Err(_) if matches!(mime, "image/svg+xml" | "text/plain") => {
                        let depth = embedded_depth
                            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |depth| {
                                depth.checked_add(1).filter(|next| *next <= MAX_SVG_DEPTH)
                            })
                            .ok()?
                            + 1;
                        peak_depth.fetch_max(depth, Ordering::Relaxed);
                        let result = usvg::Tree::from_data_nested(&data, options);
                        embedded_depth.fetch_sub(1, Ordering::Relaxed);
                        let tree = result.ok()?;
                        let size = tree.size().to_int_size();
                        (
                            usvg::ImageKind::SVG(tree),
                            [size.width() as usize, size.height() as usize],
                        )
                    }
                    Err(_) => return None,
                };
                let bytes = rgba_bytes(dimensions, max_side).ok()?;
                embedded_bytes
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                        used.checked_add(bytes)
                            .filter(|total| *total <= MAX_RGBA_BYTES)
                    })
                    .ok()?;
                Some(kind)
            }),
        },
        fontdb: crate::system_fonts::database(),
        ..Default::default()
    };
    let tree =
        usvg::Tree::from_xmltree(&xml, &options).map_err(|error| invalid(error.to_string()))?;
    let size = tree.size().to_int_size();
    let dimensions = [size.width() as usize, size.height() as usize];
    let canvas_bytes = rgba_bytes(dimensions, max_side)?;
    canvas_bytes
        .checked_mul(peak_depth.load(Ordering::Relaxed) + 1)
        .and_then(|bytes| bytes.checked_add(embedded_bytes.load(Ordering::Relaxed)))
        .filter(|bytes| *bytes <= MAX_RGBA_BYTES)
        .ok_or_else(|| invalid("SVG embedded images exceed the aggregate pixel budget"))?;
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| invalid("native SVG preview could not allocate its raster"))?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    Ok(Raster {
        size: dimensions,
        rgba: pixmap.take_demultiplied(),
        animation: None,
    })
}
