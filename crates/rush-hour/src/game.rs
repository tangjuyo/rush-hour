//! Play screen: dragging, snapping animation, undo/reset/hint, win sequence.

use crate::draw::{draw_board, draw_vehicle, BoardGeom, VehicleStyle};
use crate::progress::{Progress, Record};
use crate::theme::*;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use rush_core::{solver, stars, Board, Challenge, Move, SIZE};

pub enum GameAction {
    None,
    Menu,
    Retry,
    Next,
}

struct Drag {
    idx: usize,
    /// Mouse offset (px) from the vehicle's leading edge at grab time.
    grab: f32,
    start: f32,
    min: f32,
    max: f32,
    current: f32,
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    rot: f32,
    spin: f32,
    color: Color,
    size: f32,
    life: f32,
}

struct Win {
    t: f32,
    stars: u8,
    new_best: bool,
    car_pos: f32,
    car_vel: f32,
    burst: bool,
}

pub struct Game {
    pub challenge: Challenge,
    board: Board,
    history: Vec<Board>,
    moves: u32,
    display: Vec<f32>,
    lift: Vec<f32>,
    drag: Option<Drag>,
    hint: Option<(Move, f32)>,
    win: Option<Win>,
    confetti: Vec<Particle>,
    best: Option<Record>,
    time: f32,
    last_move_at: f32,
    toast: Option<(String, f32)>,
}

struct Layout {
    s: f32,
    g: BoardGeom,
    panel: Rect,
}

fn layout() -> Layout {
    let s = ui_scale();
    let cell = 86.0 * s;
    let board = cell * SIZE as f32;
    let pad = cell * 0.26;
    let gap = 110.0 * s;
    let panel_w = 340.0 * s;
    let total = board + 2.0 * pad + gap + panel_w;
    let left = (screen_width() - total) / 2.0 + pad;
    let top = (screen_height() - board) / 2.0;
    Layout {
        s,
        g: BoardGeom {
            x: left,
            y: top,
            cell,
        },
        panel: Rect::new(left + board + pad + gap, top - pad, panel_w, board + 2.0 * pad),
    }
}

fn approach(cur: f32, target: f32, rate: f32, dt: f32) -> f32 {
    cur + (target - cur) * (1.0 - (-rate * dt).exp())
}

fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0) - 1.0;
    1.0 + t * t * (2.7 * t + 1.7)
}

impl Game {
    pub fn new(challenge: Challenge, best: Option<Record>) -> Game {
        let board = challenge.board.clone();
        let display = board.vehicles().iter().map(|v| v.pos() as f32).collect();
        let lift = vec![0.0; board.vehicles().len()];
        Game {
            challenge,
            board,
            history: Vec::new(),
            moves: 0,
            display,
            lift,
            drag: None,
            hint: None,
            win: None,
            confetti: Vec::new(),
            best,
            time: 0.0,
            last_move_at: -10.0,
            toast: None,
        }
    }

    pub fn index(&self) -> usize {
        self.challenge.number as usize - 1
    }

    fn make_move(&mut self, mv: Move) {
        if self.board.is_legal(mv) {
            self.history.push(self.board.clone());
            self.board.apply(mv).expect("checked legal");
            self.moves += 1;
            self.hint = None;
            self.last_move_at = self.time;
        }
    }

    fn undo(&mut self) {
        if let Some(prev) = self.history.pop() {
            self.board = prev;
            self.moves -= 1;
            self.hint = None;
            self.last_move_at = self.time;
        }
    }

    fn reset(&mut self) {
        if !self.history.is_empty() {
            self.board = self.history[0].clone();
            self.history.clear();
            self.moves = 0;
            self.hint = None;
            self.last_move_at = self.time;
        }
    }

    pub fn show_hint(&mut self) {
        match solver::hint(&self.board) {
            Some(mv) => self.hint = Some((mv, self.time)),
            None => self.toast = Some(("No hint available".into(), self.time)),
        }
    }

    /// Plays the optimal solution instantly (used for screenshots/tests).
    pub fn auto_solve(&mut self) {
        if let Some(path) = solver::solve(&self.board) {
            for mv in path {
                self.make_move(mv);
            }
        }
    }

