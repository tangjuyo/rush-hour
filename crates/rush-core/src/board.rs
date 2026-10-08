//! Board model and the official Rush Hour movement rules (Constitution III).

use std::fmt;

/// The grid is `SIZE × SIZE`.
pub const SIZE: u8 = 6;
/// Row (0-based) whose right edge holds the exit.
pub const EXIT_ROW: u8 = 2;
/// Identifier of the red target car.
pub const TARGET_ID: char = 'A';

/// Ids usable for cars (2 cells): the red car `A` plus 11 others = 12 cars.
pub const CAR_IDS: [char; 12] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L'];
/// Ids usable for trucks (3 cells): 4 trucks.
pub const TRUCK_IDS: [char; 4] = ['O', 'P', 'Q', 'R'];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Car,
    Truck,
}

impl Kind {
    /// Number of cells occupied.
    pub fn length(self) -> u8 {
        match self {
            Kind::Car => 2,
            Kind::Truck => 3,
        }
    }

    /// Kind implied by a vehicle id, or `None` for an unknown id.
    pub fn of_id(id: char) -> Option<Kind> {
        if CAR_IDS.contains(&id) {
            Some(Kind::Car)
        } else if TRUCK_IDS.contains(&id) {
            Some(Kind::Truck)
        } else {
            None
        }
    }
}

/// A car or truck. Its axis (`orientation` + `fixed` lane) never changes;
/// only `pos` (offset along its axis) does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Vehicle {
    id: char,
    orientation: Orientation,
    /// Row for horizontal vehicles, column for vertical ones.
    fixed: u8,
    /// Column (horizontal) or row (vertical) of the top-left cell.
    pos: u8,
}

impl Vehicle {
    /// Creates a vehicle whose top-left cell is at (`row`, `col`).
    /// Returns `None` if the id is unknown. Bounds are checked by [`Board::new`].
    pub fn new(id: char, orientation: Orientation, row: u8, col: u8) -> Option<Vehicle> {
        Kind::of_id(id)?;
        let (fixed, pos) = match orientation {
            Orientation::Horizontal => (row, col),
            Orientation::Vertical => (col, row),
        };
        Some(Vehicle {
            id,
            orientation,
            fixed,
            pos,
        })
    }

    pub fn id(&self) -> char {
        self.id
    }
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }
    pub fn kind(&self) -> Kind {
        Kind::of_id(self.id).expect("validated at construction")
    }
    pub fn length(&self) -> u8 {
        self.kind().length()
    }
    pub fn is_target(&self) -> bool {
        self.id == TARGET_ID
    }
    /// Lane index: row if horizontal, column if vertical.
    pub fn lane(&self) -> u8 {
        self.fixed
    }
    /// Offset along the axis.
    pub fn pos(&self) -> u8 {
        self.pos
    }
    pub fn row(&self) -> u8 {
        match self.orientation {
            Orientation::Horizontal => self.fixed,
            Orientation::Vertical => self.pos,
        }
    }
    pub fn col(&self) -> u8 {
        match self.orientation {
            Orientation::Horizontal => self.pos,
            Orientation::Vertical => self.fixed,
        }
    }
    /// The (row, col) cells covered.
    pub fn cells(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        (0..self.length()).map(move |k| match self.orientation {
            Orientation::Horizontal => (self.fixed, self.pos + k),
            Orientation::Vertical => (self.pos + k, self.fixed),
        })
    }
    pub(crate) fn with_pos(mut self, pos: u8) -> Vehicle {
        self.pos = pos;
        self
    }
}

/// One player move: slide `vehicle` (index into [`Board::vehicles`]) by
/// `delta` cells (negative = left/up). Any distance counts as one move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub vehicle: usize,
    pub delta: i8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoardError {
    UnknownId(char),
    DuplicateId(char),
    OutOfBounds(char),
    Overlap(char, char),
    MissingTarget,
    /// The red car must be horizontal on the exit row.
    BadTarget,
}

impl fmt::Display for BoardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoardError::UnknownId(c) => write!(f, "unknown vehicle id '{c}'"),
            BoardError::DuplicateId(c) => write!(f, "duplicate vehicle id '{c}'"),
            BoardError::OutOfBounds(c) => write!(f, "vehicle '{c}' is outside the grid"),
            BoardError::Overlap(a, b) => write!(f, "vehicles '{a}' and '{b}' overlap"),
            BoardError::MissingTarget => write!(f, "the red car 'A' is missing"),
            BoardError::BadTarget => write!(f, "the red car must be horizontal on row 3"),
        }
    }
}
impl std::error::Error for BoardError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoveError {
    NoSuchVehicle,
    ZeroDistance,
    Blocked,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MoveError::NoSuchVehicle => "no such vehicle",
            MoveError::ZeroDistance => "a move must change the vehicle's position",
            MoveError::Blocked => "path blocked or outside the grid",
        })
    }
}
impl std::error::Error for MoveError {}

/// A valid 6×6 position. `vehicles()[0]` is always the red car; the others
/// follow in id order, so indices are stable for the lifetime of a game.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Board {
    vehicles: Vec<Vehicle>,
}

