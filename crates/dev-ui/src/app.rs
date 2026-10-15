//! The developer UI window: panels, the map canvas, and the eframe app that ties them to a
//! [`Session`]. The window also has the world panel (turn log, goods, resource types, POP
//! types) and the hex panel (the selected hex's deposits, stockpile, market and POPs), and
//! steps turns. The drawing is checked by hand (see the Step 07 and Step 08 specs); the map
//! math it uses is tested in `geometry`, and the panel text in `text`.

use std::fmt::Debug;
use std::hash::Hash;
use std::path::PathBuf;
use std::process::ExitCode;

use eframe::egui::{
    self, Align2, Button, CentralPanel, CollapsingHeader, Color32, FontId, Grid, Key, Label,
    Modifiers, Panel, Pos2, RichText, ScrollArea, Sense, Shape, Stroke, TextStyle, Ui,
    ViewportCommand,
};
use policyspace_engine::{ENGINE_NAME, ENGINE_VERSION, HexId, World};

use crate::geometry::{
    DOT_RADIUS_PX, HexShape, Point, View, Viewport, hex_at, hex_center, hex_corners, is_visible,
};
use crate::session::{Loaded, STEP_MANY, Session};
use crate::text::{
    StockLine, count, deposit_lines, good_rows, market_lines, pop_section, pop_type_rows,
    resource_rows, stockpile_lines,
};

