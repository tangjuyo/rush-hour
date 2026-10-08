//! Visual identity: palette, fonts, and primitive shapes (rounded rects,
//! stars, buttons) used by every screen. Everything is drawn with triangles
//! so translucent shapes never double-blend at the corners.

use macroquad::prelude::*;
use rush_core::Tier;

pub const fn hex(rgb: u32) -> Color {
    Color::new(
        ((rgb >> 16) & 0xff) as f32 / 255.0,
        ((rgb >> 8) & 0xff) as f32 / 255.0,
        (rgb & 0xff) as f32 / 255.0,
        1.0,
    )
}

pub fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a)
}

pub fn shade(c: Color, f: f32) -> Color {
    // f < 1 darkens, f > 1 lightens (towards white).
    if f <= 1.0 {
        Color::new(c.r * f, c.g * f, c.b * f, c.a)
    } else {
        let t = (f - 1.0).min(1.0);
        Color::new(
            c.r + (1.0 - c.r) * t,
            c.g + (1.0 - c.g) * t,
            c.b + (1.0 - c.b) * t,
            c.a,
        )
    }
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

// ---- Palette -------------------------------------------------------------

pub const BG_TOP: Color = hex(0x1d2536);
pub const BG_BOTTOM: Color = hex(0x0d1119);
pub const FRAME: Color = hex(0x2c3547);
pub const FRAME_EDGE: Color = hex(0x3d4860);
pub const WELL: Color = hex(0x1a202c);
pub const CELL: Color = hex(0x323b4d);
pub const CELL_MARK: Color = hex(0x3c465a);
pub const PANEL: Color = hex(0x222a3a);
pub const PANEL_EDGE: Color = hex(0x323c52);
pub const TEXT: Color = hex(0xf1f3f8);
pub const MUTED: Color = hex(0x8c97ad);
pub const STAR_GOLD: Color = hex(0xffc94a);
pub const EXIT: Color = hex(0x5dd39e);
pub const HINT: Color = hex(0xffd166);
pub const GLASS: Color = hex(0x1b2433);

pub fn tier_color(t: Tier) -> Color {
    match t {
        Tier::Beginner => hex(0x5dd39e),
        Tier::Intermediate => hex(0x4cc9f0),
        Tier::Advanced => hex(0xf9a03f),
        Tier::Expert => hex(0xef476f),
    }
}

/// Body colour of each vehicle (classic-inspired, all distinct).
pub fn vehicle_color(id: char) -> Color {
    hex(match id {
        'A' => 0xe63946,
        'B' => 0x8ac926,
        'C' => 0xff9f1c,
        'D' => 0x4cc9f0,
        'E' => 0xf15bb5,
        'F' => 0x9b5de5,
        'G' => 0x2a9d8f,
        'H' => 0xb8c0cc,
        'I' => 0xffd166,
        'J' => 0xa1887f,
        'K' => 0x6a994e,
        'L' => 0x3a86ff,
        'O' => 0xffbe0b,
        'P' => 0x8338ec,
        'Q' => 0x277da1,
        'R' => 0x43aa8b,
        _ => 0x888888,
    })
}

// ---- Fonts & text ----------------------------------------------------------

pub struct Fonts {
    /// Chunky display face for titles and numbers.
    pub display: Font,
    /// Rounded heavy face for UI text.
    pub ui: Font,
}

impl Fonts {
    pub fn load() -> Fonts {
        let display = load_ttf_font_from_bytes(include_bytes!("../assets/LilitaOne-Regular.ttf"))
            .expect("embedded display font");
        let ui = load_ttf_font_from_bytes(include_bytes!("../assets/Nunito-ExtraBold.ttf"))
            .expect("embedded ui font");
        Fonts { display, ui }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Draws `s` vertically centred on `cy`, horizontally aligned on `x`.
pub fn text(font: &Font, s: &str, x: f32, cy: f32, size: f32, color: Color, align: Align) -> f32 {
    let size_px = size.round().max(6.0) as u16;
    let dims = measure_text(s, Some(font), size_px, 1.0);
    let x = match align {
        Align::Left => x,
        Align::Center => x - dims.width / 2.0,
        Align::Right => x - dims.width,
    };
    // Centre on cap height rather than the full box, so digits look centred.
    let baseline = cy + size_px as f32 * 0.36;
    draw_text_ex(
        s,
        x.round(),
        baseline.round(),
        TextParams {
            font: Some(font),
            font_size: size_px,
            color,
            ..Default::default()
        },
    );
    dims.width
}

// ---- Shapes ----------------------------------------------------------------

fn rrect_points(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<Vec2> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    const SEG: usize = 6;
    let corners = [
        (x + w - r, y + r, -std::f32::consts::FRAC_PI_2),
        (x + w - r, y + h - r, 0.0),
        (x + r, y + h - r, std::f32::consts::FRAC_PI_2),
        (x + r, y + r, std::f32::consts::PI),
    ];
    let mut pts = Vec::with_capacity(4 * (SEG + 1));
    for (cx, cy, a0) in corners {
        for k in 0..=SEG {
            let a = a0 + std::f32::consts::FRAC_PI_2 * k as f32 / SEG as f32;
            pts.push(vec2(cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    pts
}

/// Filled rounded rectangle.
pub fn rrect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let pts = rrect_points(x, y, w, h, r);
    let c = vec2(x + w / 2.0, y + h / 2.0);
    for i in 0..pts.len() {
        draw_triangle(c, pts[i], pts[(i + 1) % pts.len()], color);
    }
}

/// Rounded rectangle outline.
pub fn rrect_lines(x: f32, y: f32, w: f32, h: f32, r: f32, thickness: f32, color: Color) {
    let pts = rrect_points(x, y, w, h, r);
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        draw_line(a.x, a.y, b.x, b.y, thickness, color);
    }
    for p in &pts {
        draw_circle(p.x, p.y, thickness / 2.0, color);
    }
}

/// Soft drop shadow made of a few expanding translucent layers.
pub fn soft_shadow(x: f32, y: f32, w: f32, h: f32, r: f32, spread: f32, alpha: f32) {
    for k in 0..4 {
        let e = spread * (k as f32 + 1.0) / 4.0;
        rrect(
            x - e,
            y - e,
            w + 2.0 * e,
            h + 2.0 * e,
            r + e,
            Color::new(0.0, 0.0, 0.0, alpha / 4.0),
        );
    }
}

/// Five-pointed star centred on (cx, cy).
pub fn star(cx: f32, cy: f32, r: f32, color: Color) {
    let inner = r * 0.48;
    let pts: Vec<Vec2> = (0..10)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            let rr = if k % 2 == 0 { r } else { inner };
            vec2(cx + rr * a.cos(), cy + rr * a.sin())
        })
        .collect();
    let c = vec2(cx, cy);
    for i in 0..10 {
        draw_triangle(c, pts[i], pts[(i + 1) % 10], color);
    }
}

/// A row of `total` stars, the first `filled` in gold.
pub fn star_row(cx: f32, cy: f32, r: f32, filled: u8, total: u8) {
    let gap = r * 2.3;
    let x0 = cx - gap * (total as f32 - 1.0) / 2.0;
    for i in 0..total {
        let x = x0 + gap * i as f32;
        if i < filled {
            star(x, cy + r * 0.12, r, shade(STAR_GOLD, 0.6));
            star(x, cy, r, STAR_GOLD);
        } else {
            star(x, cy, r, with_alpha(MUTED, 0.28));
        }
    }
}

/// Vertical gradient background.
pub fn backdrop() {
    let (w, h) = (screen_width(), screen_height());
    const BANDS: usize = 48;
    for i in 0..BANDS {
        let t = i as f32 / (BANDS - 1) as f32;
        let y0 = h * i as f32 / BANDS as f32;
        draw_rectangle(0.0, y0, w, h / BANDS as f32 + 1.0, mix(BG_TOP, BG_BOTTOM, t));
    }
}

/// Raised 3-D button. Returns true when clicked (released inside).
pub fn button(fonts: &Fonts, rect: Rect, label: &str, hotkey: &str, accent: Color, enabled: bool) -> bool {
    let (mx, my) = mouse_position();
    let hover = enabled && rect.contains(vec2(mx, my));
    let pressed = hover && is_mouse_button_down(MouseButton::Left);
    let depth = rect.h * 0.1;
    let r = rect.h * 0.28;
    let lift = if pressed { depth * 0.8 } else { 0.0 };
    let face = if !enabled {
        hex(0x2a3142)
    } else if hover {
        shade(accent, 1.12)
    } else {
        accent
    };
    soft_shadow(rect.x, rect.y + depth, rect.w, rect.h, r, depth * 1.2, 0.35);
    rrect(rect.x, rect.y + depth, rect.w, rect.h, r, shade(face, 0.62));
    rrect(rect.x, rect.y + lift, rect.w, rect.h, r, face);
    rrect(
        rect.x + r * 0.5,
        rect.y + lift + rect.h * 0.08,
        rect.w - r,
        rect.h * 0.22,
        r * 0.5,
        Color::new(1.0, 1.0, 1.0, if enabled { 0.12 } else { 0.03 }),
    );
    let text_color = if enabled { hex(0x10151f) } else { MUTED };
    let size = rect.h * 0.4;
    if hotkey.is_empty() {
        text(
            &fonts.ui,
            label,
            rect.center().x,
            rect.center().y + lift,
            size,
            text_color,
            Align::Center,
        );
    } else {
        text(
            &fonts.ui,
            label,
            rect.x + rect.h * 0.4,
            rect.center().y + lift,
            size,
            text_color,
            Align::Left,
        );
        text(
            &fonts.ui,
            hotkey,
            rect.right() - rect.h * 0.4,
            rect.center().y + lift,
            size * 0.78,
            with_alpha(text_color, 0.55),
            Align::Right,
        );
    }
    hover && is_mouse_button_released(MouseButton::Left)
}

/// Uniform UI scale relative to the 1200×800 reference layout.
pub fn ui_scale() -> f32 {
    (screen_width() / 1200.0).min(screen_height() / 800.0).max(0.35)
}

/// Colour of piece `i` in Bot Lab (piece 0 is the red car).
pub fn piece_color(i: usize, len: u8) -> Color {
    const CARS: [u32; 11] = [
        0x8ac926, 0xff9f1c, 0x4cc9f0, 0xf15bb5, 0x9b5de5, 0x2a9d8f, 0xb8c0cc, 0xffd166, 0xa1887f, 0x6a994e,
        0x3a86ff,
    ];
    const TRUCKS: [u32; 5] = [0xffbe0b, 0x8338ec, 0x277da1, 0x43aa8b, 0xf8961e];
    const BUSES: [u32; 4] = [0x06d6a0, 0xffb703, 0x5e60ce, 0xfb8500];
    if i == 0 {
        return vehicle_color('A');
    }
    hex(match len {
        2 => CARS[i % CARS.len()],
        3 => TRUCKS[i % TRUCKS.len()],
        _ => BUSES[i % BUSES.len()],
    })
}

pub const BOT: Color = hex(0x5ef2ff);

/// Horizontal slider. `value` is normalised to 0..=1. `grabbed` must persist
/// between frames. Returns true while the value changes.
pub fn slider(
    fonts: &Fonts,
    rect: Rect,
    label: &str,
    shown: &str,
    value: &mut f32,
    grabbed: &mut bool,
    accent: Color,
) -> bool {
    let m: Vec2 = mouse_position().into();
    let s = rect.h / 40.0;
    let track_y = rect.y + rect.h * 0.72;
    let (x0, x1) = (rect.x + 8.0 * s, rect.right() - 8.0 * s);
    text(
        &fonts.ui,
        label,
        rect.x,
        rect.y + rect.h * 0.22,
        15.0 * s,
        MUTED,
        Align::Left,
    );
    text(
        &fonts.ui,
        shown,
        rect.right(),
        rect.y + rect.h * 0.22,
        15.0 * s,
        TEXT,
        Align::Right,
    );
    let hit = Rect::new(rect.x, track_y - 12.0 * s, rect.w, 24.0 * s);
    if is_mouse_button_pressed(MouseButton::Left) && hit.contains(m) {
        *grabbed = true;
    }
    if !is_mouse_button_down(MouseButton::Left) {
        *grabbed = false;
    }
    let mut changed = false;
    if *grabbed {
        let v = ((m.x - x0) / (x1 - x0)).clamp(0.0, 1.0);
        changed = (v - *value).abs() > f32::EPSILON;
        *value = v;
    }
    let th = 6.0 * s;
    rrect(x0, track_y - th / 2.0, x1 - x0, th, th / 2.0, hex(0x151a26));
    let kx = x0 + (x1 - x0) * *value;
    rrect(x0, track_y - th / 2.0, kx - x0, th, th / 2.0, accent);
    let kr = if *grabbed || hit.contains(m) { 11.0 } else { 9.0 } * s;
    draw_circle(kx, track_y + 2.0 * s, kr, Color::new(0.0, 0.0, 0.0, 0.35));
    draw_circle(kx, track_y, kr, TEXT);
    draw_circle(kx, track_y, kr * 0.45, accent);
    changed
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mood {
    Idle,
    Busy,
    Thinking,
    Found,
    Optimal,
    Failed,
}

/// The Bot Lab robot. `size` is the head width.
pub fn robot(cx: f32, cy: f32, size: f32, mood: Mood, time: f32) {
    let u = size / 100.0;
    let bob = (time * if mood == Mood::Thinking { 7.0 } else { 2.2 }).sin() * 2.5 * u;
    let cy = cy + bob;
    let antenna = match mood {
        Mood::Idle | Mood::Busy => BOT,
        Mood::Thinking => {
            if (time * 8.0).sin() > 0.0 {
                HINT
            } else {
                shade(HINT, 0.5)
            }
        }
        Mood::Found => EXIT,
        Mood::Optimal => STAR_GOLD,
        Mood::Failed => hex(0xef476f),
    };
    let metal = hex(0xd5dde8);
    let metal_dark = hex(0x9aa6b8);
    // Antenna.
    draw_line(cx, cy - 50.0 * u, cx, cy - 72.0 * u, 4.0 * u, metal_dark);
    draw_circle(cx, cy - 76.0 * u, 12.0 * u, with_alpha(antenna, 0.25));
    draw_circle(cx, cy - 76.0 * u, 7.0 * u, antenna);
    // Ears.
    rrect(
        cx - 60.0 * u,
        cy - 18.0 * u,
        14.0 * u,
        36.0 * u,
        5.0 * u,
        metal_dark,
    );
    rrect(
        cx + 46.0 * u,
        cy - 18.0 * u,
        14.0 * u,
        36.0 * u,
        5.0 * u,
        metal_dark,
    );
    // Head.
    soft_shadow(
        cx - 50.0 * u,
        cy - 46.0 * u,
        100.0 * u,
        92.0 * u,
        26.0 * u,
        10.0 * u,
        0.4,
    );
    rrect(
        cx - 50.0 * u,
        cy - 42.0 * u,
        100.0 * u,
        92.0 * u,
        26.0 * u,
        metal_dark,
    );
    rrect(cx - 50.0 * u, cy - 50.0 * u, 100.0 * u, 92.0 * u, 26.0 * u, metal);
    // Face screen.
    rrect(
        cx - 38.0 * u,
        cy - 36.0 * u,
        76.0 * u,
        62.0 * u,
        18.0 * u,
        hex(0x141a26),
    );
    // Eyes.
    let blink = ((time * 0.7).fract() > 0.96) && mood != Mood::Thinking;
    let look = match mood {
        Mood::Thinking => (time * 3.2).sin() * 8.0 * u,
        _ => (time * 0.6).sin() * 3.0 * u,
    };
    let eye_y = cy - 10.0 * u;
    for side in [-1.0f32, 1.0] {
        let ex = cx + side * 16.0 * u + look;
        match mood {
            Mood::Optimal => star(ex, eye_y, 10.0 * u, STAR_GOLD),
            Mood::Failed => {
                draw_line(
                    ex - 7.0 * u,
                    eye_y - 2.0 * u,
                    ex + 7.0 * u,
                    eye_y + 2.0 * u,
                    3.5 * u,
                    BOT,
                );
            }
            _ if blink => {
                draw_line(ex - 7.0 * u, eye_y, ex + 7.0 * u, eye_y, 3.5 * u, BOT);
            }
            _ => {
                draw_circle(ex, eye_y, 11.0 * u, with_alpha(BOT, 0.18));
                rrect(ex - 6.0 * u, eye_y - 8.0 * u, 12.0 * u, 16.0 * u, 6.0 * u, BOT);
            }
        }
    }
    // Mouth.
    let my = cy + 12.0 * u;
    match mood {
        Mood::Found | Mood::Optimal => {
            for k in 0..7 {
                let a0 = std::f32::consts::PI * (0.15 + 0.7 * k as f32 / 7.0);
                let a1 = std::f32::consts::PI * (0.15 + 0.7 * (k + 1) as f32 / 7.0);
                draw_line(
                    cx + a0.cos() * 14.0 * u,
                    my - 6.0 * u + a0.sin() * 10.0 * u,
                    cx + a1.cos() * 14.0 * u,
                    my - 6.0 * u + a1.sin() * 10.0 * u,
                    3.5 * u,
                    BOT,
                );
            }
        }
        Mood::Failed => {
            for k in 0..7 {
                let a0 = std::f32::consts::PI * (1.15 + 0.7 * k as f32 / 7.0);
                let a1 = std::f32::consts::PI * (1.15 + 0.7 * (k + 1) as f32 / 7.0);
                draw_line(
                    cx + a0.cos() * 12.0 * u,
                    my + 6.0 * u + a0.sin() * 7.0 * u,
                    cx + a1.cos() * 12.0 * u,
                    my + 6.0 * u + a1.sin() * 7.0 * u,
                    3.5 * u,
                    BOT,
                );
            }
        }
        Mood::Thinking | Mood::Busy => {
            for k in 0..3 {
                let on = ((time * 4.0) as usize % 3) == k;
                draw_circle(
                    cx + (k as f32 - 1.0) * 9.0 * u,
                    my,
                    3.0 * u,
                    with_alpha(BOT, if on { 1.0 } else { 0.35 }),
                );
            }
        }
        Mood::Idle => {
            draw_line(cx - 10.0 * u, my, cx + 10.0 * u, my, 3.5 * u, BOT);
        }
    }
}

/// Speech bubble with a tail pointing left. Wraps `msg` on '\n'.
pub fn speech(fonts: &Fonts, rect: Rect, msg: &str, size: f32) {
    let r = 16.0 * size / 18.0;
    rrect(rect.x, rect.y, rect.w, rect.h, r, hex(0x2c3650));
    let ty = rect.center().y;
    draw_triangle(
        vec2(rect.x + 1.0, ty - 9.0 * size / 18.0),
        vec2(rect.x + 1.0, ty + 9.0 * size / 18.0),
        vec2(rect.x - 12.0 * size / 18.0, ty + 2.0 * size / 18.0),
        hex(0x2c3650),
    );
    let lines: Vec<&str> = msg.lines().collect();
    let lh = size * 1.35;
    let y0 = ty - lh * (lines.len() as f32 - 1.0) / 2.0;
    for (i, l) in lines.iter().enumerate() {
        text(
            &fonts.ui,
            l,
            rect.x + 16.0 * size / 18.0,
            y0 + lh * i as f32,
            size,
            TEXT,
            Align::Left,
        );
    }
}