impl Board {
    pub fn new(mut vehicles: Vec<Vehicle>) -> Result<Board, BoardError> {
        vehicles.sort_by_key(|v| (!v.is_target(), v.id));
        let mut grid = [[None::<char>; SIZE as usize]; SIZE as usize];
        for (i, v) in vehicles.iter().enumerate() {
            if Kind::of_id(v.id).is_none() {
                return Err(BoardError::UnknownId(v.id));
            }
            if i > 0 && vehicles[i - 1].id == v.id {
                return Err(BoardError::DuplicateId(v.id));
            }
            if v.fixed >= SIZE || v.pos + v.length() > SIZE {
                return Err(BoardError::OutOfBounds(v.id));
            }
            for (r, c) in v.cells() {
                if let Some(other) = grid[r as usize][c as usize] {
                    return Err(BoardError::Overlap(other, v.id));
                }
                grid[r as usize][c as usize] = Some(v.id);
            }
        }
        match vehicles.first() {
            Some(t) if t.is_target() => {
                if t.orientation != Orientation::Horizontal || t.fixed != EXIT_ROW {
                    return Err(BoardError::BadTarget);
                }
            }
            _ => return Err(BoardError::MissingTarget),
        }
        Ok(Board { vehicles })
    }

    pub fn vehicles(&self) -> &[Vehicle] {
        &self.vehicles
    }

    /// The red car.
    pub fn target(&self) -> &Vehicle {
        &self.vehicles[0]
    }

    /// Index of the vehicle covering (`row`, `col`), if any.
    pub fn vehicle_at(&self, row: u8, col: u8) -> Option<usize> {
        self.vehicles
            .iter()
            .position(|v| v.cells().any(|cell| cell == (row, col)))
    }

    fn occupied(&self) -> u64 {
        let mut bits = 0u64;
        for v in &self.vehicles {
            for (r, c) in v.cells() {
                bits |= 1 << (r * SIZE + c);
            }
        }
        bits
    }

    /// Inclusive range `(min, max)` of deltas vehicle `i` can slide in a
    /// single move: `min <= 0 <= max`. Stops at walls and other vehicles.
    pub fn slide_range(&self, i: usize) -> (i8, i8) {
        let Some(v) = self.vehicles.get(i) else {
            return (0, 0);
        };
        let occ = self.occupied();
        let cell = |p: u8| -> u64 {
            match v.orientation {
                Orientation::Horizontal => 1 << (v.fixed * SIZE + p),
                Orientation::Vertical => 1 << (p * SIZE + v.fixed),
            }
        };
        let mut lo = 0i8;
        while v.pos as i8 + lo > 0 && occ & cell((v.pos as i8 + lo - 1) as u8) == 0 {
            lo -= 1;
        }
        let mut hi = 0i8;
        let end = v.pos + v.length() - 1;
        while (end as i8 + hi) < SIZE as i8 - 1 && occ & cell((end as i8 + hi + 1) as u8) == 0 {
            hi += 1;
        }
        (lo, hi)
    }

    pub fn is_legal(&self, mv: Move) -> bool {
        if mv.vehicle >= self.vehicles.len() || mv.delta == 0 {
            return false;
        }
        let (lo, hi) = self.slide_range(mv.vehicle);
        (lo..=hi).contains(&mv.delta)
    }

    /// Applies a move in place.
    pub fn apply(&mut self, mv: Move) -> Result<(), MoveError> {
        if mv.vehicle >= self.vehicles.len() {
            return Err(MoveError::NoSuchVehicle);
        }
        if mv.delta == 0 {
            return Err(MoveError::ZeroDistance);
        }
        if !self.is_legal(mv) {
            return Err(MoveError::Blocked);
        }
        let v = &mut self.vehicles[mv.vehicle];
        v.pos = (v.pos as i8 + mv.delta) as u8;
        Ok(())
    }

    /// Returns a new board with the move applied.
    pub fn applied(&self, mv: Move) -> Result<Board, MoveError> {
        let mut b = self.clone();
        b.apply(mv)?;
        Ok(b)
    }

    /// Every legal single move from this position.
    pub fn legal_moves(&self) -> Vec<Move> {
        let mut out = Vec::new();
        for i in 0..self.vehicles.len() {
            let (lo, hi) = self.slide_range(i);
            out.extend(
                (lo..=hi)
                    .filter(|&d| d != 0)
                    .map(|delta| Move { vehicle: i, delta }),
            );
        }
        out
    }

    /// Won: the red car's front reaches the exit (right edge).
    pub fn is_solved(&self) -> bool {
        let t = self.target();
        t.pos + t.length() == SIZE
    }

    /// Positions of every vehicle, in index order.
    pub fn positions(&self) -> Vec<u8> {
        self.vehicles.iter().map(|v| v.pos).collect()
    }

    /// Same vehicles at new positions. Caller guarantees validity.
    pub(crate) fn with_positions(&self, positions: impl Iterator<Item = u8>) -> Board {
        Board {
            vehicles: self
                .vehicles
                .iter()
                .zip(positions)
                .map(|(v, p)| v.with_pos(p))
                .collect(),
        }
    }
}

impl fmt::Display for Board {
    /// 6 lines of 6 chars, `.` for empty cells (the level text format).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut grid = [['.'; SIZE as usize]; SIZE as usize];
        for v in &self.vehicles {
            for (r, c) in v.cells() {
                grid[r as usize][c as usize] = v.id;
            }
        }
        let lines: Vec<String> = grid.iter().map(|row| row.iter().collect()).collect();
        f.write_str(&lines.join("\n"))
    }
}
