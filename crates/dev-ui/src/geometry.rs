//! Map math for the window: hex to world to screen, fitting, zooming, and finding the hex
//! under a point.
//!
//! Everything here is in `f64` world units, where one unit is a hex's circumradius (so a
//! view's scale is a hex's on-screen radius in pixels). The layout is the pointy-top one
//! fixed in Step 04: each value of `r` is an east-west row, `q` increases eastward, and `r`
//! increases southward (down the screen). There are no egui types, so all of it is tested
//! without a window.

use policyspace_engine::{HexCoord, HexId, HexMap};

/// √3.
const SQRT_3: f64 = 1.732_050_807_568_877_2;

/// Below this on-screen hex radius (pixels), hexes are drawn as dots.
pub const DOT_BELOW_PX: f64 = 3.0;
/// The radius of a dot (pixels).
pub const DOT_RADIUS_PX: f64 = 1.5;
/// From this on-screen hex radius (pixels), hexes show their coordinates.
pub const LABEL_FROM_PX: f64 = 22.0;
/// When hexes are dots, hovering within this distance (pixels) of a dot's center counts.
pub const HOVER_SNAP_PX: f64 = 6.0;
/// Fitting never zooms in beyond this scale (so a one-hex map isn't enormous).
pub const FIT_MAX_SCALE: f64 = 48.0;
/// The fitted map fills this fraction of the canvas (a margin around it).
pub const FIT_FILL: f64 = 0.9;
/// The smallest scale a view can have (pixels per world unit).
pub const MIN_SCALE: f64 = 1e-12;
/// The largest scale a view can have (pixels per world unit).
pub const MAX_SCALE: f64 = 400.0;
/// The scale of a fitted empty map, and the default.
pub const DEFAULT_SCALE: f64 = 32.0;

/// A point (world units or screen pixels, depending on use).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Horizontal position, increasing eastward (rightward on screen).
    pub x: f64,
    /// Vertical position, increasing southward (downward on screen).
    pub y: f64,
}

impl Point {
    /// The point (x, y).
    pub const fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
}

/// The canvas on screen, in pixels: top-left corner and size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// The top-left corner.
    pub min: Point,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

impl Viewport {
    /// The center of the canvas.
    pub fn center(&self) -> Point {
        Point::new(
            self.min.x + self.width / 2.0,
            self.min.y + self.height / 2.0,
        )
    }

    /// True if `p` is inside the canvas expanded by `margin` on every side.
    pub fn contains_with_margin(&self, p: Point, margin: f64) -> bool {
        p.x >= self.min.x - margin
            && p.x <= self.min.x + self.width + margin
            && p.y >= self.min.y - margin
            && p.y <= self.min.y + self.height + margin
    }
}

/// What part of the world is on screen: `center` (a world point) is drawn at the
/// viewport's center, and `scale` is screen pixels per world unit (= hex radius in pixels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// The world point drawn at the center of the viewport.
    pub center: Point,
    /// Screen pixels per world unit.
    pub scale: f64,
}

/// How hexes are drawn at the current scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HexShape {
    /// Too small for a hexagon: a dot of `DOT_RADIUS_PX`.
    Dot,
    /// An outlined hexagon, without a label.
    Hexagon,
    /// An outlined hexagon with its coordinates inside.
    LabelledHexagon,
}

/// The world position of a hex's center: x = √3 · (q + r/2), y = 1.5 · r.
pub fn hex_center(coord: HexCoord) -> Point {
    let q = f64::from(coord.q);
    let r = f64::from(coord.r);
    Point::new(SQRT_3 * (q + r / 2.0), 1.5 * r)
}

/// The hex whose area contains world point `p`, or None if its coordinates don't fit in i32.
pub fn hex_containing(p: Point) -> Option<HexCoord> {
    let qf = (SQRT_3 / 3.0) * p.x - p.y / 3.0;
    let rf = (2.0 / 3.0) * p.y;
    let sf = -qf - rf;
    let mut q = qf.round();
    let mut r = rf.round();
    let s = sf.round();
    let dq = (q - qf).abs();
    let dr = (r - rf).abs();
    let ds = (s - sf).abs();
    if dq > dr && dq > ds {
        q = -r - s;
    } else if dr > ds {
        r = -q - s;
    }
    let range = f64::from(i32::MIN)..=f64::from(i32::MAX);
    if !range.contains(&q) || !range.contains(&r) {
        return None;
    }
    // Both values are whole numbers inside the i32 range, so these casts are exact.
    Some(HexCoord::new(q as i32, r as i32))
}

