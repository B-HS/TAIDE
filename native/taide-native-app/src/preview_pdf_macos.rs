use std::ptr::NonNull;

use objc2_core_foundation::{CFData, CGAffineTransform, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGContext, CGDataProvider, CGPDFBox, CGPDFDictionary, CGPDFDocument, CGPDFPage,
    CGRectIntersection, CGRectStandardize,
};
use taide_model::error::AppResult;

use crate::preview::{invalid, rgba_bytes};
use crate::preview_pdf::{Page, ZOOM_DIVISOR};

const QUARTER_TURN: i32 = 90;
const FULL_TURN: i32 = 360;
const DEFAULT_PAGE_WIDTH: f64 = 612.0;
const DEFAULT_PAGE_HEIGHT: f64 = 792.0;

fn valid_box(rect: CGRect) -> bool {
    [
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    ]
    .iter()
    .all(|value| value.is_finite())
        && rect.size.width > 0.0
        && rect.size.height > 0.0
}

pub(crate) fn decode(bytes: &[u8], page: usize, zoom: u8, max_side: usize) -> AppResult<Page> {
    let data = CFData::from_bytes(bytes);
    let provider = CGDataProvider::with_cf_data(Some(&data))
        .ok_or_else(|| invalid("PDF data provider could not be created"))?;
    let document = CGPDFDocument::with_provider(Some(&provider))
        .ok_or_else(|| invalid("native PDF decoder rejected the document"))?;
    if !CGPDFDocument::is_unlocked(Some(&document)) {
        return Err(invalid("native PDF decoder could not unlock the document"));
    }
    let total_pages = CGPDFDocument::number_of_pages(Some(&document));
    if total_pages == 0 || page > total_pages {
        return Err(invalid("PDF page is outside the document"));
    }
    let page = CGPDFDocument::page(Some(&document), page)
        .ok_or_else(|| invalid("native PDF decoder could not read the page"))?;
    let media = CGRectStandardize(CGPDFPage::box_rect(Some(&page), CGPDFBox::MediaBox));
    let media = if valid_box(media) {
        media
    } else {
        CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize {
                width: DEFAULT_PAGE_WIDTH,
                height: DEFAULT_PAGE_HEIGHT,
            },
        }
    };
    let crop = CGRectStandardize(CGPDFPage::box_rect(Some(&page), CGPDFBox::CropBox));
    let intersection = if valid_box(crop) {
        CGRectIntersection(media, crop)
    } else {
        media
    };
    let rect = if valid_box(intersection) {
        intersection
    } else {
        media
    };
    let mut user_unit = 1.0;
    let key = NonNull::new(c"UserUnit".as_ptr().cast_mut()).expect("static PDF key is not null");
    let has_unit =
        unsafe { CGPDFDictionary::number(CGPDFPage::dictionary(Some(&page)), key, &mut user_unit) };
    if !has_unit || !user_unit.is_finite() || user_unit <= 0.0 {
        user_unit = 1.0;
    }
    let scale = f64::from(zoom) / f64::from(ZOOM_DIVISOR) * user_unit;
    let angle = CGPDFPage::rotation_angle(Some(&page));
    let rotation = if angle % QUARTER_TURN == 0 {
        angle.rem_euclid(FULL_TURN)
    } else {
        0
    };
    let [a, b, c, d] = match rotation {
        90 => [0.0, 1.0, 1.0, 0.0],
        180 => [-1.0, 0.0, 0.0, 1.0],
        270 => [0.0, -1.0, -1.0, 0.0],
        _ => [1.0, 0.0, 0.0, -1.0],
    };
    let logical = if a == 0.0 {
        [rect.size.height * scale, rect.size.width * scale]
    } else {
        [rect.size.width * scale, rect.size.height * scale]
    };
    if [rect.origin.x, rect.origin.y, scale]
        .iter()
        .any(|value| !value.is_finite())
        || logical
            .iter()
            .any(|value| !value.is_finite() || *value < 1.0 || *value > max_side as f64)
    {
        return Err(invalid("PDF page dimensions exceed the renderer limit"));
    }
    let size = [logical[0].floor() as usize, logical[1].floor() as usize];
    rgba_bytes(size, max_side)?;
    let center = CGPoint {
        x: rect.origin.x + rect.size.width / 2.0,
        y: rect.origin.y + rect.size.height / 2.0,
    };
    let transform = CGAffineTransform {
        a: a * scale,
        b: -b * scale,
        c: c * scale,
        d: -d * scale,
        tx: logical[0] / 2.0 - a * scale * center.x - c * scale * center.y,
        ty: size[1] as f64 - logical[1] / 2.0 + b * scale * center.x + d * scale * center.y,
    };
    if [
        transform.a,
        transform.b,
        transform.c,
        transform.d,
        transform.tx,
        transform.ty,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(invalid("PDF page transform is not finite"));
    }
    let raster = crate::preview_macos::render(size, max_side, |context| {
        CGContext::set_rgb_fill_color(Some(context), 1.0, 1.0, 1.0, 1.0);
        CGContext::fill_rect(
            Some(context),
            CGRect {
                origin: CGPoint { x: 0.0, y: 0.0 },
                size: CGSize {
                    width: size[0] as f64,
                    height: size[1] as f64,
                },
            },
        );
        CGContext::concat_ctm(Some(context), transform);
        CGContext::draw_pdf_page(Some(context), Some(&page));
    })?;
    Ok(Page::new(total_pages, raster))
}
