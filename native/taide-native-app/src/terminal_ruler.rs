use eframe::egui::{Color32, Rect, Ui, pos2, vec2};
use taide_model::error::AppResult;
use taide_native_terminal::{CommandDecoration, GridDimensions, Line, Mode, Rgb, TerminalCore};

pub(crate) const WIDTH: f64 = 14.0;
const BORDER_PIXELS: f64 = 1.0;
const ZONE_COLUMNS: f64 = 3.0;
const MIN_ZONE_HEIGHT: f64 = 6.0;
const MAX_ZONE_HEIGHT: f64 = 12.0;
const CENTER_DIVISOR: f64 = 2.0;
const ROUND_HALF: f64 = 0.5;

struct Geometry {
    rect: Rect,
    physical_height: f64,
    scale_x: f64,
    scale_y: f64,
    right_x: f64,
    right_width: f64,
    zone_height: f64,
    line_padding: f64,
    lines: f64,
}

struct Zone {
    start: f64,
    end: f64,
    color: Rgb,
}

fn round(value: f64) -> f64 {
    (value + ROUND_HALF).floor()
}

impl Geometry {
    fn new(screen: Rect, scale: f32, lines: usize) -> Option<Self> {
        if !screen.is_finite()
            || screen.height() <= 0.0
            || !scale.is_finite()
            || scale <= 0.0
            || lines == 0
        {
            return None;
        }
        let scale = f64::from(scale);
        let physical_width = round(WIDTH * scale);
        let physical_height = round(f64::from(screen.height()) * scale);
        if physical_width < BORDER_PIXELS || physical_height < BORDER_PIXELS {
            return None;
        }
        let outer_width = ((physical_width - BORDER_PIXELS) / ZONE_COLUMNS).floor();
        let inner_width = ((physical_width - BORDER_PIXELS) / ZONE_COLUMNS).ceil();
        let lines = lines as f64;
        let zone_height =
            round((physical_height / lines).clamp(MIN_ZONE_HEIGHT, MAX_ZONE_HEIGHT) * scale);
        Some(Self {
            rect: Rect::from_min_max(
                pos2(screen.right() - WIDTH as f32, screen.top()),
                screen.right_bottom(),
            ),
            physical_height,
            scale_x: WIDTH / physical_width,
            scale_y: f64::from(screen.height()) / physical_height,
            right_x: BORDER_PIXELS + outer_width + inner_width,
            right_width: outer_width,
            zone_height,
            line_padding: (lines / (physical_height - BORDER_PIXELS) * zone_height).floor(),
            lines,
        })
    }

    fn border(&self) -> Rect {
        Rect::from_min_size(
            self.rect.min,
            vec2((BORDER_PIXELS * self.scale_x) as f32, self.rect.height()),
        )
    }

    fn zone(&self, zone: &Zone) -> Rect {
        let y = round(
            (self.physical_height - BORDER_PIXELS) * zone.start / self.lines
                - self.zone_height / CENTER_DIVISOR,
        );
        let height = round(
            (self.physical_height - BORDER_PIXELS) * (zone.end - zone.start) / self.lines
                + self.zone_height,
        );
        Rect::from_min_size(
            self.rect.min
                + vec2(
                    (self.right_x * self.scale_x) as f32,
                    (y * self.scale_y) as f32,
                ),
            vec2(
                (self.right_width * self.scale_x) as f32,
                (height * self.scale_y) as f32,
            ),
        )
    }

    fn zones(&self, decorations: &mut [CommandDecoration], history: usize) -> Vec<Zone> {
        decorations.sort_by_key(|decoration| decoration.line);
        let mut zones: Vec<Zone> = Vec::new();
        for decoration in decorations {
            let line = f64::from(decoration.line.0) + history as f64;
            if let Some(zone) = zones.iter_mut().find(|zone| {
                zone.color == decoration.color
                    && line >= zone.start - self.line_padding
                    && line <= zone.end + self.line_padding
            }) {
                zone.start = zone.start.min(line);
                zone.end = zone.end.max(line);
            } else {
                zones.push(Zone {
                    start: line,
                    end: line,
                    color: decoration.color,
                });
            }
        }
        zones
    }
}