/// The six corners of a pointy-top hex of `radius` around `center`, clockwise from the
/// top-right one: corner i is at angle (60·i − 30)° from +x, with y pointing down.
pub fn hex_corners(center: Point, radius: f64) -> [Point; 6] {
    std::array::from_fn(|i| {
        let angle = (60.0 * i as f64 - 30.0).to_radians();
        Point::new(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        )
    })
}

impl View {
    /// Fits every hex in `coords` inside `viewport`. Empty → center (0, 0), DEFAULT_SCALE.
    pub fn fit(coords: impl IntoIterator<Item = HexCoord>, viewport: Viewport) -> View {
        let mut bounds: Option<(f64, f64, f64, f64)> = None;
        for coord in coords {
            let c = hex_center(coord);
            let (min_x, max_x) = (c.x - SQRT_3 / 2.0, c.x + SQRT_3 / 2.0);
            let (min_y, max_y) = (c.y - 1.0, c.y + 1.0);
            bounds = Some(match bounds {
                None => (min_x, max_x, min_y, max_y),
                Some((a, b, c, d)) => (a.min(min_x), b.max(max_x), c.min(min_y), d.max(max_y)),
            });
        }
        let Some((min_x, max_x, min_y, max_y)) = bounds else {
            return View {
                center: Point::new(0.0, 0.0),
                scale: DEFAULT_SCALE,
            };
        };
        let box_w = max_x - min_x;
        let box_h = max_y - min_y;
        let view_w = viewport.width.max(1.0);
        let view_h = viewport.height.max(1.0);
        let scale =
            ((view_w / box_w).min(view_h / box_h) * FIT_FILL).clamp(MIN_SCALE, FIT_MAX_SCALE);
        View {
            center: Point::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0),
            scale,
        }
    }

    /// The screen position of world point `p`.
    pub fn world_to_screen(&self, p: Point, viewport: Viewport) -> Point {
        let vc = viewport.center();
        Point::new(
            vc.x + (p.x - self.center.x) * self.scale,
            vc.y + (p.y - self.center.y) * self.scale,
        )
    }

    /// The world point at screen position `p` (the inverse of `world_to_screen`).
    pub fn screen_to_world(&self, p: Point, viewport: Viewport) -> Point {
        let vc = viewport.center();
        Point::new(
            self.center.x + (p.x - vc.x) / self.scale,
            self.center.y + (p.y - vc.y) / self.scale,
        )
    }

    /// Moves the map by `dx, dy` screen pixels (dragging right moves the map right).
    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.center.x -= dx / self.scale;
        self.center.y -= dy / self.scale;
    }

    /// Multiplies the scale by `factor` (clamped to MIN_SCALE..=MAX_SCALE), keeping the
    /// world point under `screen` fixed on screen.
    pub fn zoom_at(&mut self, screen: Point, factor: f64, viewport: Viewport) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let world = self.screen_to_world(screen, viewport);
        self.scale = (self.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
        let vc = viewport.center();
        self.center = Point::new(
            world.x - (screen.x - vc.x) / self.scale,
            world.y - (screen.y - vc.y) / self.scale,
        );
    }

    /// How hexes are drawn at this scale.
    pub fn hex_shape(&self) -> HexShape {
        if self.scale < DOT_BELOW_PX {
            HexShape::Dot
        } else if self.scale >= LABEL_FROM_PX {
            HexShape::LabelledHexagon
        } else {
            HexShape::Hexagon
        }
    }
}

/// The hex under screen point `screen`, if any. First the hex that contains the point; then,
/// only when hexes are dots, the dot whose center is nearest within `HOVER_SNAP_PX` (ties
/// go to the lower `HexId`); otherwise none.
pub fn hex_at(map: &HexMap, view: &View, viewport: Viewport, screen: Point) -> Option<HexId> {
    let world = view.screen_to_world(screen, viewport);
    if let Some(id) = hex_containing(world).and_then(|coord| map.id(coord)) {
        return Some(id);
    }
    if view.hex_shape() != HexShape::Dot {
        return None;
    }
    let mut best: Option<(HexId, f64)> = None;
    for (id, coord) in map.iter() {
        let center = view.world_to_screen(hex_center(coord), viewport);
        let distance = (center.x - screen.x).hypot(center.y - screen.y);
        if distance <= HOVER_SNAP_PX && best.is_none_or(|(_, nearest)| distance < nearest) {
            best = Some((id, distance));
        }
    }
    best.map(|(id, _)| id)
}

