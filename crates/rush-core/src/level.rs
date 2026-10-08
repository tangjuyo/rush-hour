//! Level text format (see `specs/001-rush-hour-game/contracts/level-format.md`),
//! difficulty tiers, challenges and star rating.

use crate::board::{Board, Kind, Orientation, Vehicle, SIZE};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tier {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

impl Tier {
    pub const ALL: [Tier; 4] = [Tier::Beginner, Tier::Intermediate, Tier::Advanced, Tier::Expert];

    pub fn name(self) -> &'static str {
        match self {
            Tier::Beginner => "Beginner",
            Tier::Intermediate => "Intermediate",
            Tier::Advanced => "Advanced",
            Tier::Expert => "Expert",
        }
    }

    pub fn from_name(s: &str) -> Option<Tier> {
        Tier::ALL.into_iter().find(|t| t.name().eq_ignore_ascii_case(s))
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One of the 40 shipped puzzles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Challenge {
    /// 1..=40
    pub number: u32,
    pub tier: Tier,
    pub board: Board,
    /// Minimum number of moves (proven by the solver).
    pub par: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ParseError {}

fn err<T>(msg: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError(msg.into()))
}

/// Parses 6 lines of 6 characters (`.` = empty, letters = vehicles).
/// Orientation is inferred from the shape; length must match the id's kind.
pub fn parse_board(text: &str) -> Result<Board, ParseError> {
    let rows: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if rows.len() != SIZE as usize {
        return err(format!("expected {SIZE} rows, found {}", rows.len()));
    }
    // id -> list of (row, col)
    let mut cells: Vec<(char, Vec<(u8, u8)>)> = Vec::new();
    for (r, line) in rows.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        if chars.len() != SIZE as usize {
            return err(format!("row {} must have {SIZE} cells", r + 1));
        }
        for (c, ch) in chars.into_iter().enumerate() {
            if ch == '.' {
                continue;
            }
            let pos = (r as u8, c as u8);
            match cells.iter_mut().find(|(id, _)| *id == ch) {
                Some((_, list)) => list.push(pos),
                None => cells.push((ch, vec![pos])),
            }
        }
    }
    let mut vehicles = Vec::new();
    for (id, list) in cells {
        let Some(kind) = Kind::of_id(id) else {
            return err(format!("unknown vehicle '{id}'"));
        };
        if list.len() != kind.length() as usize {
            return err(format!(
                "vehicle '{id}' covers {} cells but a {:?} is {} long",
                list.len(),
                kind,
                kind.length()
            ));
        }
        let (r0, c0) = list[0];
        let horizontal = list
            .iter()
            .enumerate()
            .all(|(k, &(r, c))| r == r0 && c == c0 + k as u8);
        let vertical = list
            .iter()
            .enumerate()
            .all(|(k, &(r, c))| c == c0 && r == r0 + k as u8);
        let orientation = match (horizontal, vertical) {
            (true, _) => Orientation::Horizontal,
            (_, true) => Orientation::Vertical,
            _ => return err(format!("vehicle '{id}' is not a straight line")),
        };
        vehicles.push(Vehicle::new(id, orientation, r0, c0).expect("id checked"));
    }
    Board::new(vehicles).map_err(|e| ParseError(e.to_string()))
}

/// Parses a whole levels file.
pub fn parse_levels(text: &str) -> Result<Vec<Challenge>, ParseError> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.is_empty() || line.starts_with('#') {
            i += 1;
            continue;
        }
        let Some(header) = line.strip_prefix('@') else {
            return err(format!("line {}: expected '@' header", i + 1));
        };
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() != 3 {
            return err(format!("line {}: header is '@ <n> <Tier> par=<p>'", i + 1));
        }
        let number: u32 = parts[0]
            .parse()
            .map_err(|_| ParseError(format!("line {}: bad number", i + 1)))?;
        let tier = Tier::from_name(parts[1])
            .ok_or_else(|| ParseError(format!("line {}: unknown tier '{}'", i + 1, parts[1])))?;
        let par: u32 = parts[2]
            .strip_prefix("par=")
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| ParseError(format!("line {}: bad par", i + 1)))?;
        let rows = SIZE as usize;
        if lines.len() < i + 1 + rows {
            return err(format!("challenge {number}: board truncated"));
        }
        let board = parse_board(&lines[i + 1..i + 1 + rows].join("\n"))
            .map_err(|e| ParseError(format!("challenge {number}: {e}")))?;
        out.push(Challenge {
            number,
            tier,
            board,
            par,
        });
        i += 1 + rows;
    }
    Ok(out)
}

/// Serialises challenges in the levels file format.
pub fn write_levels(levels: &[Challenge]) -> String {
    let mut s = String::new();
    for c in levels {
        s.push_str(&format!(
            "@ {} {} par={}\n{}\n\n",
            c.number, c.tier, c.par, c.board
        ));
    }
    s
}

const BUILTIN: &str = include_str!("../assets/levels.txt");

/// The 40 shipped challenges (embedded at compile time).
pub fn builtin() -> Vec<Challenge> {
    parse_levels(BUILTIN).expect("embedded levels.txt is valid (checked by tests/levels.rs)")
}

/// Star rating: 3 at par (or better), 2 within par + ceil(par/2), else 1.
pub fn stars(moves: u32, par: u32) -> u8 {
    if moves <= par {
        3
    } else if moves <= par + par.div_ceil(2) {
        2
    } else {
        1
    }
}