/// How much one unit of scroll-wheel input zooms: the factor is `exp(scroll · this)`.
const SCROLL_ZOOM_PER_UNIT: f64 = 0.0015;
/// The radius of a hovered hex's dot (pixels).
const HOVER_DOT_RADIUS_PX: f32 = 3.0;
/// The largest label font size (points).
const MAX_LABEL_SIZE: f64 = 14.0;
/// The width side panels start with (points).
const SIDE_PANEL_WIDTH: f32 = 320.0;
/// The width the hex panel starts with (points): wide enough for a POP's seven-column goods
/// table at the sample's numbers.
const HEX_PANEL_WIDTH: f32 = 460.0;
/// The narrowest a side panel can be made (points).
const SIDE_PANEL_MIN_WIDTH: f32 = 200.0;
/// The tallest the turn log's scroll area gets (points).
const TURN_LOG_MAX_HEIGHT: f32 = 320.0;
/// The stroke width of the selected hex's outline (pixels).
const SELECTED_STROKE_PX: f32 = 3.0;
/// The radius of the selected hex's ring when hexes are dots (pixels).
const SELECTED_DOT_RADIUS_PX: f32 = 5.0;
/// The stroke width of the selected hex's ring when hexes are dots (pixels).
const SELECTED_DOT_STROKE_PX: f32 = 2.0;

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
        match self.session.world() {
            Some(world) => format!("{} — {}", base_title(), world.manifest().name),
            None => base_title(),
        }
    }

    /// Handles the keyboard shortcuts: Space steps once, Shift+Space steps `STEP_MANY`
    /// times, and Esc clears the selection. Only with a world loaded. This runs at the very
    /// start of a frame, before any widget is drawn, so a focused button doesn't also react
    /// to Space.
    fn shortcuts(&mut self, ui: &mut Ui) {
        if self.session.world().is_none() {
            return;
        }
        // Both Space keys are consumed every frame, Shift+Space first because `consume_key`
        // ignores extra Shift, so no leftover Space can reach a focused button. Shift+Space
        // then wins when both were pressed.
        let (step_many, step_one, clear) = ui.input_mut(|input| {
            let many = input.consume_key(Modifiers::SHIFT, Key::Space);
            let one = input.consume_key(Modifiers::NONE, Key::Space);
            let clear = input.consume_key(Modifiers::NONE, Key::Escape);
            (many, one, clear)
        });
        if step_many {
            self.session.step(STEP_MANY);
        } else if step_one {
            self.session.step(1);
        }
        if clear {
            self.session.select(None);
        }
    }

    /// The world panel (left) and the hex panel (right), when a world is loaded. They must
    /// be shown before the central panel.
    fn side_panels(&self, ui: &mut Ui) {
        let Some(world) = self.session.world() else {
            return;
        };
        Panel::left("world")
            .resizable(true)
            .default_size(SIDE_PANEL_WIDTH)
            .min_size(SIDE_PANEL_MIN_WIDTH)
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| self.world_panel_contents(ui, world));
            });
        Panel::right("hex")
            .resizable(true)
            .default_size(HEX_PANEL_WIDTH)
            .min_size(SIDE_PANEL_MIN_WIDTH)
            .show(ui, |ui| {
                ScrollArea::both().show(ui, |ui| self.hex_panel_contents(ui, world));
            });
    }

    /// The world panel's sections: the turn log, the goods, the resource types and the POP
    /// types.
    fn world_panel_contents(&self, ui: &mut Ui, world: &World) {
        CollapsingHeader::new("Turn log")
            .default_open(true)
            .show(ui, |ui| {
                if let Some(error) = self.session.step_error() {
                    let color = ui.visuals().error_fg_color;
                    ui.colored_label(color, error);
                }
                let log = self.session.log();
                let row_height = ui.text_style_height(&TextStyle::Body);
                ScrollArea::both()
                    .id_salt("turn log")
                    .max_height(TURN_LOG_MAX_HEIGHT)
                    .show_rows(ui, row_height, log.len(), |ui, rows| {
                        for row in rows {
                            // Newest first.
                            let line = &log[log.len() - 1 - row];
                            ui.add(Label::new(line.as_str()).extend());
                        }
                    });
            });
        CollapsingHeader::new("Goods")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("goods")
                    .num_columns(5)
                    .striped(true)
                    .show(ui, |ui| {
                        for heading in ["Name", "Lasts", "Movable", "In hexes", "In POPs"] {
                            ui.strong(heading);
                        }
                        ui.end_row();
                        for row in good_rows(world) {
                            ui.label(row.name);
                            ui.label(row.lasts);
                            ui.label(row.movable);
                            ui.label(row.in_hexes);
                            ui.label(row.in_pops);
                            ui.end_row();
                        }
                    });
            });
        CollapsingHeader::new("Resource types")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("resource types")
                    .num_columns(4)
                    .striped(true)
                    .show(ui, |ui| {
                        for heading in ["Name", "Produces", "Output per deposit", "Deposits"] {
                            ui.strong(heading);
                        }
                        ui.end_row();
                        for row in resource_rows(world) {
                            ui.label(row.name);
                            ui.label(row.produces);
                            ui.label(row.output);
                            ui.label(row.deposits.to_string());
                            ui.end_row();
                        }
                    });
            });
        CollapsingHeader::new("POP types")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("pop types")
                    .num_columns(6)
                    .striped(true)
                    .show(ui, |ui| {
                        for heading in [
                            "Name",
                            "Endowment",
                            "Utility",
                            "Growth",
                            "POPs",
                            "Total size",
                        ] {
                            ui.strong(heading);
                        }
                        ui.end_row();
                        for row in pop_type_rows(world) {
                            ui.label(row.name);
                            ui.vertical(|ui| {
                                for line in row.endowment {
                                    ui.label(line);
                                }
                            });
                            ui.vertical(|ui| {
                                for line in row.utility {
                                    ui.label(line.line).on_hover_text(line.share);
                                }
                            });
                            ui.label(row.growth);
                            ui.label(row.pops.to_string());
                            ui.label(row.total_size);
                            ui.end_row();
                        }
                    });
            });
    }

    /// The hex panel's contents: the selected hex's deposits, stockpile, market and POPs, or
    /// a hint.
    fn hex_panel_contents(&self, ui: &mut Ui, world: &World) {
        if world.map().is_empty() {
            ui.label("This setting has no hexes.");
            return;
        }
        let Some(hex) = self.session.selected() else {
            ui.label("Click a hex to inspect it.");
            return;
        };
        ui.heading(format!("Hex {}", world.map().coord(hex)));
        ui.add_space(4.0);
        ui.strong("Deposits");
        let deposits = deposit_lines(world, hex);
        if deposits.is_empty() {
            ui.label("No deposits.");
        }
        for line in deposits {
            ui.label(line);
        }
        ui.add_space(4.0);
        ui.strong("Stockpile");
        stockpile_listing(ui, "hex", stockpile_lines(world, hex));
        ui.add_space(4.0);
        ui.strong("Market");
        for line in market_lines(world, hex) {
            ui.label(line);
        }
        ui.add_space(4.0);
        ui.strong("POPs");
        let pops = world.pops().on_hex(hex);
        if pops.is_empty() {
            ui.label("No POPs.");
        }
        for &pop in pops {
            let section = pop_section(world, pop);
            // The ID is explicit: the heading holds the size, which changes every step, and a
            // collapsed section must stay collapsed.
            CollapsingHeader::new(section.heading)
                .id_salt(("pop", pop.index()))
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(section.growth).on_hover_text(section.growth_hover);
                    ui.label(section.demand);
                    Grid::new(("pop goods", pop.index()))
                        .num_columns(7)
                        .striped(true)
                        .show(ui, |ui| {
                            for heading in [
                                "Good",
                                "Held",
                                "Demanded",
                                "Subsistence",
                                "Sold",
                                "Bought",
                                "Consumed",
                            ] {
                                ui.strong(heading);
                            }
                            ui.end_row();
                            for row in section.goods {
                                let cells = [
                                    row.good,
                                    row.held,
                                    row.demanded,
                                    row.subsistence,
                                    row.sold,
                                    row.bought,
                                    row.consumed,
                                ];
                                if row.below_subsistence {
                                    let color = ui.visuals().error_fg_color;
                                    for cell in cells {
                                        ui.colored_label(color, cell);
                                    }
                                } else {
                                    for cell in cells {
                                        ui.label(cell);
                                    }
                                }
                                ui.end_row();
                            }
                        });
                    ui.label(section.carried_over);
                    ui.strong("Stockpile");
                    stockpile_listing(ui, ("pop", pop.index()), section.stockpile);
                });
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
                let can_fit = self
                    .session
                    .world()
                    .is_some_and(|world| !world.map().is_empty());
                if ui.add_enabled(can_fit, Button::new("Fit")).clicked() {
                    self.view = None;
                }
                let can_step = self.session.world().is_some();
                if ui
                    .add_enabled(can_step, Button::new("Step"))
                    .on_hover_text("Space")
                    .clicked()
                {
                    self.session.step(1);
                }
                if ui
                    .add_enabled(can_step, Button::new("Step ×10"))
                    .on_hover_text("Shift+Space")
                    .clicked()
                {
                    self.session.step(STEP_MANY);
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
        let mut clicked: Option<Option<HexId>> = None;
        let selected = self.session.selected();
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
                clicked = draw_map(ui, world, &mut self.view, &mut self.hovered, selected);
            }
        });
        if let Some(hex) = clicked {
            self.session.select(hex);
        }
        if open_requested {
            self.open_setting();
        }
    }
}