/// True if a hex centered at screen point `center` can be seen in `viewport` at this scale.
pub fn is_visible(view: &View, viewport: Viewport, center: Point) -> bool {
    let radius = match view.hex_shape() {
        HexShape::Dot => DOT_RADIUS_PX,
        HexShape::Hexagon | HexShape::LabelledHexagon => view.scale,
    };
    viewport.contains_with_margin(center, radius)
}

#[cfg(test)]
mod tests {
    use policyspace_engine::World;

    use super::*;
    use crate::test_support::world_at;

    /// A canvas that doesn't start at the screen's origin.
    const VIEWPORT: Viewport = Viewport {
        min: Point::new(10.0, 20.0),
        width: 800.0,
        height: 600.0,
    };

    /// Every hex coordinate of the world's map.
    fn coords(world: &World) -> Vec<HexCoord> {
        world.map().iter().map(|(_, coord)| coord).collect()
    }

    /// True if the two numbers are within `1e-9` (relative to their size, at least 1).
    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    #[test]
    fn hex_center_follows_the_formula() {
        let origin = hex_center(HexCoord::new(0, 0));
        assert_eq!((origin.x, origin.y), (0.0, 0.0));
        let east = hex_center(HexCoord::new(1, 0));
        assert!(close(east.x, SQRT_3) && close(east.y, 0.0));
        let south = hex_center(HexCoord::new(0, 1));
        assert!(close(south.x, SQRT_3 / 2.0) && close(south.y, 1.5));
        let far = hex_center(HexCoord::new(i32::MAX, i32::MIN));
        assert!(far.x.is_finite() && far.y.is_finite());
        assert!(close(
            far.x,
            SQRT_3 * (f64::from(i32::MAX) + f64::from(i32::MIN) / 2.0)
        ));
        assert!(close(far.y, 1.5 * f64::from(i32::MIN)));
    }

    #[test]
    fn hex_containing_finds_each_hex_from_its_center_and_corners() {
        let coords = [
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            HexCoord::new(0, 1),
            HexCoord::new(-3, 2),
            HexCoord::new(5, -7),
            HexCoord::new(-100, -100),
            HexCoord::new(i32::MAX, i32::MIN),
            HexCoord::new(i32::MIN, i32::MAX),
            HexCoord::new(i32::MAX, i32::MAX),
            HexCoord::new(i32::MIN, i32::MIN),
        ];
        for coord in coords {
            let center = hex_center(coord);
            assert_eq!(hex_containing(center), Some(coord), "{coord}");
            for corner in hex_corners(center, 1.0) {
                // A point slightly inside the corner.
                let inside = Point::new(
                    center.x + (corner.x - center.x) * 0.98,
                    center.y + (corner.y - center.y) * 0.98,
                );
                assert_eq!(hex_containing(inside), Some(coord), "{coord}");
            }
        }
    }

    #[test]
    fn hex_containing_is_none_beyond_the_i32_range() {
        assert_eq!(hex_containing(Point::new(1e12, 0.0)), None);
        assert_eq!(hex_containing(Point::new(0.0, -1e12)), None);
        assert_eq!(hex_containing(Point::new(f64::NAN, 0.0)), None);
    }

    #[test]
    fn hex_corners_are_on_the_circle_with_one_straight_above() {
        let center = Point::new(5.0, -3.0);
        let corners = hex_corners(center, 4.0);
        for corner in corners {
            let distance = (corner.x - center.x).hypot(corner.y - center.y);
            assert!(close(distance, 4.0));
        }
        // Corner 5 is straight above the center (y points down).
        assert!(close(corners[5].x, center.x));
        assert!(close(corners[5].y, center.y - 4.0));
        // Corner 0 is the top-right one.
        assert!(corners[0].x > center.x && corners[0].y < center.y);
    }

