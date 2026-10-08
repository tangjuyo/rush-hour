//! Bot Lab (feature 002): a robot solves random N×N grids against a 30 s
//! clock. The search runs on a background thread; this screen shows the
//! positions it explores live, then replays its best solution.

use crate::draw::{draw_grid, draw_piece, BoardGeom, PieceLook, VehicleStyle};
use crate::fx::Confetti;
use crate::theme::*;
use macroquad::miniquad;
use macroquad::prelude::*;
use rush_core::engine::{Puzzle, State};
use rush_core::lab;
use rush_core::search::{anytime, Limits, Observer, Strategy};
use rush_core::Move;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const THINK_SECONDS: f32 = 30.0;
const MAX_SAMPLES: usize = 20_000;
/// The exploration view is shown for at least this long, even when the
/// search finishes instantly.
const MIN_SHOW: f32 = 2.5;

pub enum LabAction {
    None,
    Back,
}

#[derive(Clone, Copy)]
struct Found {
    strategy: Strategy,
    len: usize,
    optimal: bool,
    at: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Finish {
    Optimal,
    TimeUp,
    Stopped,
    GaveUp,
}

#[derive(Default)]
struct Shared {
    phase: Option<Strategy>,
    expanded: u64,
    depth: u32,
    samples: Vec<State>,
    found: Vec<Found>,
    best: Option<Vec<Move>>,
    finish: Option<Finish>,
    elapsed: f32,
}

struct BotObserver {
    shared: Arc<Mutex<Shared>>,
    cancel: Arc<AtomicBool>,
    start: Instant,
    count: u64,
    last_sample: Instant,
    timed_out: bool,
}

impl Observer for BotObserver {
    fn expanded(&mut self, state: State, depth: u32, _n: u64) -> bool {
        self.count += 1;
        let early = self.count < 4000 && self.count % 16 == 0;
        if early || self.count % 512 == 0 {
            if self.cancel.load(Ordering::Relaxed) {
                return false;
            }
            if self.start.elapsed().as_secs_f32() > THINK_SECONDS {
                self.timed_out = true;
                return false;
            }
            let now = Instant::now();
            let mut sh = self.shared.lock().unwrap();
            sh.expanded = self.count;
            sh.depth = depth;
            sh.elapsed = self.start.elapsed().as_secs_f32();
            if (early || now - self.last_sample > Duration::from_millis(20)) && sh.samples.len() < MAX_SAMPLES
            {
                sh.samples.push(state);
                self.last_sample = now;
            }
        }
        true
    }

    fn phase(&mut self, strategy: Strategy) {
        self.shared.lock().unwrap().phase = Some(strategy);
    }

