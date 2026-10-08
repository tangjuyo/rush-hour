//! Level select: 4 tiers × 10 challenges with a live preview.

use crate::draw::{draw_static, draw_vehicle, BoardGeom, VehicleStyle};
use crate::progress::Progress;
use crate::theme::*;
use macroquad::prelude::*;
use rush_core::{Challenge, Orientation, Tier, Vehicle};

pub enum MenuAction {
    None,
    Play(usize),
    Lab,
}

pub struct Menu {
    pub selected: usize,
    time: f32,
    /// Smoothed hover lift per tile.
    lift: [f32; 40],
}

impl Menu {
    pub fn new() -> Menu {
        Menu {
            selected: 0,
            time: 0.0,
            lift: [0.0; 40],
        }
    }

    pub fn frame(&mut self, fonts: &Fonts, levels: &[Challenge], progress: &Progress) -> MenuAction {
        let dt = get_frame_time().min(1.0 / 20.0);
        self.time += dt;
        let s = ui_scale();
        let (sw, sh) = (screen_width(), screen_height());
        let mut action = MenuAction::None;
        backdrop();

        // ---- Title -----------------------------------------------------------
        let title_y = sh / 2.0 - 315.0 * s;
        let tw = text(
            &fonts.display,
            "RUSH HOUR",
            sw / 2.0,
            title_y + 5.0 * s,
            86.0 * s,
            hex(0x0a0d14),
            Align::Center,
        );
        text(
            &fonts.display,
            "RUSH HOUR",
            sw / 2.0,
            title_y,
            86.0 * s,
            TEXT,
            Align::Center,
        );
        // A little red car racing out of the title.
        let cell = 44.0 * s;
        let car = Vehicle::new('A', Orientation::Horizontal, 2, 0).expect("valid");
        let bob = (self.time * 3.0).sin() * 2.0 * s;
        let g = BoardGeom::classic(sw / 2.0 + tw / 2.0 + 18.0 * s, title_y - 2.5 * cell + bob, cell);
        draw_vehicle(&g, &car, 0.0, VehicleStyle::default());
        text(
            &fonts.ui,
            "Slide the traffic. Free the red car.",
            sw / 2.0,
            title_y + 62.0 * s,
            21.0 * s,
            MUTED,
            Align::Center,
        );

        // ---- Tile grid ---------------------------------------------------------
        let tile = 58.0 * s;
        let gap = 10.0 * s;
        let grid_w = 10.0 * tile + 9.0 * gap;
        let preview_w = 340.0 * s;
        let col_gap = 50.0 * s;
        let x0 = (sw - (grid_w + col_gap + preview_w)) / 2.0;
        let y0 = title_y + 112.0 * s;
        let row_h = 118.0 * s;
        let m: Vec2 = mouse_position().into();
        let mut hovered = None;

        for (t, tier) in Tier::ALL.iter().enumerate() {
            let ry = y0 + t as f32 * row_h;
            let color = tier_color(*tier);
            rrect(x0, ry + 6.0 * s, 6.0 * s, 20.0 * s, 3.0 * s, color);
            text(
                &fonts.ui,
                tier.name(),
                x0 + 16.0 * s,
                ry + 16.0 * s,
                20.0 * s,
                TEXT,
                Align::Left,
            );
            let numbers = levels.iter().filter(|c| c.tier == *tier).map(|c| c.number);
            let done = progress.solved_count(numbers);
            text(
                &fonts.ui,
                &format!("{done}/10"),
                x0 + grid_w,
                ry + 16.0 * s,
                16.0 * s,
                MUTED,
                Align::Right,
            );
            for k in 0..10 {
                let i = t * 10 + k;
                let Some(ch) = levels.get(i) else { continue };
                let tx = x0 + k as f32 * (tile + gap);
                let ty = ry + 36.0 * s;
                let rect = Rect::new(tx, ty, tile, tile);
                let hover = rect.contains(m);
                if hover {
                    hovered = Some(i);
                }
                let target = if hover {
                    1.0
                } else if self.selected == i {
                    0.5
                } else {
                    0.0
                };
                self.lift[i] += (target - self.lift[i]) * (1.0 - (-16.0 * dt).exp());
                let lift = self.lift[i] * 4.0 * s;
                let rec = progress.get(ch.number);
                let face = if rec.is_some() { color } else { hex(0x2a3346) };
                let r = 14.0 * s;
                soft_shadow(tx, ty + 6.0 * s, tile, tile, r, 8.0 * s, 0.35);
                rrect(tx, ty + 5.0 * s, tile, tile, r, shade(face, 0.6));
                rrect(tx, ty - lift, tile, tile, r, face);
                if self.selected == i {
                    rrect_lines(
                        tx - 3.0 * s,
                        ty - lift - 3.0 * s,
                        tile + 6.0 * s,
                        tile + 6.0 * s,
                        r + 3.0 * s,
                        2.5 * s,
                        TEXT,
                    );
                }
                let num_color = if rec.is_some() { hex(0x10151f) } else { TEXT };
                text(
                    &fonts.display,
                    &ch.number.to_string(),
                    tx + tile / 2.0,
                    ty - lift + tile * 0.4,
                    26.0 * s,
                    num_color,
                    Align::Center,
                );
                let filled = rec.map_or(0, |r| r.stars);
                for st in 0..3u8 {
                    let sx = tx + tile / 2.0 + (st as f32 - 1.0) * 12.0 * s;
                    let c = if st < filled {
                        hex(0x10151f)
                    } else {
                        with_alpha(MUTED, 0.35)
                    };
                    star(sx, ty - lift + tile * 0.76, 5.0 * s, c);
                }
                if hover && is_mouse_button_released(MouseButton::Left) {
                    action = MenuAction::Play(i);
                }
            }
        }
        if let Some(i) = hovered {
            self.selected = i;
        }

        // Keyboard navigation.
        let sel = self.selected as i32;
        let moved = if is_key_pressed(KeyCode::Right) {
            Some(sel + 1)
        } else if is_key_pressed(KeyCode::Left) {
            Some(sel - 1)
        } else if is_key_pressed(KeyCode::Down) {
            Some(sel + 10)
        } else if is_key_pressed(KeyCode::Up) {
            Some(sel - 10)
        } else {
            None
        };
        if let Some(n) = moved {
            self.selected = n.clamp(0, levels.len() as i32 - 1) as usize;
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
            action = MenuAction::Play(self.selected);
        }

        // ---- Preview -------------------------------------------------------------
        if let Some(ch) = levels.get(self.selected) {
            let px = x0 + grid_w + col_gap;
            let py = y0;
            let ph = 4.0 * row_h - 18.0 * s;
            let r = 22.0 * s;
            soft_shadow(px, py + 10.0 * s, preview_w, ph, r, 24.0 * s, 0.5);
            rrect(px, py, preview_w, ph, r, PANEL);
            rrect_lines(px, py, preview_w, ph, r, 2.0, PANEL_EDGE);
            let pad = 26.0 * s;
            text(
                &fonts.display,
                &format!("Challenge {}", ch.number),
                px + pad,
                py + 38.0 * s,
                32.0 * s,
                TEXT,
                Align::Left,
            );
            let color = tier_color(ch.tier);
            text(
                &fonts.ui,
                ch.tier.name(),
                px + pad,
                py + 70.0 * s,
                17.0 * s,
                color,
                Align::Left,
            );
            let cell = 32.0 * s;
            let g = BoardGeom::classic(px + (preview_w - 6.0 * cell) / 2.0, py + 104.0 * s, cell);
            draw_static(&g, &ch.board, self.time);
            let iy = g.y + g.size() + 36.0 * s;
            text(
                &fonts.ui,
                &format!("Par: {} moves", ch.par),
                px + pad,
                iy,
                18.0 * s,
                TEXT,
                Align::Left,
            );
            match progress.get(ch.number) {
                Some(rec) => {
                    text(
                        &fonts.ui,
                        &format!("Best: {}", rec.best_moves),
                        px + pad,
                        iy + 28.0 * s,
                        18.0 * s,
                        EXIT,
                        Align::Left,
                    );
                    star_row(
                        px + preview_w - pad - 34.0 * s,
                        iy + 14.0 * s,
                        10.0 * s,
                        rec.stars,
                        3,
                    );
                }
                None => {
                    text(
                        &fonts.ui,
                        "Not solved yet",
                        px + pad,
                        iy + 28.0 * s,
                        18.0 * s,
                        MUTED,
                        Align::Left,
                    );
                }
            }
            let b = Rect::new(
                px + pad,
                py + ph - pad - 52.0 * s,
                preview_w - 2.0 * pad,
                52.0 * s,
            );
            if button(fonts, b, "Play", "Enter", EXIT, true) {
                action = MenuAction::Play(self.selected);
            }
        }

        // ---- Footer ----------------------------------------------------------------
        let fy = y0 + 4.0 * row_h + 18.0 * s;
        // Bot Lab entry, under the preview panel.
        let lab_rect = Rect::new(x0 + grid_w + col_gap, fy - 24.0 * s, preview_w, 50.0 * s);
        if button(fonts, lab_rect, "Bot Lab", "B", BOT, true) || is_key_pressed(KeyCode::B) {
            action = MenuAction::Lab;
        }
        robot(
            lab_rect.x + preview_w * 0.62,
            lab_rect.center().y + 3.0 * s,
            26.0 * s,
            Mood::Idle,
            self.time,
        );
        let total = progress.total_stars();
        star(sw / 2.0 - 70.0 * s, fy, 11.0 * s, STAR_GOLD);
        text(
            &fonts.ui,
            &format!("{total} / {}", levels.len() * 3),
            sw / 2.0 - 52.0 * s,
            fy,
            20.0 * s,
            TEXT,
            Align::Left,
        );
        action
    }
}