    #[test]
    fn world_and_screen_are_inverses() {
        let view = View {
            center: Point::new(3.0, -4.0),
            scale: 7.5,
        };
        for p in [Point::new(0.0, 0.0), Point::new(-12.5, 99.0)] {
            let screen = view.world_to_screen(p, VIEWPORT);
            let back = view.screen_to_world(screen, VIEWPORT);
            assert!(close(back.x, p.x) && close(back.y, p.y));
        }
        // The view's center is drawn at the viewport's center.
        let at = view.world_to_screen(view.center, VIEWPORT);
        assert_eq!(at, VIEWPORT.center());
    }

    #[test]
    fn pan_moves_the_map_by_exactly_the_drag() {
        let mut view = View {
            center: Point::new(1.0, 2.0),
            scale: 10.0,
        };
        let p = Point::new(4.0, 5.0);
        let before = view.world_to_screen(p, VIEWPORT);
        view.pan(30.0, -12.0);
        let after = view.world_to_screen(p, VIEWPORT);
        assert!(close(after.x - before.x, 30.0));
        assert!(close(after.y - before.y, -12.0));
    }

    #[test]
    fn zoom_keeps_the_point_under_the_cursor_and_clamps() {
        let mut view = View {
            center: Point::new(1.0, 2.0),
            scale: 10.0,
        };
        let cursor = Point::new(100.0, 450.0);
        let world = view.screen_to_world(cursor, VIEWPORT);
        view.zoom_at(cursor, 1.5, VIEWPORT);
        assert!(close(view.scale, 15.0));
        let after = view.world_to_screen(world, VIEWPORT);
        assert!(close(after.x, cursor.x) && close(after.y, cursor.y));

        view.zoom_at(cursor, 1e9, VIEWPORT);
        assert_eq!(view.scale, MAX_SCALE);
        view.zoom_at(cursor, 1e-30, VIEWPORT);
        assert_eq!(view.scale, MIN_SCALE);
        // A bad factor changes nothing.
        let unchanged = view;
        view.zoom_at(cursor, f64::NAN, VIEWPORT);
        view.zoom_at(cursor, 0.0, VIEWPORT);
        assert_eq!(view, unchanged);
    }

    #[test]
    fn fit_an_empty_map() {
        let view = View::fit(Vec::new(), VIEWPORT);
        assert_eq!(view.center, Point::new(0.0, 0.0));
        assert_eq!(view.scale, DEFAULT_SCALE);
    }

    #[test]
    fn fit_a_one_hex_map() {
        let coord = HexCoord::new(3, -2);
        let view = View::fit([coord], VIEWPORT);
        assert_eq!(view.center, hex_center(coord));
        assert_eq!(view.scale, FIT_MAX_SCALE);
    }

    #[test]
    fn fit_the_sample_puts_every_hex_inside_the_viewport() {
        let sample = world_at("settings/sample");
        let coords = coords(&sample);
        assert_eq!(coords.len(), 19);
        let view = View::fit(coords.iter().copied(), VIEWPORT);
        assert!(view.scale > 0.0 && view.scale <= FIT_MAX_SCALE);
        for coord in coords {
            let screen = view.world_to_screen(hex_center(coord), VIEWPORT);
            assert!(VIEWPORT.contains_with_margin(screen, 0.0), "{coord}");
        }
    }

    #[test]
    fn fit_extreme_coordinates_gives_a_finite_positive_scale_and_shows_every_hex() {
        let extreme = world_at("crates/engine/tests/settings/hexes_valid");
        let coords = coords(&extreme);
        let view = View::fit(coords.iter().copied(), VIEWPORT);
        assert!(view.scale.is_finite() && view.scale > MIN_SCALE);
        assert!(view.center.x.is_finite() && view.center.y.is_finite());
        for coord in coords {
            let screen = view.world_to_screen(hex_center(coord), VIEWPORT);
            assert!(is_visible(&view, VIEWPORT, screen), "{coord}");
        }
    }

    #[test]
    fn fit_a_zero_size_viewport_is_finite_and_positive() {
        let sample = world_at("settings/sample");
        for (width, height) in [(0.0, 0.0), (0.0, 500.0), (500.0, 0.0), (-5.0, -5.0)] {
            let viewport = Viewport {
                min: Point::new(0.0, 0.0),
                width,
                height,
            };
            let view = View::fit(coords(&sample), viewport);
            assert!(view.scale.is_finite() && view.scale > 0.0);
        }
    }

