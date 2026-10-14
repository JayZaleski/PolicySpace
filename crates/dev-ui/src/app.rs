//! The developer UI window: panels, the map canvas, and the eframe app that ties them to a
//! [`Session`]. The drawing is checked by hand (see the Step 07 spec); the map math it uses
//! is tested in `geometry`.

use std::path::PathBuf;
use std::process::ExitCode;

use eframe::egui::{
    self, Align2, Button, CentralPanel, Color32, FontId, Label, Panel, Pos2, RichText, ScrollArea,
    Sense, Shape, Stroke, Ui, ViewportCommand,
};
use policyspace_engine::{ENGINE_NAME, ENGINE_VERSION, HexId, World};

use crate::check::count;
use crate::geometry::{
    DOT_RADIUS_PX, HexShape, Point, View, Viewport, hex_at, hex_center, hex_corners, is_visible,
};
use crate::session::{Loaded, Session};

/// How much one unit of scroll-wheel input zooms: the factor is `exp(scroll · this)`.
const SCROLL_ZOOM_PER_UNIT: f64 = 0.0015;
/// The radius of a hovered hex's dot (pixels).
const HOVER_DOT_RADIUS_PX: f32 = 3.0;
/// The largest label font size (points).
const MAX_LABEL_SIZE: f64 = 14.0;

/// Opens the window, with `folder` loaded if given, and runs until it's closed.
/// If the window can't be created, prints `error: could not open the window: {error}`
/// to stderr and returns exit code 1.
pub fn run(folder: Option<PathBuf>) -> ExitCode {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title(base_title()),
        ..Default::default()
    };
    let result = eframe::run_native(
        "PolicySpace Dev UI",
        options,
        Box::new(move |_creation_context| Ok(Box::new(DevApp::new(folder)))),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: could not open the window: {error}");
            ExitCode::from(1)
        }
    }
}

/// The window title when nothing is loaded.
fn base_title() -> String {
    format!("{ENGINE_NAME} {ENGINE_VERSION} — Dev UI")
}

/// The window's state.
struct DevApp {
    /// The current folder and what loading it gave.
    session: Session,
    /// The map's pan and zoom; None means "fit the map on the next draw".
    view: Option<View>,
    /// The hex under the pointer, if any.
    hovered: Option<HexId>,
    /// The last window title sent, so the title is only sent when it changes.
    last_title: String,
}

impl DevApp {
    /// A window state with `folder` loaded if given.
    fn new(folder: Option<PathBuf>) -> DevApp {
        let mut session = Session::new();
        if let Some(folder) = folder {
            session.open(folder);
        }
        DevApp {
            session,
            view: None,
            hovered: None,
            last_title: base_title(),
        }
    }

    /// Shows the system folder picker and, if a folder was chosen, opens it and fits the
    /// map to the window next time it's drawn. Cancelling changes nothing.
    fn open_setting(&mut self) {
        let picked = rfd::FileDialog::new()
            .set_title("Open setting folder")
            .pick_folder();
        if let Some(folder) = picked {
            self.session.open(folder);
            self.view = None;
            self.hovered = None;
        }
    }

    /// The window title for what is loaded.
    fn title(&self) -> String {
        match self.session.loaded() {
            Some(Loaded::World(world)) => {
                format!("{} — {}", base_title(), world.manifest().name)
            }
            _ => base_title(),
        }
    }

    /// The top row of controls.
    fn top_panel(&mut self, ui: &mut Ui) {
        Panel::top("controls").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open setting…").clicked() {
                    self.open_setting();
                }
                let can_reload = self.session.folder().is_some();
                if ui.add_enabled(can_reload, Button::new("Reload")).clicked() {
                    self.session.reload();
                    self.hovered = None;
                }
                let can_fit = matches!(
                    self.session.loaded(),
                    Some(Loaded::World(world)) if !world.map().is_empty()
                );
                if ui.add_enabled(can_fit, Button::new("Fit")).clicked() {
                    self.view = None;
                }
                if let Some(folder) = self.session.folder() {
                    ui.label(folder.display().to_string());
                }
            });
        });
    }

    /// The status bar.
    fn bottom_panel(&self, ui: &mut Ui) {
        Panel::bottom("status").show(ui, |ui| {
            let text = match self.session.loaded() {
                Some(Loaded::World(world)) => {
                    let mut text = format!(
                        "{} · {} · turn {}",
                        world.manifest().name,
                        count(world.map().len(), "hex", "hexes"),
                        world.turn()
                    );
                    if let Some(hex) = self.hovered {
                        text.push_str(&format!(" · hex {}", world.map().coord(hex)));
                    }
                    text
                }
                Some(Loaded::Errors(lines)) => {
                    // The last line is the summary, not an error.
                    format!("{} error(s)", lines.len().saturating_sub(1))
                }
                None => "No setting loaded".to_string(),
            };
            ui.label(text);
        });
    }

    /// The central area: a message, the error list, or the map, depending on the state.
    fn central_panel(&mut self, ui: &mut Ui) {
        let mut open_requested = false;
        CentralPanel::default().show(ui, |ui| match self.session.loaded() {
            None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 3.0);
                    ui.label("No setting loaded.");
                    if ui.button("Open setting…").clicked() {
                        open_requested = true;
                    }
                });
            }
            Some(Loaded::Errors(lines)) => {
                ui.heading("The setting could not be loaded");
                ui.add_space(8.0);
                ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    for line in lines {
                        ui.add(Label::new(RichText::new(line).monospace()).selectable(true));
                    }
                });
            }
            Some(Loaded::World(world)) if world.map().is_empty() => {
                ui.centered_and_justified(|ui| {
                    ui.label("This setting has no hexes.");
                });
            }
            Some(Loaded::World(world)) => {
                draw_map(ui, world, &mut self.view, &mut self.hovered);
            }
        });
        if open_requested {
            self.open_setting();
        }
    }
}