    fn exit_clear(&self) -> bool {
        let t = self.board.target();
        let (_, hi) = self.board.slide_range(0);
        t.pos() as i8 + hi + t.length() as i8 == SIZE as i8
    }

    fn handle_board_input(&mut self, g: &BoardGeom) {
        let m: Vec2 = mouse_position().into();
        if self.drag.is_none() && is_mouse_button_pressed(MouseButton::Left) {
            let hit = self
                .board
                .vehicles()
                .iter()
                .enumerate()
                .find(|(_, v)| g.vehicle_rect(v, v.pos() as f32).contains(m));
            if let Some((idx, v)) = hit {
                let (lo, hi) = self.board.slide_range(idx);
                let start = v.pos() as f32;
                self.drag = Some(Drag {
                    idx,
                    grab: g.along(v.orientation(), m) - start * g.cell,
                    start,
                    min: start + lo as f32,
                    max: start + hi as f32,
                    current: start,
                });
            }
        }
        if let Some(d) = &mut self.drag {
            let o = self.board.vehicles()[d.idx].orientation();
            d.current = ((g.along(o, m) - d.grab) / g.cell).clamp(d.min, d.max);
            if !is_mouse_button_down(MouseButton::Left) {
                // Snap: commit to a cell once dragged ~40% of the way into it.
                let off = d.current - d.start;
                let delta = (off.signum() * (off.abs() + 0.1).floor()) as i8;
                let mv = Move {
                    vehicle: d.idx,
                    delta,
                };
                self.drag = None;
                if delta != 0 {
                    self.make_move(mv);
                }
            }
        }
    }

    fn start_win(&mut self, progress: &mut Progress) {
        let st = stars(self.moves, self.challenge.par);
        let new_best = progress.record(self.challenge.number, self.moves, st);
        self.best = progress.get(self.challenge.number);
        self.win = Some(Win {
            t: 0.0,
            stars: st,
            new_best,
            car_pos: self.display[0],
            car_vel: 1.0,
            burst: false,
        });
    }

    fn burst(&mut self, origin: Vec2, s: f32) {
        let palette = [
            hex(0xe63946),
            hex(0xffd166),
            hex(0x5dd39e),
            hex(0x4cc9f0),
            hex(0xf15bb5),
            hex(0x9b5de5),
            hex(0xff9f1c),
        ];
        for _ in 0..170 {
            let angle = gen_range(-3.0f32, -0.25);
            let speed = gen_range(250.0, 1050.0) * s;
            self.confetti.push(Particle {
                pos: origin,
                vel: vec2(angle.cos() * speed, angle.sin() * speed),
                rot: gen_range(0.0, std::f32::consts::TAU),
                spin: gen_range(-14.0, 14.0),
                color: palette[gen_range(0, palette.len())],
                size: gen_range(6.0, 13.0) * s,
                life: gen_range(1.8, 3.2),
            });
        }
    }

    pub fn frame(&mut self, fonts: &Fonts, progress: &mut Progress, has_next: bool) -> GameAction {
        let dt = get_frame_time().min(1.0 / 20.0);
        self.time += dt;
        let Layout { s, g, panel } = layout();
        let mut action = GameAction::None;

        // ---- Input -------------------------------------------------------
        if self.win.is_none() {
            self.handle_board_input(&g);
            if self.drag.is_none() {
                if is_key_pressed(KeyCode::Z)
                    || is_key_pressed(KeyCode::U)
                    || is_key_pressed(KeyCode::Backspace)
                {
                    self.undo();
                }
                if is_key_pressed(KeyCode::R) {
                    self.reset();
                }
                if is_key_pressed(KeyCode::H) {
                    self.show_hint();
                }
            }
            if is_key_pressed(KeyCode::Escape) {
                action = GameAction::Menu;
            }
            if self.board.is_solved() && self.drag.is_none() {
                self.start_win(progress);
            }
        } else if let Some(w) = &self.win {
            if w.t > 1.0 {
                if (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::N)) && has_next {
                    action = GameAction::Next;
                }
                if is_key_pressed(KeyCode::R) {
                    action = GameAction::Retry;
                }
                if is_key_pressed(KeyCode::Escape) {
                    action = GameAction::Menu;
                }
            }
        }