pub(crate) fn paint(ui: &Ui, screen: Rect, core: &TerminalCore) -> AppResult<()> {
    if core.mode()?.contains(Mode::ALT_SCREEN) {
        return Ok(());
    }
    let grid = core.grid()?;
    let Some(geometry) = Geometry::new(screen, ui.ctx().pixels_per_point(), grid.total_lines())
    else {
        return Ok(());
    };
    let mut decorations = core.command_decorations(
        Line(-(grid.history_size() as i32))..Line(grid.screen_lines() as i32),
    )?;
    let painter = ui
        .painter()
        .with_clip_rect(geometry.rect.intersect(ui.clip_rect()));
    painter.rect_filled(geometry.border(), 0.0, Color32::WHITE);
    for zone in geometry.zones(&mut decorations, grid.history_size()) {
        painter.rect_filled(
            geometry.zone(&zone),
            0.0,
            Color32::from_rgb(zone.color.r, zone.color.g, zone.color.b),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEIGHT: f32 = 300.0;
    const SCREEN_WIDTH: f32 = 640.0;
    const LINES: usize = 1000;
    const HISTORY: usize = 980;
    const PIXEL_TOLERANCE: f32 = 0.001;
    const SUCCESS: Rgb = Rgb { r: 1, g: 2, b: 3 };
    const FAILURE: Rgb = Rgb { r: 4, g: 5, b: 6 };
    const CORE_COLUMNS: u16 = 20;
    const CORE_ROWS: u16 = 4;
    const CORE_HISTORY: usize = 64;
    const HISTORY_OUTPUT_LINES: usize = 40;

    #[test]
    fn ruler_paint는_화면밖_history_색_clip_dpr_alt와_빈_border를_그린다() {
        let mut core = TerminalCore::new(
            taide_native_terminal::Size {
                columns: CORE_COLUMNS,
                rows: CORE_ROWS,
            },
            CORE_HISTORY,
            Default::default(),
        )
        .unwrap();
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(SCREEN_WIDTH, HEIGHT));
        for scale in [1.0, 2.0, 1.25] {
            let context = eframe::egui::Context::default();
            context.set_pixels_per_point(scale);
            let render = |core: &TerminalCore| {
                let mut output = context.run_ui(
                    eframe::egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ui| paint(ui, screen, core).unwrap(),
                );
                output.textures_delta.clear();
                output
                    .shapes
                    .into_iter()
                    .filter_map(|shape| match shape.shape {
                        eframe::egui::Shape::Rect(rect)
                            if [
                                Color32::WHITE,
                                Color32::from_rgb(SUCCESS.r, SUCCESS.g, SUCCESS.b),
                            ]
                            .contains(&rect.fill) =>
                        {
                            Some((shape.clip_rect, rect.rect, rect.fill))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            core.advance(b"\x1bc").unwrap();
            let empty = render(&core);
            assert_eq!(empty.len(), 1);
            assert_eq!(empty[0].2, Color32::WHITE);
            core.configure_command_colors(taide_native_terminal::CommandColors {
                success: Some(SUCCESS),
                failure: Some(FAILURE),
            })
            .unwrap();
            core.advance(b"\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;0\x07")
                .unwrap();
            for _ in 0..HISTORY_OUTPUT_LINES {
                core.advance(b"line\r\n").unwrap();
            }
            assert!(core.command_blocks().unwrap()[0].start.0 < 0);
            let painted = render(&core);
            assert_eq!(painted.len(), 2);
            let grid = core.grid().unwrap();
            let geometry = Geometry::new(screen, scale, grid.total_lines()).unwrap();
            assert_eq!(painted[0].1, geometry.border());
            assert_eq!(
                painted[1].1,
                geometry.zone(&Zone {
                    start: 0.0,
                    end: 0.0,
                    color: SUCCESS,
                })
            );
            assert_eq!(painted[0].0, painted[1].0);
            assert!(painted[1].0.left() >= geometry.rect.left());
            assert!(painted[1].0.right() <= geometry.rect.right());
            assert!(painted[1].0.top() >= screen.top());
            assert!(painted[1].1.top() < painted[1].0.top());
            core.advance(b"\x1b[?1049h").unwrap();
            assert!(render(&core).is_empty());
            core.advance(b"\x1b[?1049l").unwrap();
            assert_eq!(render(&core), painted);
            core.advance(b"\x1b[3J").unwrap();
            assert_eq!(render(&core).len(), 1);
        }
    }

    #[test]
    fn ruler_geometry는_js_반올림_dpr_right_폭_padding_merge와_잘못된_크기를_처리한다() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(SCREEN_WIDTH, HEIGHT));
        for (scale, border_width, right_x, right_width, zone_height) in [
            (1.0, 1.0, 10.0, 4.0, 6.0),
            (2.0, 0.5, 9.5, 4.5, 6.0),
            (
                1.25,
                14.0 / 18.0,
                12.0 * 14.0 / 18.0,
                5.0 * 14.0 / 18.0,
                8.0 * 300.0 / 375.0,
            ),
        ] {
            let geometry = Geometry::new(screen, scale, LINES).unwrap();
            assert_eq!(geometry.rect.width(), WIDTH as f32);
            assert!((geometry.border().width() - border_width as f32).abs() < PIXEL_TOLERANCE);
            let zone = geometry.zone(&Zone {
                start: 100.0,
                end: 100.0,
                color: SUCCESS,
            });
            assert_eq!(zone.left(), screen.right() - WIDTH as f32 + right_x as f32);
            assert!((zone.width() - right_width as f32).abs() < PIXEL_TOLERANCE);
            assert!((zone.height() - zone_height as f32).abs() < PIXEL_TOLERANCE);
        }
        let geometry = Geometry::new(screen, 1.0, LINES).unwrap();
        let mut decorations = [
            CommandDecoration {
                id: 1,
                line: Line(-980),
                color: SUCCESS,
            },
            CommandDecoration {
                id: 2,
                line: Line(-960),
                color: SUCCESS,
            },
            CommandDecoration {
                id: 3,
                line: Line(-970),
                color: FAILURE,
            },
            CommandDecoration {
                id: 4,
                line: Line(-961),
                color: SUCCESS,
            },
            CommandDecoration {
                id: 5,
                line: Line(-958),
                color: SUCCESS,
            },
            CommandDecoration {
                id: 6,
                line: Line(-940),
                color: SUCCESS,
            },
        ];
        let zones = geometry.zones(&mut decorations, HISTORY);
        assert_eq!(geometry.line_padding, 20.0);
        assert_eq!(zones.len(), 2);
        assert_eq!(
            (zones[0].start, zones[0].end, zones[0].color),
            (0.0, 40.0, SUCCESS)
        );
        assert_eq!(
            (zones[1].start, zones[1].end, zones[1].color),
            (10.0, 10.0, FAILURE)
        );
        assert_eq!(round(-0.5), 0.0);
        assert!(Geometry::new(Rect::ZERO, 1.0, LINES).is_none());
        assert!(Geometry::new(screen, f32::NAN, LINES).is_none());
        assert!(Geometry::new(screen, 0.0, LINES).is_none());
        assert!(Geometry::new(screen, 1.0, 0).is_none());
        assert!(
            Geometry::new(
                Rect::from_min_size(pos2(0.0, 0.0), vec2(SCREEN_WIDTH, f32::INFINITY)),
                1.0,
                LINES
            )
            .is_none()
        );
    }
}
