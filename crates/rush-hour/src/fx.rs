//! Shared effects: confetti.

use crate::theme::{hex, with_alpha};
use macroquad::prelude::*;
use macroquad::rand::gen_range;

struct Particle {
    pos: Vec2,
    vel: Vec2,
    rot: f32,
    spin: f32,
    color: Color,
    size: f32,
    life: f32,
}

#[derive(Default)]
pub struct Confetti {
    particles: Vec<Particle>,
}

impl Confetti {
    /// Bursts up and to the left from `origin` (the exit gate).
    pub fn burst(&mut self, origin: Vec2, s: f32) {
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
            self.particles.push(Particle {
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

    pub fn update(&mut self, dt: f32, s: f32) {
        for p in &mut self.particles {
            p.vel.y += 1500.0 * s * dt;
            p.vel *= 1.0 - 0.9 * dt;
            p.pos += p.vel * dt;
            p.rot += p.spin * dt;
            p.life -= dt;
        }
        self.particles
            .retain(|p| p.life > 0.0 && p.pos.y < screen_height() + 40.0);
    }

    pub fn draw(&self) {
        for p in &self.particles {
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
    }
}
