use std::io::Cursor;
use std::time::Duration;

use image::codecs::{gif::GifDecoder, png::PngDecoder, webp::WebPDecoder};
use image::metadata::{LoopCount, Orientation};
use image::{AnimationDecoder, DynamicImage, Frames, ImageDecoder, ImageFormat};
use taide_model::error::AppResult;

use crate::preview::{MAX_RGBA_BYTES, Raster, invalid, raster_limits, rgba_bytes};

pub(crate) const MAX_FRAMES: usize = 4096;
const SHORT_DELAY: Duration = Duration::from_millis(11);
const DEFAULT_DELAY: Duration = Duration::from_millis(100);
const NANOS_PER_SECOND: u128 = 1_000_000_000;

#[derive(Debug)]
pub struct Frame {
    pub rgba: Vec<u8>,
    pub delay: Duration,
}

#[derive(Debug)]
pub struct Animation {
    pub first_delay: Duration,
    pub frames: Vec<Frame>,
    pub plays: Option<u32>,
}

fn plays(loop_count: LoopCount) -> Option<u32> {
    match loop_count {
        LoopCount::Infinite => None,
        LoopCount::Finite(count) => Some(count.get()),
    }
}

fn gif_plays(bytes: &[u8]) -> AppResult<Option<u32>> {
    let mut options = gif::DecodeOptions::new();
    options.skip_frame_decoding(true);
    let mut decoder = options
        .read_info(Cursor::new(bytes))
        .map_err(|error| invalid(error.to_string()))?;
    let mut count = 0;
    while decoder
        .next_frame_info()
        .map_err(|error| invalid(error.to_string()))?
        .is_some()
    {
        count += 1;
        if count > MAX_FRAMES {
            return Err(invalid("animation exceeds the frame budget"));
        }
    }
    Ok(match decoder.repeat() {
        gif::Repeat::Infinite => None,
        gif::Repeat::Finite(repeats) => Some(u32::from(repeats) + 1),
    })
}

fn collect(
    frames: Frames<'_>,
    orientation: Orientation,
    max_side: usize,
    plays: Option<u32>,
) -> AppResult<Raster> {
    let mut decoded = Vec::new();
    let mut size = None;
    let mut used = 0usize;
    for frame in frames {
        if decoded.len() >= MAX_FRAMES {
            return Err(invalid("animation exceeds the frame budget"));
        }
        let frame = frame.map_err(|error| invalid(error.to_string()))?;
        let mut delay = Duration::from(frame.delay());
        if delay < SHORT_DELAY {
            delay = DEFAULT_DELAY;
        }
        let mut image = DynamicImage::ImageRgba8(frame.into_buffer());
        image.apply_orientation(orientation);
        let dimensions = [image.width() as usize, image.height() as usize];
        let bytes = rgba_bytes(dimensions, max_side)?;
        if size.is_some_and(|size| size != dimensions) {
            return Err(invalid("animation frame dimensions changed"));
        }
        size = Some(dimensions);
        used = used
            .checked_add(bytes)
            .filter(|bytes| *bytes <= MAX_RGBA_BYTES)
            .ok_or_else(|| invalid("animation exceeds the decoded pixel budget"))?;
        decoded.push(Frame {
            rgba: image.into_rgba8().into_raw(),
            delay,
        });
    }
    let size = size.ok_or_else(|| invalid("animation contains no frames"))?;
    let mut frames = decoded.into_iter();
    let first = frames
        .next()
        .ok_or_else(|| invalid("animation contains no first frame"))?;
    let remaining: Vec<_> = frames.collect();
    let animation = if remaining.is_empty() {
        None
    } else {
        Some(Animation {
            first_delay: first.delay,
            frames: remaining,
            plays,
        })
    };
    Ok(Raster {
        size,
        rgba: first.rgba,
        animation,
    })
}

fn prepare(decoder: &mut impl ImageDecoder, max_side: usize) -> AppResult<Orientation> {
    let (width, height) = decoder.dimensions();
    rgba_bytes([width as usize, height as usize], max_side)?;
    decoder
        .set_limits(raster_limits(max_side))
        .map_err(|error| invalid(error.to_string()))?;
    decoder
        .orientation()
        .map_err(|error| invalid(error.to_string()))
}

pub(crate) fn decode(
    bytes: &[u8],
    format: ImageFormat,
    max_side: usize,
) -> AppResult<Option<Raster>> {
    match format {
        ImageFormat::Gif => {
            let count = gif_plays(bytes)?;
            let mut decoder =
                GifDecoder::new(Cursor::new(bytes)).map_err(|error| invalid(error.to_string()))?;
            let orientation = prepare(&mut decoder, max_side)?;
            collect(decoder.into_frames(), orientation, max_side, count).map(Some)
        }
        ImageFormat::Png => {
            let mut decoder = PngDecoder::with_limits(Cursor::new(bytes), raster_limits(max_side))
                .map_err(|error| invalid(error.to_string()))?;
            if !decoder
                .is_apng()
                .map_err(|error| invalid(error.to_string()))?
            {
                return Ok(None);
            }
            let orientation = prepare(&mut decoder, max_side)?;
            let decoder = decoder.apng().map_err(|error| invalid(error.to_string()))?;
            let count = plays(decoder.loop_count());
            collect(decoder.into_frames(), orientation, max_side, count).map(Some)
        }
        ImageFormat::WebP => {
            let mut decoder =
                WebPDecoder::new(Cursor::new(bytes)).map_err(|error| invalid(error.to_string()))?;
            if !decoder.has_animation() {
                return Ok(None);
            }
            let orientation = prepare(&mut decoder, max_side)?;
            let count = plays(decoder.loop_count());
            collect(decoder.into_frames(), orientation, max_side, count).map(Some)
        }
        _ => Ok(None),
    }
}

pub(crate) struct Playback {
    pub frames: Vec<Frame>,
    ends: Vec<Duration>,
    plays: Option<u32>,
    started: Option<Duration>,
    pub current: usize,
}

impl Playback {
    pub fn new(first: Vec<u8>, animation: Animation) -> AppResult<Self> {
        if animation.plays == Some(0) || animation.frames.len() >= MAX_FRAMES {
            return Err(invalid("invalid animation playback metadata"));
        }
        let mut frames = vec![Frame {
            rgba: first,
            delay: animation.first_delay,
        }];
        frames.extend(animation.frames);
        let mut total = Duration::ZERO;
        let mut ends = Vec::with_capacity(frames.len());
        for frame in &frames {
            if frame.delay.is_zero() {
                return Err(invalid("animation has a zero playback delay"));
            }
            total = total
                .checked_add(frame.delay)
                .ok_or_else(|| invalid("animation duration overflow"))?;
            ends.push(total);
        }
        Ok(Self {
            frames,
            ends,
            plays: animation.plays,
            started: None,
            current: 0,
        })
    }

    pub fn step(&mut self, now: Duration) -> (usize, Option<Duration>) {
        let started = *self.started.get_or_insert(now);
        let elapsed = now.saturating_sub(started).as_nanos();
        let total = self.ends.last().unwrap().as_nanos();
        if self
            .plays
            .is_some_and(|plays| elapsed / total >= u128::from(plays))
        {
            return (self.frames.len() - 1, None);
        }
        let cycle = elapsed % total;
        let index = self.ends.partition_point(|end| end.as_nanos() <= cycle);
        let delay = self.ends[index].as_nanos() - cycle;
        let remaining = Duration::new(
            (delay / NANOS_PER_SECOND) as u64,
            (delay % NANOS_PER_SECOND) as u32,
        );
        (index, Some(remaining))
    }
}