    fn solution(&mut self, strategy: Strategy, path: &[Move], optimal: bool) {
        let at = self.start.elapsed().as_secs_f32();
        let mut sh = self.shared.lock().unwrap();
        let better = sh.best.as_ref().is_none_or(|b| path.len() < b.len());
        if better {
            sh.found.push(Found {
                strategy,
                len: path.len(),
                optimal,
                at,
            });
            sh.best = Some(path.to_vec());
        } else if optimal {
            if let Some(f) = sh.found.last_mut() {
                f.optimal = true;
            }
        }
    }
}

struct Search {
    shared: Arc<Mutex<Shared>>,
    cancel: Arc<AtomicBool>,
}

impl Search {
    fn start(puzzle: Puzzle, start: State) -> Search {
        let shared: Arc<Mutex<Shared>> = Arc::default();
        let cancel = Arc::new(AtomicBool::new(false));
        let (sh, cn) = (Arc::clone(&shared), Arc::clone(&cancel));
        std::thread::spawn(move || {
            let mut obs = BotObserver {
                shared: Arc::clone(&sh),
                cancel: Arc::clone(&cn),
                start: Instant::now(),
                count: 0,
                last_sample: Instant::now(),
                timed_out: false,
            };
            let r = anytime(&puzzle, start, Limits::default(), &mut obs);
            let mut s = sh.lock().unwrap();
            s.expanded = obs.count;
            s.elapsed = obs.start.elapsed().as_secs_f32();
            s.finish = Some(if r.optimal {
                Finish::Optimal
            } else if cn.load(Ordering::Relaxed) {
                Finish::Stopped
            } else if obs.timed_out {
                Finish::TimeUp
            } else {
                Finish::GaveUp
            });
        });
        Search { shared, cancel }
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

type GenSlot = Arc<Mutex<Option<(Puzzle, State)>>>;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Generating,
    Ready,
    Thinking,
    Rewind(f32),
    Replaying,
    Exiting(f32),
    Done,
}

pub struct Lab {
    size: u8,
    seed: u64,
    puzzle: Option<Puzzle>,
    start: State,
    gen: Option<GenSlot>,
    search: Option<Search>,
    mode: Mode,
    display: Vec<f32>,
    target: Vec<f32>,
    cursor: f32,
    think_t: f32,
    path: Vec<Move>,
    step: usize,
    step_t: f32,
    replay_state: State,
    car_pos: f32,
    car_vel: f32,
    confetti: Confetti,
    size_v: f32,
    size_grab: bool,
    speed_v: f32,
    speed_grab: bool,
    seed_edit: Option<String>,
    time: f32,
    copied_at: f32,
    // Snapshot of the shared search state for this frame.
    snap_expanded: u64,
    snap_depth: u32,
    snap_phase: Option<Strategy>,
    snap_found: Vec<Found>,
    snap_finish: Option<Finish>,
    snap_elapsed: f32,
}

fn random_seed() -> u64 {
    macroquad::rand::gen_range(0u32, 1_000_000_000) as u64
}

impl Lab {
    pub fn new(size: u8, seed: Option<u64>) -> Lab {
        let size = size.clamp(6, 10);
        let mut lab = Lab {
            size,
            seed: seed.unwrap_or_else(random_seed),
            puzzle: None,
            start: 0,
            gen: None,
            search: None,
            mode: Mode::Generating,
            display: Vec::new(),
            target: Vec::new(),
            cursor: 0.0,
            think_t: 0.0,
            path: Vec::new(),
            step: 0,
            step_t: 0.0,
            replay_state: 0,
            car_pos: 0.0,
            car_vel: 0.0,
            confetti: Confetti::default(),
            size_v: (size - 6) as f32 / 4.0,
            size_grab: false,
            speed_v: 0.45,
            speed_grab: false,
            seed_edit: None,
            time: 0.0,
            copied_at: -10.0,
            snap_expanded: 0,
            snap_depth: 0,
            snap_phase: None,
            snap_found: Vec::new(),
            snap_finish: None,
            snap_elapsed: 0.0,
        };
        lab.generate();
        lab
    }

    fn generate(&mut self) {
        self.search = None;
        self.mode = Mode::Generating;
        self.snap_found.clear();
        self.snap_finish = None;
        self.snap_expanded = 0;
        self.snap_phase = None;
        let slot: GenSlot = Arc::default();
        let out = Arc::clone(&slot);
        let (size, seed) = (self.size, self.seed);
        std::thread::spawn(move || {
            let g = lab::generate(size, seed);
            *out.lock().unwrap() = Some(g);
        });
        self.gen = Some(slot);
    }

    pub fn play(&mut self) {
        if let Some(p) = &self.puzzle {
            self.search = Some(Search::start(p.clone(), self.start));
            self.mode = Mode::Thinking;
            self.cursor = 0.0;
            self.think_t = 0.0;
            self.snap_found.clear();
            self.snap_finish = None;
            self.path.clear();
        }
    }

    /// Ready to accept a Play command (used by dev flags).
    pub fn is_ready(&self) -> bool {
        self.mode == Mode::Ready
    }