        // ---- Animation ---------------------------------------------------
        let m: Vec2 = mouse_position().into();
        let hovered = if self.drag.is_none() && self.win.is_none() {
            self.board
                .vehicles()
                .iter()
                .position(|v| g.vehicle_rect(v, v.pos() as f32).contains(m))
        } else {
            None
        };
        for i in 0..self.display.len() {
            let dragging = self.drag.as_ref().filter(|d| d.idx == i);
            let target = dragging.map_or(self.board.vehicles()[i].pos() as f32, |d| d.current);
            let rate = if dragging.is_some() { 40.0 } else { 16.0 };
            self.display[i] = approach(self.display[i], target, rate, dt);
            let lift_target = if dragging.is_some() {
                1.0
            } else if hovered == Some(i) {
                0.4
            } else {
                0.0
            };
            self.lift[i] = approach(self.lift[i], lift_target, 18.0, dt);
        }
        let exit_point = vec2(g.x + g.size() + g.pad(), g.y + 2.5 * g.cell);
        let mut do_burst = false;
        if let Some(w) = &mut self.win {
            w.t += dt;
            if w.t > 0.12 {
                w.car_vel += 38.0 * dt;
                w.car_pos += w.car_vel * dt;
            }
            if !w.burst && w.car_pos > SIZE as f32 - 1.0 {
                w.burst = true;
                do_burst = true;
            }
        }
        if do_burst {
            self.burst(exit_point, s);
        }
        for p in &mut self.confetti {
            p.vel.y += 1500.0 * s * dt;
            p.vel *= 1.0 - 0.9 * dt;
            p.pos += p.vel * dt;
            p.rot += p.spin * dt;
            p.life -= dt;
        }
        self.confetti
            .retain(|p| p.life > 0.0 && p.pos.y < screen_height() + 40.0);

        // ---- Draw ---------------------------------------------------------
        backdrop();
        let glow = if self.exit_clear() || self.win.is_some() {
            0.5 + 0.5 * (self.time * 5.0).sin()
        } else {
            0.0
        };
        draw_board(&g, self.time, glow);

        let pulse = 0.5 + 0.5 * (self.time * 6.0).sin();
        if let Some((mv, _)) = self.hint {
            let v = &self.board.vehicles()[mv.vehicle];
            draw_vehicle(
                &g,
                v,
                v.pos() as f32 + mv.delta as f32,
                VehicleStyle {
                    alpha: 0.28 + 0.12 * pulse,
                    ..Default::default()
                },
            );
        }
        // Draw resting vehicles first, the active one last (on top).
        let top = self.drag.as_ref().map(|d| d.idx).or(hovered);
        let order = (0..self.display.len()).filter(|&i| Some(i) != top).chain(top);
        for i in order {
            let v = &self.board.vehicles()[i];
            let mut style = VehicleStyle {
                lift: self.lift[i],
                ..Default::default()
            };
            let mut pos = self.display[i];
            if let Some((mv, _)) = self.hint {
                if mv.vehicle == i {
                    style.glow = Some(with_alpha(HINT, 0.35 + 0.45 * pulse));
                }
            }
            if i == 0 {
                if let Some(w) = &self.win {
                    pos = w.car_pos;
                    style.alpha = (1.0 - (pos - SIZE as f32 + 0.5) / 1.6).clamp(0.0, 1.0);
                }
            }
            draw_vehicle(&g, v, pos, style);
        }