/// Lists a stockpile: "Empty." when it has nothing, else each good's line with its groups
/// indented below it. `salt` keeps the indents of different stockpiles apart.
fn stockpile_listing(ui: &mut Ui, salt: impl Hash + Debug, lines: Vec<StockLine>) {
    if lines.is_empty() {
        ui.label("Empty.");
    }
    for stock in lines {
        ui.label(stock.good);
        ui.indent(&salt, |ui| {
            for part in stock.parts {
                ui.label(part);
            }
        });
    }
}

impl eframe::App for DevApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.shortcuts(ui);
        self.top_panel(ui);
        self.bottom_panel(ui);
        self.side_panels(ui);
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
/// hovering. A `view` of None is fitted to the canvas first. Hexes with deposits get a filled
/// mark below the label and hexes with POPs a hollow one above it, and `selected` gets an
/// outline. Returns `Some(hex under the pointer, or None)` if the
/// canvas was clicked this frame (a click, not a drag), for the caller to select.
fn draw_map(
    ui: &mut Ui,
    world: &World,
    view: &mut Option<View>,
    hovered: &mut Option<HexId>,
    selected: Option<HexId>,
) -> Option<Option<HexId>> {
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
    let clicked = match pointer {
        Some(pointer) if response.clicked() => Some(hex_at(map, view, viewport, pointer)),
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
            center,
            view.scale,
            HexLook {
                foreground,
                highlight: is_hovered.then_some(highlight),
                stroke,
                has_deposits: !world.deposits().on_hex(id).is_empty(),
                has_pops: !world.pops().on_hex(id).is_empty(),
            },
            (coord.q, coord.r),
        );
    }

    // The selected hex is outlined in a second pass so neighbours don't paint over it.
    if let Some(id) = selected {
        let center = view.world_to_screen(hex_center(map.coord(id)), viewport);
        if is_visible(view, viewport, center) {
            let accent = ui.visuals().selection.stroke.color;
            match shape {
                HexShape::Dot => {
                    painter.circle_stroke(
                        to_pos(center),
                        SELECTED_DOT_RADIUS_PX,
                        Stroke::new(SELECTED_DOT_STROKE_PX, accent),
                    );
                }
                HexShape::Hexagon | HexShape::LabelledHexagon => {
                    let corners: Vec<Pos2> = hex_corners(center, view.scale)
                        .into_iter()
                        .map(to_pos)
                        .collect();
                    painter.add(Shape::closed_line(
                        corners,
                        Stroke::new(SELECTED_STROKE_PX, accent),
                    ));
                }
            }
        }
    }
    clicked
}