impl eframe::App for DevApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.top_panel(ui);
        self.bottom_panel(ui);
        self.central_panel(ui);

        let title = self.title();
        if title != self.last_title {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
    }
}

/// Converts a geometry point to egui's `f32` position. Only call this for points that
/// passed `is_visible`, which are near the canvas.
fn to_pos(p: Point) -> Pos2 {
    Pos2::new(p.x as f32, p.y as f32)
}

/// Draws the map canvas for `world` in the remaining area, and handles panning, zooming and
/// hovering. A `view` of None is fitted to the canvas first.
fn draw_map(ui: &mut Ui, world: &World, view: &mut Option<View>, hovered: &mut Option<HexId>) {
    let map = world.map();
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;
    let viewport = Viewport {
        min: Point::new(f64::from(rect.min.x), f64::from(rect.min.y)),
        width: f64::from(rect.width()),
        height: f64::from(rect.height()),
    };
    let view = view.get_or_insert_with(|| View::fit(map.iter().map(|(_, coord)| coord), viewport));

    let pointer = response
        .hover_pos()
        .map(|pos| Point::new(f64::from(pos.x), f64::from(pos.y)));
    let mut changed = false;
    if response.dragged() {
        let delta = response.drag_delta();
        view.pan(f64::from(delta.x), f64::from(delta.y));
        changed = true;
    }
    if let Some(pointer) = pointer {
        let (scroll, pinch) = ui.input(|input| (input.smooth_scroll_delta.y, input.zoom_delta()));
        if scroll != 0.0 {
            let factor = (f64::from(scroll) * SCROLL_ZOOM_PER_UNIT).exp();
            view.zoom_at(pointer, factor, viewport);
            changed = true;
        }
        if pinch != 1.0 {
            view.zoom_at(pointer, f64::from(pinch), viewport);
            changed = true;
        }
    }
    if changed {
        ui.ctx().request_repaint();
    }

    *hovered = match pointer {
        Some(pointer) if !response.dragged() => hex_at(map, view, viewport, pointer),
        _ => None,
    };

    let foreground = ui.visuals().text_color();
    let highlight = ui.visuals().selection.bg_fill;
    let stroke = Stroke::new(1.0, foreground);
    let shape = view.hex_shape();
    for (id, coord) in map.iter() {
        let center = view.world_to_screen(hex_center(coord), viewport);
        if !is_visible(view, viewport, center) {
            continue;
        }
        let is_hovered = *hovered == Some(id);
        paint_hex(
            &painter,
            shape,
            to_pos(center),
            view.scale,
            HexLook {
                foreground,
                highlight: is_hovered.then_some(highlight),
                stroke,
            },
            (coord.q, coord.r),
        );
    }
}

/// How one hex is painted.
struct HexLook {
    /// The color of dots, outlines and labels.
    foreground: Color32,
    /// The fill color, if the hex is hovered.
    highlight: Option<Color32>,
    /// The outline stroke.
    stroke: Stroke,
}

/// Paints one visible hex at screen position `center`, in the style for `shape`.
fn paint_hex(
    painter: &egui::Painter,
    shape: HexShape,
    center: Pos2,
    scale: f64,
    look: HexLook,
    (q, r): (i32, i32),
) {
    match shape {
        HexShape::Dot => {
            if let Some(highlight) = look.highlight {
                painter.circle_filled(center, HOVER_DOT_RADIUS_PX, highlight);
            } else {
                painter.circle_filled(center, DOT_RADIUS_PX as f32, look.foreground);
            }
        }
        HexShape::Hexagon | HexShape::LabelledHexagon => {
            let corners: Vec<Pos2> =
                hex_corners(Point::new(f64::from(center.x), f64::from(center.y)), scale)
                    .into_iter()
                    .map(to_pos)
                    .collect();
            match look.highlight {
                Some(highlight) => {
                    painter.add(Shape::convex_polygon(corners, highlight, look.stroke));
                }
                None => {
                    painter.add(Shape::closed_line(corners, look.stroke));
                }
            }
            if shape == HexShape::LabelledHexagon {
                let size = (scale * 0.3).min(MAX_LABEL_SIZE) as f32;
                painter.text(
                    center,
                    Align2::CENTER_CENTER,
                    format!("{q}, {r}"),
                    FontId::proportional(size),
                    look.foreground,
                );
            }
        }
    }
}
