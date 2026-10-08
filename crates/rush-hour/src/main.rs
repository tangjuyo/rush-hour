//! Rush Hour — desktop game.
//!
//! Run: `cargo run --release -p rush-hour`
//!
//! Developer flags (for automated screenshots):
//! `--level N` open challenge N directly, `--hint` show a hint,
//! `--solve` auto-play the optimal solution, `--screenshot FILE` save a PNG
//! after `--frames K` frames (default 90) and exit.
//! Bot Lab: `--lab N` opens an N×N lab, `--seed S` fixes the seed and
//! `--lab-play` presses Play as soon as the grid is ready.

mod draw;
mod fx;
mod game;
mod lab;
mod menu;
mod progress;
mod theme;

use game::{Game, GameAction};
use lab::{Lab, LabAction};
use macroquad::prelude::*;
use menu::{Menu, MenuAction};
use progress::Progress;
use theme::{Fonts, BG_BOTTOM};

fn window_conf() -> Conf {
    Conf {
        window_title: "Rush Hour".to_owned(),
        window_width: 1200,
        window_height: 800,
        window_resizable: true,
        sample_count: 4,
        ..Default::default()
    }
}

struct DevArgs {
    lab: Option<u8>,
    seed: Option<u64>,
    lab_play: bool,
    level: Option<usize>,
    hint: bool,
    solve: bool,
    screenshot: Option<String>,
    frames: u32,
}

fn dev_args() -> DevArgs {
    let args: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    };
    DevArgs {
        lab: value("--lab").and_then(|v| v.parse().ok()),
        seed: value("--seed").and_then(|v| v.parse().ok()),
        lab_play: args.iter().any(|a| a == "--lab-play"),
        level: value("--level").and_then(|v| v.parse().ok()),
        hint: args.iter().any(|a| a == "--hint"),
        solve: args.iter().any(|a| a == "--solve"),
        screenshot: value("--screenshot"),
        frames: value("--frames").and_then(|v| v.parse().ok()).unwrap_or(90),
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let dev = dev_args();
    let fonts = Fonts::load();
    let levels = rush_core::level::builtin();
    let mut progress = if dev.screenshot.is_some() {
        Progress::ephemeral()
    } else {
        Progress::load()
    };
    let mut menu = Menu::new();
    let mut game: Option<Game> = None;
    macroquad::rand::srand(macroquad::miniquad::date::now().to_bits());
    let mut lab: Option<Lab> = dev.lab.map(|n| Lab::new(n, dev.seed));
    let mut auto_play = dev.lab_play;

    if let Some(n) = dev.level.filter(|n| (1..=levels.len()).contains(n)) {
        let mut g = Game::new(levels[n - 1].clone(), progress.get(n as u32));
        if dev.hint {
            g.show_hint();
        }
        if dev.solve {
            g.auto_solve();
        }
        game = Some(g);
    }

    let mut frame = 0u32;
    loop {
        clear_background(BG_BOTTOM);
        if let Some(l) = &mut lab {
            if auto_play && l.is_ready() {
                l.play();
                auto_play = false;
            }
            if let LabAction::Back = l.frame(&fonts) {
                lab = None;
            }
        } else if let Some(g) = &mut game {
            let idx = g.index();
            let has_next = idx + 1 < levels.len();
            match g.frame(&fonts, &mut progress, has_next) {
                GameAction::None => {}
                GameAction::Menu => {
                    menu.selected = idx;
                    game = None;
                }
                GameAction::Retry => {
                    game = Some(Game::new(levels[idx].clone(), progress.get(idx as u32 + 1)));
                }
                GameAction::Next => {
                    let n = (idx + 1).min(levels.len() - 1);
                    menu.selected = n;
                    game = Some(Game::new(levels[n].clone(), progress.get(n as u32 + 1)));
                }
            }
        } else {
            match menu.frame(&fonts, &levels, &progress) {
                MenuAction::Play(i) => game = Some(Game::new(levels[i].clone(), progress.get(i as u32 + 1))),
                MenuAction::Lab => lab = Some(Lab::new(8, None)),
                MenuAction::None => {}
            }
        }

        frame += 1;
        if let Some(path) = &dev.screenshot {
            if frame == dev.frames {
                let mut img = get_screen_data();
                // Framebuffer alpha is meaningless on screen; make the PNG opaque.
                for i in (3..img.bytes.len()).step_by(4) {
                    img.bytes[i] = 255;
                }
                img.export_png(path);
                break;
            }
        }
        next_frame().await;
    }
}
