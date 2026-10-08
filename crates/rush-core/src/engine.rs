//! Size-generic puzzle engine shared by the classic game and Bot Lab.
//!
//! A puzzle is an N×N grid (4 ≤ N ≤ 11) with up to 32 pieces of length 2–4.
//! Each piece's axis and lane never change, so a whole position is the list
//! of offsets along those axes, packed 4 bits per piece into a `u128`.
//! Occupancy is a `u128` bitboard (bit `row * N + col`).

use crate::board::{Board, Move, Orientation};

pub type State = u128;

pub const MAX_PIECES: usize = 32;
pub const MIN_SIZE: u8 = 4;
pub const MAX_SIZE: u8 = 11;

/// The fixed part of a vehicle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Piece {
    pub horizontal: bool,
    /// 2 = car, 3 = truck, 4 = bus.
    pub len: u8,
    /// Row (horizontal) or column (vertical).
    pub lane: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Puzzle {
    size: u8,
    exit_row: u8,
    pieces: Vec<Piece>,
}

impl Puzzle {
    /// `pieces[0]` must be the red car: horizontal, length 2, on `exit_row`.
    pub fn new(size: u8, exit_row: u8, pieces: Vec<Piece>) -> Result<Puzzle, String> {
        if !(MIN_SIZE..=MAX_SIZE).contains(&size) {
            return Err(format!("size must be {MIN_SIZE}..={MAX_SIZE}"));
        }
        if exit_row >= size {
            return Err("exit row outside the grid".into());
        }
        if pieces.is_empty() || pieces.len() > MAX_PIECES {
            return Err(format!("need 1..={MAX_PIECES} pieces"));
        }
        let red = pieces[0];
        if !red.horizontal || red.len != 2 || red.lane != exit_row {
            return Err("piece 0 must be the red car: horizontal, length 2, on the exit row".into());
        }
        for (i, p) in pieces.iter().enumerate() {
            if !(2..=4).contains(&p.len) || p.len > size || p.lane >= size {
                return Err(format!("piece {i} has an invalid length or lane"));
            }
        }
        Ok(Puzzle {
            size,
            exit_row,
            pieces,
        })
    }

    /// Converts a classic 6×6 board. Piece indices equal vehicle indices.
    pub fn from_board(board: &Board) -> (Puzzle, State) {
        let pieces = board
            .vehicles()
            .iter()
            .map(|v| Piece {
                horizontal: v.orientation() == Orientation::Horizontal,
                len: v.length(),
                lane: v.lane(),
            })
            .collect();
        let p = Puzzle::new(crate::SIZE, crate::EXIT_ROW, pieces).expect("a valid Board is a valid Puzzle");
        (p, Puzzle::encode(&board.positions()))
    }

    pub fn size(&self) -> u8 {
        self.size
    }
    pub fn exit_row(&self) -> u8 {
        self.exit_row
    }
    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    pub fn encode(positions: &[u8]) -> State {
        positions
            .iter()
            .enumerate()
            .fold(0, |s, (i, &p)| s | ((p as u128 & 15) << (4 * i)))
    }

    #[inline]
    pub fn pos(state: State, i: usize) -> u8 {
        ((state >> (4 * i)) & 15) as u8
    }

    pub fn positions(&self, state: State) -> Vec<u8> {
        (0..self.pieces.len()).map(|i| Self::pos(state, i)).collect()
    }

    #[inline]
    fn with_pos(state: State, i: usize, p: u8) -> State {
        (state & !(15u128 << (4 * i))) | ((p as u128) << (4 * i))
    }

    #[inline]
    fn bit(&self, i: usize, along: u8) -> u128 {
        let pc = &self.pieces[i];
        if pc.horizontal {
            1u128 << (pc.lane as u32 * self.size as u32 + along as u32)
        } else {
            1u128 << (along as u32 * self.size as u32 + pc.lane as u32)
        }
    }

    /// Bits covered by piece `i` at offset `p`.
    fn mask(&self, i: usize, p: u8) -> u128 {
        (0..self.pieces[i].len).fold(0, |m, k| m | self.bit(i, p + k))
    }

    pub fn occupancy(&self, state: State) -> u128 {
        (0..self.pieces.len()).fold(0, |occ, i| occ | self.mask(i, Self::pos(state, i)))
    }

    /// All pieces inside the grid and no overlaps.
    pub fn is_valid(&self, state: State) -> bool {
        let mut occ = 0u128;
        for i in 0..self.pieces.len() {
            let p = Self::pos(state, i);
            if p + self.pieces[i].len > self.size {
                return false;
            }
            let m = self.mask(i, p);
            if occ & m != 0 {
                return false;
            }
            occ |= m;
        }
        true
    }

    pub fn is_goal(&self, state: State) -> bool {
        Self::pos(state, 0) + 2 == self.size
    }

    /// Inclusive `(min, max)` deltas piece `i` can slide in one move.
    pub fn slide_range(&self, state: State, i: usize) -> (i8, i8) {
        if i >= self.pieces.len() {
            return (0, 0);
        }
        let occ = self.occupancy(state);
        let p = Self::pos(state, i);
        let mut lo = 0u8;
        while lo < p && occ & self.bit(i, p - lo - 1) == 0 {
            lo += 1;
        }
        let end = p + self.pieces[i].len - 1;
        let mut hi = 0u8;
        while end + hi + 1 < self.size && occ & self.bit(i, end + hi + 1) == 0 {
            hi += 1;
        }
        (-(lo as i8), hi as i8)
    }

    /// The state after `mv`, or `None` if the move is illegal.
    pub fn apply(&self, state: State, mv: Move) -> Option<State> {
        let (lo, hi) = self.slide_range(state, mv.vehicle);
        if mv.delta == 0 || !(lo..=hi).contains(&mv.delta) {
            return None;
        }
        let p = Self::pos(state, mv.vehicle) as i8 + mv.delta;
        Some(Self::with_pos(state, mv.vehicle, p as u8))
    }

    /// Calls `f(next, move)` for every legal single slide.
    #[inline]
    pub fn for_each_neighbor(&self, state: State, mut f: impl FnMut(State, Move)) {
        let occ = self.occupancy(state);
        for i in 0..self.pieces.len() {
            let p = Self::pos(state, i);
            let mut d = 1u8;
            while d <= p && occ & self.bit(i, p - d) == 0 {
                f(
                    Self::with_pos(state, i, p - d),
                    Move {
                        vehicle: i,
                        delta: -(d as i8),
                    },
                );
                d += 1;
            }
            let end = p + self.pieces[i].len - 1;
            let mut d = 1u8;
            while end + d < self.size && occ & self.bit(i, end + d) == 0 {
                f(
                    Self::with_pos(state, i, p + d),
                    Move {
                        vehicle: i,
                        delta: d as i8,
                    },
                );
                d += 1;
            }
        }
    }

    /// Admissible, consistent lower bound on the moves left:
    /// 0 when solved, else 1 (the red car's final slide) + the number of
    /// distinct pieces between the red car and the exit (each must move).
    pub fn heuristic(&self, state: State) -> u32 {
        if self.is_goal(state) {
            return 0;
        }
        let front = Self::pos(state, 0) + 2; // first cell right of the red car
        let mut path = 0u128;
        for c in front..self.size {
            path |= 1u128 << (self.exit_row as u32 * self.size as u32 + c as u32);
        }
        let blockers = (1..self.pieces.len())
            .filter(|&i| self.mask(i, Self::pos(state, i)) & path != 0)
            .count() as u32;
        1 + blockers
    }
}
