//! Board and vehicle rendering.

use crate::theme::*;
use macroquad::prelude::*;
use rush_core::{Board, Orientation, Vehicle, EXIT_ROW, SIZE};

/// Where the board sits on screen.
#[derive(Clone, Copy, Debug)]
pub struct BoardGeom {
    /// Top-left corner of cell (0, 0).
    pub x: f32,
    pub y: f32,
    pub cell: f32,
}

impl BoardGeom {
    pub fn size(&self) -> f32 {
        self.cell * SIZE as f32
    }
    pub fn pad(&self) -> f32 {
        self.cell * 0.26
    }
    /// Screen rectangle of a vehicle whose along-axis offset is `pos` (cells, fractional).
    pub fn vehicle_rect(&self, v: &Vehicle, pos: f32) -> Rect {
        let len = v.length() as f32 * self.cell;
        let lane = v.lane() as f32 * self.cell;
        let along = pos * self.cell;
        match v.orientation() {
            Orientation::Horizontal => Rect::new(self.x + along, self.y + lane, len, self.cell),
            Orientation::Vertical => Rect::new(self.x + lane, self.y + along, self.cell, len),
        }
    }
    /// Mouse coordinate along a vehicle's axis.
    pub fn along(&self, o: Orientation, p: Vec2) -> f32 {
        match o {
            Orientation::Horizontal => p.x - self.x,
            Orientation::Vertical => p.y - self.y,
        }
    }
}

/// Frame, parking cells, and the animated exit gate.
pub fn draw_board(g: &BoardGeom, time: f32, exit_glow: f32) {
    let (s, p) = (g.size(), g.pad());
    let (fx, fy, fw) = (g.x - p, g.y - p, s + 2.0 * p);
    let r = g.cell * 0.32;
    // Body + bevel.
    soft_shadow(fx, fy + g.cell * 0.12, fw, fw, r, g.cell * 0.35, 0.55);
    rrect(fx, fy + g.cell * 0.08, fw, fw, r, shade(FRAME, 0.6));
    rrect(fx, fy, fw, fw, r, FRAME);
    rrect_lines(
        fx + 1.0,
        fy + 1.0,
        fw - 2.0,
        fw - 2.0,
        r,
        2.0,
        with_alpha(FRAME_EDGE, 0.8),
    );
    // Inner well.
    let wi = p * 0.35;
    rrect(
        g.x - wi,
        g.y - wi,
        s + 2.0 * wi,
        s + 2.0 * wi,
        g.cell * 0.18,
        WELL,
    );

    // Exit gap through the right frame wall.
    let ey = g.y + EXIT_ROW as f32 * g.cell;
    draw_rectangle(g.x + s - 1.0, ey + g.cell * 0.04, p + 2.0, g.cell * 0.92, WELL);
    let glow = with_alpha(EXIT, 0.25 + 0.35 * exit_glow);
    draw_rectangle(g.x + s + p - 3.0, ey + g.cell * 0.04, 3.0, g.cell * 0.92, glow);

    // Cells with painted marks.
    let inset = g.cell * 0.05;
    for row in 0..SIZE {
        for col in 0..SIZE {
            let cx = g.x + col as f32 * g.cell + inset;
            let cy = g.y + row as f32 * g.cell + inset;
            let cw = g.cell - 2.0 * inset;
            let base = if row == EXIT_ROW {
                mix(CELL, EXIT, 0.06)
            } else {
                CELL
            };
            rrect(cx, cy, cw, cw, g.cell * 0.14, base);
            let m = g.cell * 0.3;
            rrect(
                cx + cw / 2.0 - m / 2.0,
                cy + cw / 2.0 - 1.5,
                m,
                3.0,
                1.5,
                with_alpha(CELL_MARK, 0.9),
            );
        }
    }

    // Chevrons outside the exit, pulsing outwards.
    let cy = ey + g.cell / 2.0;
    for k in 0..3 {
        let phase = ((time * 1.6 - k as f32 * 0.25).rem_euclid(1.0) * std::f32::consts::PI).sin();
        let a = 0.25 + 0.75 * phase * (0.4 + 0.6 * exit_glow);
        let x = g.x + s + p + g.cell * (0.22 + 0.24 * k as f32);
        let h = g.cell * 0.2;
        let w = g.cell * 0.13;
        let t = g.cell * 0.07;
        let c = with_alpha(EXIT, a);
        draw_line(x, cy - h, x + w, cy, t, c);
        draw_line(x + w, cy, x, cy + h, t, c);
        draw_circle(x + w, cy, t / 2.0, c);
    }
}

#[derive(Clone, Copy)]
pub struct VehicleStyle {
    /// 0 = resting, 1 = picked up.
    pub lift: f32,
    pub alpha: f32,
    pub glow: Option<Color>,
}

impl Default for VehicleStyle {
    fn default() -> Self {
        VehicleStyle {
            lift: 0.0,
            alpha: 1.0,
            glow: None,
        }
    }
}