    fn explore_rate(&self) -> f32 {
        3.0 * 20f32.powf(self.speed_v) // 3..60 positions / s
    }
    fn replay_rate(&self) -> f32 {
        1.2 * 10f32.powf(self.speed_v) // 1.2..12 moves / s
    }
    fn anim_rate(&self) -> f32 {
        7.0 + 30.0 * self.speed_v
    }

    fn set_target(&mut self, state: State) {
        if let Some(p) = &self.puzzle {
            self.target = p.positions(state).iter().map(|&x| x as f32).collect();
        }
    }

    fn start_replay(&mut self) {
        self.step = 0;
        self.step_t = 0.0;
        self.replay_state = self.start;
        self.set_target(self.start);
        self.mode = Mode::Rewind(0.0);
    }

    fn update(&mut self, dt: f32, exit_point: Vec2, s: f32) {
        // Grid generation finished?
        if let Some(slot) = &self.gen {
            let ready = slot.lock().unwrap().take();
            if let Some((p, st)) = ready {
                self.display = p.positions(st).iter().map(|&x| x as f32).collect();
                self.target = self.display.clone();
                self.puzzle = Some(p);
                self.start = st;
                self.gen = None;
                self.mode = Mode::Ready;
            }
        }
        // Snapshot shared search state.
        let mut sample = None;
        if let Some(search) = &self.search {
            let sh = search.shared.lock().unwrap();
            self.snap_expanded = sh.expanded;
            self.snap_depth = sh.depth;
            self.snap_phase = sh.phase;
            self.snap_found.clone_from(&sh.found);
            self.snap_finish = sh.finish;
            self.snap_elapsed = sh.elapsed;
            if self.mode == Mode::Thinking && !sh.samples.is_empty() {
                let len = sh.samples.len() as f32;
                self.cursor += dt * self.explore_rate();
                if sh.finish.is_none() && len - self.cursor > 30.0 {
                    self.cursor = len - 30.0; // stay close to live
                }
                self.cursor = self.cursor.min(len - 1.0);
                sample = Some((sh.samples[self.cursor as usize], self.cursor >= len - 1.0));
            }
            if self.path.is_empty() {
                if let Some(best) = &sh.best {
                    if sh.finish.is_some() {
                        self.path.clone_from(best);
                    }
                }
            }
        }

        match self.mode {
            Mode::Thinking => {
                self.think_t += dt;
                if let Some((st, at_end)) = sample {
                    self.set_target(st);
                    if self.snap_finish.is_some() && (at_end || self.think_t >= MIN_SHOW) {
                        if self.path.is_empty() {
                            self.set_target(self.start);
                            self.mode = Mode::Done;
                        } else {
                            self.start_replay();
                        }
                    }
                } else if self.snap_finish.is_some() {
                    if self.path.is_empty() {
                        self.mode = Mode::Done;
                    } else {
                        self.start_replay();
                    }
                }
            }
            Mode::Rewind(t) => {
                self.mode = if t > 0.8 {
                    Mode::Replaying
                } else {
                    Mode::Rewind(t + dt)
                };
            }
            Mode::Replaying => {
                self.step_t += dt;
                let interval = 1.0 / self.replay_rate();
                if self.step_t >= interval {
                    self.step_t = 0.0;
                    if self.step < self.path.len() {
                        if let Some(p) = &self.puzzle {
                            if let Some(next) = p.apply(self.replay_state, self.path[self.step]) {
                                self.replay_state = next;
                            }
                        }
                        self.step += 1;
                        let st = self.replay_state;
                        self.set_target(st);
                    } else if (self.display[0] - self.target[0]).abs() < 0.05 {
                        self.car_pos = self.display[0];
                        self.car_vel = 1.0;
                        self.mode = Mode::Exiting(0.0);
                    }
                }
            }
            Mode::Exiting(t) => {
                self.car_vel += 38.0 * dt;
                self.car_pos += self.car_vel * dt;
                let n = self.size as f32;
                if t < 0.5 && self.car_pos > n - 1.0 && self.car_pos - self.car_vel * dt <= n - 1.0 {
                    self.confetti.burst(exit_point, s);
                }
                self.mode = if t > 1.6 {
                    Mode::Done
                } else {
                    Mode::Exiting(t + dt)
                };
            }
            _ => {}
        }
        // Ease every vehicle toward its target.
        let k = 1.0 - (-self.anim_rate() * dt).exp();
        for (d, t) in self.display.iter_mut().zip(&self.target) {
            *d += (t - *d) * k;
        }
        self.confetti.update(dt, s);
    }