/// How one hex is painted.
struct HexLook {
    /// The color of dots, outlines and labels.
    foreground: Color32,
    /// The fill color, if the hex is hovered.
    highlight: Option<Color32>,
    /// The outline stroke.
    stroke: Stroke,
    /// Whether the hex has any deposits (it then gets a filled mark below the label, unless
    /// it is drawn as a dot).
    has_deposits: bool,
    /// Whether the hex has any POPs (it then gets a hollow mark above the label, unless it is
    /// drawn as a dot).
    has_pops: bool,
}

/// Paints one visible hex centered at the screen point `center`, in the style for `shape`.
fn paint_hex(
    painter: &egui::Painter,
    shape: HexShape,
    center: Point,
    scale: f64,
    look: HexLook,
    (q, r): (i32, i32),
) {
    let pos = to_pos(center);
    match shape {
        HexShape::Dot => {
            if let Some(highlight) = look.highlight {
                painter.circle_filled(pos, HOVER_DOT_RADIUS_PX, highlight);
            } else {
                painter.circle_filled(pos, DOT_RADIUS_PX as f32, look.foreground);
            }
        }
        HexShape::Hexagon | HexShape::LabelledHexagon => {
            let corners: Vec<Pos2> = hex_corners(center, scale).into_iter().map(to_pos).collect();
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
                    pos,
                    Align2::CENTER_CENTER,
                    format!("{q}, {r}"),
                    FontId::proportional(size),
                    look.foreground,
                );
            }
            // The marks sit below and above the label, so they never cover the coordinates.
            let offset = (0.55 * scale) as f32;
            // Capped at 0.35 of the hexagon scale so they stay inside the hexagon.
            let radius = ((0.12 * scale) as f32)
                .clamp(2.0, 6.0)
                .min((0.35 * scale) as f32);
            if look.has_deposits {
                painter.circle_filled(pos + egui::vec2(0.0, offset), radius, look.foreground);
            }
            if look.has_pops {
                painter.circle_stroke(
                    pos - egui::vec2(0.0, offset),
                    radius,
                    Stroke::new(1.0, look.foreground),
                );
            }
        }
    }
}