        if let GameAction::None = action {
            action = self.draw_panel(fonts, panel, s);
        }
        for p in &self.confetti {
            let a = (p.life / 0.6).min(1.0);
            draw_rectangle_ex(
                p.pos.x,
                p.pos.y,
                p.size,
                p.size * 0.55,
                DrawRectangleParams {
                    offset: vec2(0.5, 0.5),
                    rotation: p.rot,
                    color: with_alpha(p.color, a),
                },
            );
        }
        if let Some((msg, at)) = &self.toast {
            let age = self.time - at;
            if age < 1.6 {
                let a = (1.6 - age).min(1.0);
                let c = vec2(g.x + g.size() / 2.0, g.y - 40.0 * s);
                rrect(
                    c.x - 150.0 * s,
                    c.y - 20.0 * s,
                    300.0 * s,
                    40.0 * s,
                    20.0 * s,
                    with_alpha(PANEL, 0.95 * a),
                );
                text(
                    &fonts.ui,
                    msg,
                    c.x,
                    c.y,
                    18.0 * s,
                    with_alpha(TEXT, a),
                    Align::Center,
                );
            }
        }
        if self.win.as_ref().is_some_and(|w| w.t > 0.9) {
            if let Some(a) = self.draw_win_card(fonts, s, has_next) {
                action = a;
            }
        }
        action
    }

    fn draw_panel(&mut self, fonts: &Fonts, p: Rect, s: f32) -> GameAction {
        let mut action = GameAction::None;
        let r = 22.0 * s;
        soft_shadow(p.x, p.y + 10.0 * s, p.w, p.h, r, 26.0 * s, 0.5);
        rrect(p.x, p.y, p.w, p.h, r, PANEL);
        rrect_lines(p.x, p.y, p.w, p.h, r, 2.0, PANEL_EDGE);
        let pad = 28.0 * s;
        let x = p.x + pad;
        let w = p.w - 2.0 * pad;
        let tier = self.challenge.tier;

        text(
            &fonts.ui,
            "CHALLENGE",
            x,
            p.y + 40.0 * s,
            16.0 * s,
            MUTED,
            Align::Left,
        );
        text(
            &fonts.display,
            &self.challenge.number.to_string(),
            x,
            p.y + 92.0 * s,
            72.0 * s,
            TEXT,
            Align::Left,
        );
        // Tier pill.
        let pill_w = 150.0 * s;
        let pill = Rect::new(p.right() - pad - pill_w, p.y + 74.0 * s, pill_w, 36.0 * s);
        rrect(pill.x, pill.y, pill.w, pill.h, pill.h / 2.0, tier_color(tier));
        text(
            &fonts.ui,
            tier.name(),
            pill.center().x,
            pill.center().y,
            17.0 * s,
            hex(0x10151f),
            Align::Center,
        );

        // Stat cards.
        let cy = p.y + 150.0 * s;
        let ch = 112.0 * s;
        let cw = (w - 2.0 * 12.0 * s) / 3.0;
        let pop = 1.0 + 0.3 * (-(self.time - self.last_move_at) * 9.0).exp();
        let best_txt = self.best.map_or("—".to_string(), |b| b.best_moves.to_string());
        let cards: [(&str, String, Color, f32); 3] = [
            ("MOVES", self.moves.to_string(), TEXT, pop),
            ("PAR", self.challenge.par.to_string(), STAR_GOLD, 1.0),
            ("BEST", best_txt, EXIT, 1.0),
        ];
        for (k, (label, value, color, scale)) in cards.iter().enumerate() {
            let cx = x + k as f32 * (cw + 12.0 * s);
            rrect(cx, cy, cw, ch, 16.0 * s, hex(0x1a2030));
            text(
                &fonts.ui,
                label,
                cx + cw / 2.0,
                cy + 24.0 * s,
                13.0 * s,
                MUTED,
                Align::Center,
            );
            text(
                &fonts.display,
                value,
                cx + cw / 2.0,
                cy + 64.0 * s,
                44.0 * s * scale,
                *color,
                Align::Center,
            );
            if k == 2 {
                let filled = self.best.map_or(0, |b| b.stars);
                star_row(cx + cw / 2.0, cy + 95.0 * s, 7.0 * s, filled, 3);
            }
        }

        // Buttons.
        let by = cy + ch + 34.0 * s;
        let bh = 54.0 * s;
        let bw = (w - 12.0 * s) / 2.0;
        let active = self.win.is_none() && self.drag.is_none();
        let row1 = Rect::new(x, by, bw, bh);
        let row1b = Rect::new(x + bw + 12.0 * s, by, bw, bh);
        let row2 = Rect::new(x, by + bh + 16.0 * s, bw, bh);
        let row2b = Rect::new(x + bw + 12.0 * s, by + bh + 16.0 * s, bw, bh);
        if button(
            fonts,
            row1,
            "Undo",
            "Z",
            hex(0x4cc9f0),
            active && !self.history.is_empty(),
        ) {
            self.undo();
        }
        if button(
            fonts,
            row1b,
            "Reset",
            "R",
            hex(0xf9a03f),
            active && !self.history.is_empty(),
        ) {
            self.reset();
        }
        if button(fonts, row2, "Hint", "H", HINT, active) {
            self.show_hint();
        }
        if button(fonts, row2b, "Menu", "Esc", hex(0xb8c0cc), self.win.is_none()) {
            action = GameAction::Menu;
        }

        // Help.
        let hy = p.bottom() - 62.0 * s;
        text(
            &fonts.ui,
            "Drag vehicles along their lane.",
            x,
            hy,
            16.0 * s,
            MUTED,
            Align::Left,
        );
        text(
            &fonts.ui,
            "Get the red car to the exit!",
            x,
            hy + 26.0 * s,
            16.0 * s,
            MUTED,
            Align::Left,
        );
        action
    }

    fn draw_win_card(&mut self, fonts: &Fonts, s: f32, has_next: bool) -> Option<GameAction> {
        let w = self.win.as_ref()?;
        let t = w.t - 0.9;
        let appear = ease_out_back((t / 0.45).min(1.0));
        let fade = (t / 0.3).clamp(0.0, 1.0);
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.02, 0.03, 0.06, 0.6 * fade),
        );

        let (cw, ch) = (480.0 * s, 400.0 * s);
        let cx = screen_width() / 2.0;
        let cy = screen_height() / 2.0 + (1.0 - appear) * 60.0 * s;
        let card = Rect::new(cx - cw / 2.0, cy - ch / 2.0, cw, ch);
        let r = 28.0 * s;
        soft_shadow(card.x, card.y + 14.0 * s, cw, ch, r, 34.0 * s, 0.6 * fade);
        rrect(card.x, card.y, cw, ch, r, with_alpha(PANEL, fade));
        rrect_lines(card.x, card.y, cw, ch, r, 2.0, with_alpha(PANEL_EDGE, fade));

        text(
            &fonts.display,
            "Solved!",
            cx,
            card.y + 64.0 * s,
            60.0 * s,
            with_alpha(TEXT, fade),
            Align::Center,
        );

        // Stars pop in one by one.
        for i in 0..3u8 {
            let st = ease_out_back(((t - 0.25 - 0.18 * i as f32) / 0.35).clamp(0.0, 1.0));
            let sx = cx + (i as f32 - 1.0) * 78.0 * s;
            let sy = card.y + 150.0 * s - if i == 1 { 14.0 * s } else { 0.0 };
            let rad = 32.0 * s;
            star(sx, sy, rad, with_alpha(MUTED, 0.22 * fade));
            if i < w.stars && st > 0.0 {
                star(sx, sy + 3.0 * s, rad * st, shade(STAR_GOLD, 0.6));
                star(sx, sy, rad * st, STAR_GOLD);
            }
        }
        let par = self.challenge.par;
        let line = format!(
            "{} move{}  ·  par {}",
            self.moves,
            if self.moves == 1 { "" } else { "s" },
            par
        );
        text(
            &fonts.ui,
            &line,
            cx,
            card.y + 222.0 * s,
            24.0 * s,
            with_alpha(TEXT, fade),
            Align::Center,
        );
        let (msg, col) = if self.moves <= par {
            ("Perfect: optimal solution!", EXIT)
        } else if w.new_best {
            ("New personal best!", STAR_GOLD)
        } else {
            ("Can you do it in fewer moves?", MUTED)
        };
        text(
            &fonts.ui,
            msg,
            cx,
            card.y + 256.0 * s,
            19.0 * s,
            with_alpha(col, fade),
            Align::Center,
        );

        let bw = (cw - 2.0 * 30.0 * s - 2.0 * 12.0 * s) / 3.0;
        let bh = 56.0 * s;
        let by = card.bottom() - 30.0 * s - bh;
        let bx = card.x + 30.0 * s;
        let mut action = None;
        if t > 0.3 {
            if button(fonts, Rect::new(bx, by, bw, bh), "Retry", "", hex(0xf9a03f), true) {
                action = Some(GameAction::Retry);
            }
            if button(
                fonts,
                Rect::new(bx + bw + 12.0 * s, by, bw, bh),
                "Menu",
                "",
                hex(0xb8c0cc),
                true,
            ) {
                action = Some(GameAction::Menu);
            }
            if button(
                fonts,
                Rect::new(bx + 2.0 * (bw + 12.0 * s), by, bw, bh),
                "Next",
                "",
                EXIT,
                has_next,
            ) {
                action = Some(GameAction::Next);
            }
        }
        action
    }
}