    #[test]
    fn hex_shape_switches_at_the_thresholds() {
        let at = |scale| View {
            center: Point::new(0.0, 0.0),
            scale,
        };
        assert_eq!(at(DOT_BELOW_PX - 0.001).hex_shape(), HexShape::Dot);
        assert_eq!(at(DOT_BELOW_PX).hex_shape(), HexShape::Hexagon);
        assert_eq!(at(LABEL_FROM_PX - 0.001).hex_shape(), HexShape::Hexagon);
        assert_eq!(at(LABEL_FROM_PX).hex_shape(), HexShape::LabelledHexagon);
    }

    #[test]
    fn hex_at_finds_each_hex_at_its_screen_center() {
        for relative in [
            "settings/sample",
            "crates/engine/tests/settings/hexes_valid",
        ] {
            let world = world_at(relative);
            let view = View::fit(coords(&world), VIEWPORT);
            for (id, coord) in world.map().iter() {
                let screen = view.world_to_screen(hex_center(coord), VIEWPORT);
                assert_eq!(
                    hex_at(world.map(), &view, VIEWPORT, screen),
                    Some(id),
                    "{relative} {coord}"
                );
            }
        }
    }

    #[test]
    fn hex_at_finds_nothing_between_islands() {
        let world = world_at("crates/engine/tests/settings/hexes_valid");
        let view = View {
            center: Point::new(0.0, 0.0),
            scale: 32.0,
        };
        let between = view.world_to_screen(hex_center(HexCoord::new(10, 10)), VIEWPORT);
        assert_eq!(hex_at(world.map(), &view, VIEWPORT, between), None);
        // At a hexagon scale there's no snapping to a near miss either.
        let near = view.world_to_screen(hex_center(HexCoord::new(-1, 2)), VIEWPORT);
        assert_eq!(hex_at(world.map(), &view, VIEWPORT, near), None);
    }

    #[test]
    fn hex_at_snaps_to_dots_within_the_hover_distance() {
        let world = world_at("crates/engine/tests/settings/hexes_valid");
        // Hexes at this scale are dots; (-3, 2) is centered at world (-√3 · 2, 3).
        let view = View {
            center: Point::new(0.0, 0.0),
            scale: 1.0,
        };
        assert_eq!(view.hex_shape(), HexShape::Dot);
        let expected = world.map().id(HexCoord::new(-3, 2)).unwrap();
        let dot = view.world_to_screen(hex_center(HexCoord::new(-3, 2)), VIEWPORT);
        let near = Point::new(dot.x - (HOVER_SNAP_PX - 1.0), dot.y);
        assert_eq!(hex_at(world.map(), &view, VIEWPORT, near), Some(expected));
        let far = Point::new(dot.x - (HOVER_SNAP_PX + 1.0), dot.y);
        assert_eq!(hex_at(world.map(), &view, VIEWPORT, far), None);
    }

    #[test]
    fn is_visible_allows_the_drawn_radius_beyond_the_edge() {
        let view = View {
            center: Point::new(0.0, 0.0),
            scale: 10.0,
        };
        let inside = VIEWPORT.center();
        assert!(is_visible(&view, VIEWPORT, inside));
        // Just outside the left edge, but within the hexagon radius (10 px).
        assert!(is_visible(
            &view,
            VIEWPORT,
            Point::new(VIEWPORT.min.x - 9.0, inside.y)
        ));
        assert!(!is_visible(
            &view,
            VIEWPORT,
            Point::new(VIEWPORT.min.x - 11.0, inside.y)
        ));
        assert!(!is_visible(&view, VIEWPORT, Point::new(1e6, 1e6)));
        // A dot's margin is its own, smaller radius.
        let small = View {
            center: Point::new(0.0, 0.0),
            scale: 1.0,
        };
        assert!(is_visible(
            &small,
            VIEWPORT,
            Point::new(VIEWPORT.min.x - 1.0, inside.y)
        ));
        assert!(!is_visible(
            &small,
            VIEWPORT,
            Point::new(VIEWPORT.min.x - 2.0, inside.y)
        ));
    }
}