    fn mood(&self) -> Mood {
        match self.mode {
            Mode::Generating => Mood::Busy,
            Mode::Ready => Mood::Idle,
            Mode::Thinking => {
                let recent = self
                    .snap_found
                    .last()
                    .is_some_and(|f| self.snap_elapsed - f.at < 1.2);
                if recent {
                    Mood::Found
                } else {
                    Mood::Thinking
                }
            }
            _ => match (self.path.is_empty(), self.snap_finish) {
                (true, _) => Mood::Failed,
                (false, Some(Finish::Optimal)) => Mood::Optimal,
                _ => Mood::Found,
            },
        }
    }

    fn message(&self) -> String {
        let best = self.snap_found.last().map(|f| f.len);
        match self.mode {
            Mode::Generating => "Building a traffic jam...".into(),
            Mode::Ready => "Press Play and I'll find\nthe way out!".into(),
            Mode::Thinking => match (self.snap_phase, best) {
                (_, Some(b))
                    if self
                        .snap_found
                        .last()
                        .is_some_and(|f| self.snap_elapsed - f.at < 1.2) =>
                {
                    format!("Got one: {b} moves!")
                }
                (Some(Strategy::Greedy), _) => "Quick idea first...".into(),
                (Some(Strategy::Weighted(_)), _) => "Can I do it shorter?".into(),
                (Some(Strategy::AStar), _) => "Checking every shorter\nroute, to be sure...".into(),
                _ => "Hmm...".into(),
            },
            Mode::Rewind(_) | Mode::Replaying => format!("Watch this: {} moves.", self.path.len()),
            Mode::Exiting(_) | Mode::Done => match (best, self.snap_finish) {
                (None, _) => "I couldn't find a way out\nin time. New grid?".into(),
                (Some(b), Some(Finish::Optimal)) => format!("{b} moves, and that's\nproven optimal!"),
                (Some(b), Some(Finish::Stopped)) => format!("Stopped. Best so far:\n{b} moves."),
                (Some(b), _) => format!("Time's up! Best I found:\n{b} moves (not proven)."),
            },
        }
    }