/// Draws a top-down car or truck. Horizontal vehicles face right
/// (towards the exit), vertical vehicles face down.
pub fn draw_vehicle(g: &BoardGeom, v: &Vehicle, pos: f32, style: VehicleStyle) {
    let full = g.vehicle_rect(v, pos);
    let m = g.cell * 0.09;
    let body = Rect::new(full.x + m, full.y + m, full.w - 2.0 * m, full.h - 2.0 * m);
    let lift = style.lift * g.cell * 0.06;
    let a = style.alpha;
    let base = vehicle_color(v.id());
    let r = g.cell * 0.2;
    let horizontal = v.orientation() == Orientation::Horizontal;

    // Maps (along, across) fractions of the body to screen rectangles.
    let part = |a0: f32, a1: f32, c0: f32, c1: f32| -> Rect {
        if horizontal {
            Rect::new(
                body.x + body.w * a0,
                body.y - lift + body.h * c0,
                body.w * (a1 - a0),
                body.h * (c1 - c0),
            )
        } else {
            Rect::new(
                body.x + body.w * c0,
                body.y - lift + body.h * a0,
                body.w * (c1 - c0),
                body.h * (a1 - a0),
            )
        }
    };
    let fill = |rc: Rect, rad: f32, c: Color| rrect(rc.x, rc.y, rc.w, rc.h, rad, with_alpha(c, a));
    let cells = v.length() as f32;
    // Normalise "one cell" along the body so details keep their proportions.
    let u = 1.0 / cells;

    if let Some(glow) = style.glow {
        let e = g.cell * 0.07;
        rrect(
            body.x - e,
            body.y - lift - e,
            body.w + 2.0 * e,
            body.h + 2.0 * e,
            r + e,
            glow,
        );
    }
    // Shadow grows when lifted.
    let sh = g.cell * (0.05 + 0.1 * style.lift);
    soft_shadow(
        body.x,
        body.y + sh,
        body.w,
        body.h,
        r,
        g.cell * (0.06 + 0.08 * style.lift),
        0.45 * a,
    );
    // Chassis (side depth) then body.
    rrect(
        body.x,
        body.y - lift + g.cell * 0.05,
        body.w,
        body.h,
        r,
        with_alpha(shade(base, 0.55), a),
    );
    rrect(body.x, body.y - lift, body.w, body.h, r, with_alpha(base, a));

    if v.length() == 2 {
        // Hood/roof/trunk lines.
        fill(part(0.30, 0.74, 0.14, 0.86), r * 0.7, shade(base, 0.86));
        fill(part(0.36, 0.66, 0.2, 0.8), r * 0.6, shade(base, 1.08));
        // Windshield (front) and rear window.
        fill(part(0.64, 0.76, 0.16, 0.84), r * 0.4, GLASS);
        fill(part(0.22, 0.31, 0.2, 0.8), r * 0.35, GLASS);
        // Headlights & tail lights.
        fill(part(0.92, 0.975, 0.12, 0.3), r * 0.3, hex(0xfff3c4));
        fill(part(0.92, 0.975, 0.7, 0.88), r * 0.3, hex(0xfff3c4));
        fill(part(0.02, 0.06, 0.14, 0.32), r * 0.2, hex(0x8b1e2b));
        fill(part(0.02, 0.06, 0.68, 0.86), r * 0.2, hex(0x8b1e2b));
        if v.is_target() {
            // Racing stripes.
            fill(part(0.08, 0.30, 0.40, 0.46), 1.0, hex(0xffffff));
            fill(part(0.08, 0.30, 0.54, 0.60), 1.0, hex(0xffffff));
            fill(part(0.76, 0.92, 0.40, 0.46), 1.0, hex(0xffffff));
            fill(part(0.76, 0.92, 0.54, 0.60), 1.0, hex(0xffffff));
        }
    } else {
        // Truck: cargo box + cab.
        let cab0 = 1.0 - 0.9 * u;
        fill(part(0.02, cab0 - 0.03, 0.06, 0.94), r * 0.6, shade(base, 1.18));
        for k in 1..4 {
            let x = 0.02 + (cab0 - 0.05) * k as f32 / 4.0;
            fill(part(x, x + 0.012, 0.1, 0.9), 0.0, shade(base, 0.9));
        }
        fill(part(cab0 - 0.03, cab0, 0.12, 0.88), 0.0, shade(base, 0.45));
        fill(
            part(cab0 + 0.03, cab0 + 0.22 * u, 0.12, 0.88),
            r * 0.4,
            shade(base, 0.9),
        );
        fill(part(cab0 + 0.24 * u, cab0 + 0.4 * u, 0.14, 0.86), r * 0.35, GLASS);
        fill(part(0.955, 0.985, 0.1, 0.26), r * 0.3, hex(0xfff3c4));
        fill(part(0.955, 0.985, 0.74, 0.9), r * 0.3, hex(0xfff3c4));
    }
    // Top highlight.
    let hl = part(0.06, 0.94, 0.06, 0.16);
    rrect(
        hl.x,
        hl.y,
        hl.w,
        hl.h,
        r * 0.3,
        Color::new(1.0, 1.0, 1.0, 0.16 * a),
    );
}

/// Board plus vehicles at rest — used for the menu preview.
pub fn draw_static(g: &BoardGeom, board: &Board, time: f32) {
    draw_board(g, time, 0.0);
    for v in board.vehicles() {
        draw_vehicle(g, v, v.pos() as f32, VehicleStyle::default());
    }
}