    pub fn frame(&mut self, fonts: &Fonts) -> LabAction {
        let dt = get_frame_time().min(1.0 / 20.0);
        self.time += dt;
        let s = ui_scale();
        let (sw, sh) = (screen_width(), screen_height());
        let mut action = LabAction::None;

        // ---- Layout -------------------------------------------------------------
        let n = self.size;
        let b = 600.0 * s;
        let cell = b / n as f32;
        let pad = cell * 0.26;
        let panel_w = 400.0 * s;
        let gap = 70.0 * s + cell * 0.9;
        let total = b + 2.0 * pad + gap + panel_w;
        let left = (sw - total) / 2.0 + pad;
        let top = (sh - b) / 2.0 - 6.0 * s;
        let g = BoardGeom {
            x: left,
            y: top,
            cell,
            n,
        };
        let er = lab::exit_row(n);
        let exit_point = vec2(g.x + g.size() + g.pad(), g.y + (er as f32 + 0.5) * cell);
        self.update(dt, exit_point, s);

        backdrop();

        // ---- Header ---------------------------------------------------------------
        let hy = top - pad - 44.0 * s;
        if button(
            fonts,
            Rect::new(left - pad, hy - 22.0 * s, 104.0 * s, 40.0 * s),
            "Back",
            "",
            hex(0xb8c0cc),
            true,
        ) || (self.seed_edit.is_none() && is_key_pressed(KeyCode::Escape))
        {
            action = LabAction::Back;
        }
        text(
            &fonts.display,
            "Bot Lab",
            left - pad + 124.0 * s,
            hy - 2.0 * s,
            38.0 * s,
            TEXT,
            Align::Left,
        );

        // ---- Board ----------------------------------------------------------------
        let glow = if matches!(self.mode, Mode::Exiting(_) | Mode::Done) && !self.path.is_empty() {
            0.5 + 0.5 * (self.time * 5.0).sin()
        } else {
            0.0
        };
        draw_grid(&g, er, self.time, glow);
        if let Some(p) = &self.puzzle {
            let start_pos = p.positions(self.start);
            let moving = if self.mode == Mode::Replaying && self.step > 0 {
                Some(self.path[self.step - 1].vehicle)
            } else {
                None
            };
            for (i, pc) in p.pieces().iter().enumerate() {
                let mut style = VehicleStyle::default();
                let mut pos = self.display.get(i).copied().unwrap_or(0.0);
                if self.mode == Mode::Thinking && (self.target[i] - start_pos[i] as f32).abs() > 0.01 {
                    style.glow = Some(with_alpha(BOT, 0.4));
                    style.lift = 0.5;
                }
                if moving == Some(i) {
                    style.glow = Some(with_alpha(HINT, 0.55));
                    style.lift = 0.8;
                }
                if i == 0 {
                    if let Mode::Exiting(_) = self.mode {
                        pos = self.car_pos;
                        style.alpha = (1.0 - (pos - n as f32 + 0.5) / 1.6).clamp(0.0, 1.0);
                    } else if self.mode == Mode::Done && !self.path.is_empty() {
                        style.alpha = 0.0;
                    }
                }
                let look = PieceLook {
                    horizontal: pc.horizontal,
                    len: pc.len,
                    lane: pc.lane,
                    color: piece_color(i, pc.len),
                    target: i == 0,
                };
                draw_piece(&g, look, pos, style);
            }
        }
        if self.mode == Mode::Generating {
            draw_rectangle(g.x, g.y, g.size(), g.size(), with_alpha(WELL, 0.6));
            text(
                &fonts.ui,
                "Generating...",
                g.x + g.size() / 2.0,
                g.y + g.size() / 2.0,
                26.0 * s,
                TEXT,
                Align::Center,
            );
        }

        // ---- Controls under the board -----------------------------------------------
        let busy = matches!(self.mode, Mode::Thinking | Mode::Generating);
        let cy = top + b + pad + 22.0 * s;
        let row_x = left - pad;
        let size_rect = Rect::new(row_x, cy - 4.0 * s, 170.0 * s, 44.0 * s);
        let shown = format!("{n}×{n}");
        if !busy
            && slider(
                fonts,
                size_rect,
                "Grid size",
                &shown,
                &mut self.size_v,
                &mut self.size_grab,
                BOT,
            )
        {
            let new_size = 6 + (self.size_v * 4.0).round() as u8;
            if new_size != self.size {
                self.size = new_size;
            }
        }
        if busy {
            slider(
                fonts,
                size_rect,
                "Grid size",
                &shown,
                &mut self.size_v.clone(),
                &mut false,
                MUTED,
            );
        }
        if !self.size_grab {
            let snapped = (self.size - 6) as f32 / 4.0;
            if (snapped - self.size_v).abs() > 0.001 {
                self.size_v = snapped;
                if self.puzzle.as_ref().is_some_and(|p| p.size() != self.size) && !busy {
                    self.generate();
                }
            }
            if self.puzzle.as_ref().is_some_and(|p| p.size() != self.size) && !busy && self.gen.is_none() {
                self.generate();
            }
        }
        // Seed box.
        let seed_rect = Rect::new(size_rect.right() + 16.0 * s, cy, 180.0 * s, 40.0 * s);
        let m: Vec2 = mouse_position().into();
        let editing = self.seed_edit.is_some();
        rrect(
            seed_rect.x,
            seed_rect.y,
            seed_rect.w,
            seed_rect.h,
            12.0 * s,
            hex(0x151a26),
        );
        rrect_lines(
            seed_rect.x,
            seed_rect.y,
            seed_rect.w,
            seed_rect.h,
            12.0 * s,
            2.0,
            if editing { BOT } else { PANEL_EDGE },
        );
        let seed_text = match &self.seed_edit {
            Some(t) => {
                let caret = if (self.time * 2.0).fract() < 0.5 { "|" } else { " " };
                format!("#{t}{caret}")
            }
            None => format!("#{}", self.seed),
        };
        text(
            &fonts.ui,
            "SEED",
            seed_rect.x + 12.0 * s,
            seed_rect.center().y,
            12.0 * s,
            MUTED,
            Align::Left,
        );
        text(
            &fonts.ui,
            &seed_text,
            seed_rect.x + 52.0 * s,
            seed_rect.center().y,
            18.0 * s,
            TEXT,
            Align::Left,
        );
        if !busy && is_mouse_button_pressed(MouseButton::Left) {
            if seed_rect.contains(m) {
                self.seed_edit = Some(String::new());
            } else if editing {
                self.seed_edit = None;
            }
        }
        if let Some(t) = &mut self.seed_edit {
            while let Some(c) = get_char_pressed() {
                if c.is_ascii_digit() && t.len() < 9 {
                    t.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                t.pop();
            }
            if is_key_pressed(KeyCode::Escape) {
                self.seed_edit = None;
            } else if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                if let Ok(v) = t.parse::<u64>() {
                    self.seed = v;
                    self.seed_edit = None;
                    self.generate();
                } else {
                    self.seed_edit = None;
                }
            }
        } else {
            while get_char_pressed().is_some() {}
        }
        let copy_rect = Rect::new(seed_rect.right() + 10.0 * s, cy, 76.0 * s, 40.0 * s);
        let copy_label = if self.time - self.copied_at < 1.2 {
            "Copied"
        } else {
            "Copy"
        };
        if button(fonts, copy_rect, copy_label, "", hex(0x8c97ad), true) {
            miniquad::window::clipboard_set(&format!("Rush Hour Bot Lab {n}x{n} seed {}", self.seed));
            self.copied_at = self.time;
        }
        let new_rect = Rect::new(copy_rect.right() + 10.0 * s, cy, 120.0 * s, 40.0 * s);
        if button(fonts, new_rect, "New grid", "", hex(0xf9a03f), !busy) {
            self.seed = random_seed();
            self.generate();
        }

        // ---- Panel ------------------------------------------------------------------
        let p = Rect::new(left + b + pad + gap, top - pad, panel_w, b + 2.0 * pad + 62.0 * s);
        let r = 22.0 * s;
        soft_shadow(p.x, p.y + 10.0 * s, p.w, p.h, r, 26.0 * s, 0.5);
        rrect(p.x, p.y, p.w, p.h, r, PANEL);
        rrect_lines(p.x, p.y, p.w, p.h, r, 2.0, PANEL_EDGE);
        let px = p.x + 24.0 * s;
        let pw = p.w - 48.0 * s;

        robot(px + 50.0 * s, p.y + 92.0 * s, 84.0 * s, self.mood(), self.time);
        speech(
            fonts,
            Rect::new(px + 118.0 * s, p.y + 40.0 * s, pw - 118.0 * s, 92.0 * s),
            &self.message(),
            16.0 * s,
        );

        // Timer ring.
        let tc = vec2(px + 64.0 * s, p.y + 230.0 * s);
        let tr = 56.0 * s;
        let elapsed = match self.mode {
            Mode::Ready | Mode::Generating => 0.0,
            _ => self.snap_elapsed.min(THINK_SECONDS),
        };
        let frac = 1.0 - elapsed / THINK_SECONDS;
        draw_circle(tc.x, tc.y, tr, hex(0x151a26));
        let segs = 90;
        let ring_color = if frac < 0.2 { hex(0xef476f) } else { BOT };
        for k in 0..segs {
            let f0 = k as f32 / segs as f32;
            let on = f0 < frac;
            let a0 = -std::f32::consts::FRAC_PI_2 + f0 * std::f32::consts::TAU;
            let a1 = a0 + std::f32::consts::TAU / segs as f32 * 1.05;
            let c = if on { ring_color } else { hex(0x2a3346) };
            draw_line(
                tc.x + a0.cos() * tr * 0.86,
                tc.y + a0.sin() * tr * 0.86,
                tc.x + a1.cos() * tr * 0.86,
                tc.y + a1.sin() * tr * 0.86,
                9.0 * s,
                c,
            );
        }
        let secs = (THINK_SECONDS - elapsed).max(0.0);
        text(
            &fonts.display,
            &format!("{:.0}", secs.ceil()),
            tc.x,
            tc.y - 4.0 * s,
            38.0 * s,
            TEXT,
            Align::Center,
        );
        text(
            &fonts.ui,
            "SECONDS",
            tc.x,
            tc.y + 24.0 * s,
            11.0 * s,
            MUTED,
            Align::Center,
        );

        // Best.
        let bx = px + 150.0 * s;
        text(
            &fonts.ui,
            "BEST SOLUTION",
            bx,
            p.y + 192.0 * s,
            13.0 * s,
            MUTED,
            Align::Left,
        );
        let best = self.snap_found.last();
        let best_txt = best.map_or("-".to_string(), |f| f.len.to_string());
        let w = text(
            &fonts.display,
            &best_txt,
            bx,
            p.y + 232.0 * s,
            52.0 * s,
            TEXT,
            Align::Left,
        );
        if best.is_some() {
            text(
                &fonts.ui,
                "moves",
                bx + w + 8.0 * s,
                p.y + 240.0 * s,
                16.0 * s,
                MUTED,
                Align::Left,
            );
        }
        let badge = match (best, self.snap_finish) {
            (Some(_), Some(Finish::Optimal)) => Some(("OPTIMAL", EXIT)),
            (Some(_), Some(_)) => Some(("UNPROVEN", hex(0xf9a03f))),
            (Some(_), None) => Some(("IMPROVING...", BOT)),
            _ => None,
        };
        if let Some((label, color)) = badge {
            let bw = 120.0 * s;
            rrect(bx, p.y + 262.0 * s, bw, 26.0 * s, 13.0 * s, color);
            text(
                &fonts.ui,
                label,
                bx + bw / 2.0,
                p.y + 275.0 * s,
                12.0 * s,
                hex(0x10151f),
                Align::Center,
            );
        }

        // Stats.
        let sy = p.y + 308.0 * s;
        let cw = (pw - 12.0 * s) / 2.0;
        let rate = if self.snap_elapsed > 0.05 {
            self.snap_expanded as f32 / self.snap_elapsed
        } else {
            0.0
        };
        let strategy = self.snap_phase.map_or("-", |p| p.name());
        let stats = [
            ("POSITIONS EXPLORED", fmt_count(self.snap_expanded as f64)),
            ("PER SECOND", fmt_count(rate as f64)),
            ("STRATEGY", strategy.to_string()),
            ("SEARCH DEPTH", self.snap_depth.to_string()),
        ];
        for (k, (label, value)) in stats.iter().enumerate() {
            let x = px + (k % 2) as f32 * (cw + 12.0 * s);
            let y = sy + (k / 2) as f32 * 62.0 * s;
            rrect(x, y, cw, 54.0 * s, 12.0 * s, hex(0x1a2030));
            text(
                &fonts.ui,
                label,
                x + 12.0 * s,
                y + 16.0 * s,
                10.5 * s,
                MUTED,
                Align::Left,
            );
            text(
                &fonts.ui,
                value,
                x + 12.0 * s,
                y + 37.0 * s,
                17.0 * s,
                TEXT,
                Align::Left,
            );
        }

        // Solutions log.
        let ly = sy + 134.0 * s;
        text(&fonts.ui, "SOLUTIONS FOUND", px, ly, 12.0 * s, MUTED, Align::Left);
        let shown_found: Vec<&Found> = self.snap_found.iter().rev().take(4).collect();
        if shown_found.is_empty() {
            text(
                &fonts.ui,
                "none yet",
                px,
                ly + 24.0 * s,
                15.0 * s,
                with_alpha(MUTED, 0.6),
                Align::Left,
            );
        }
        for (k, f) in shown_found.iter().enumerate() {
            let y = ly + 24.0 * s + k as f32 * 22.0 * s;
            let c = if k == 0 { TEXT } else { MUTED };
            text(
                &fonts.ui,
                &format!("{:>5.2}s", f.at),
                px,
                y,
                14.0 * s,
                c,
                Align::Left,
            );
            text(
                &fonts.ui,
                f.strategy.name(),
                px + 70.0 * s,
                y,
                14.0 * s,
                c,
                Align::Left,
            );
            let tail = if f.optimal {
                format!("{} (optimal)", f.len)
            } else {
                f.len.to_string()
            };
            text(
                &fonts.ui,
                &tail,
                px + pw,
                y,
                14.0 * s,
                if f.optimal { EXIT } else { c },
                Align::Right,
            );
        }

        // Speed + main button.
        let spd = Rect::new(px, p.bottom() - 132.0 * s, pw, 44.0 * s);
        let shown = format!("{:.0} moves/s", self.replay_rate());
        slider(
            fonts,
            spd,
            "Animation speed",
            &shown,
            &mut self.speed_v,
            &mut self.speed_grab,
            HINT,
        );
        let br = Rect::new(px, p.bottom() - 74.0 * s, pw, 52.0 * s);
        match self.mode {
            Mode::Thinking => {
                if button(fonts, br, "Stop", "Space", hex(0xef476f), true) || is_key_pressed(KeyCode::Space) {
                    if let Some(sr) = &self.search {
                        sr.cancel.store(true, Ordering::Relaxed);
                    }
                }
            }
            Mode::Rewind(_) | Mode::Replaying | Mode::Exiting(_) => {
                button(fonts, br, "Replaying...", "", hex(0x8c97ad), false);
            }
            Mode::Done if !self.path.is_empty() => {
                let half = (pw - 12.0 * s) / 2.0;
                if button(
                    fonts,
                    Rect::new(br.x, br.y, half, br.h),
                    "Replay",
                    "R",
                    HINT,
                    true,
                ) || is_key_pressed(KeyCode::R)
                {
                    self.start_replay();
                }
                if button(
                    fonts,
                    Rect::new(br.x + half + 12.0 * s, br.y, half, br.h),
                    "Think again",
                    "",
                    EXIT,
                    true,
                ) {
                    self.set_target(self.start);
                    self.play();
                }
            }
            _ => {
                let ready = matches!(self.mode, Mode::Ready | Mode::Done);
                if button(fonts, br, "Play", "Space", EXIT, ready)
                    || (ready && is_key_pressed(KeyCode::Space))
                {
                    self.set_target(self.start);
                    self.play();
                }
            }
        }

        self.confetti.draw();
        action
    }
}

fn fmt_count(v: f64) -> String {
    if v >= 1e6 {
        format!("{:.2} M", v / 1e6)
    } else if v >= 1e4 {
        format!("{:.0} k", v / 1e3)
    } else if v >= 1e3 {
        format!("{:.1} k", v / 1e3)
    } else {
        format!("{v:.0}")
    }
}
