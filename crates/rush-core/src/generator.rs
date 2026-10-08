//! Original puzzle generator (Constitution IV, research R3).
//!
//! Hill-climbs on vehicle *layouts* (which vehicles, in which lanes),
//! scoring each layout by the hardest position reachable in its cluster.

use crate::board::{Board, Orientation, Vehicle, CAR_IDS, EXIT_ROW, SIZE, TRUCK_IDS};
use crate::solver::Cluster;

/// xorshift64* — small deterministic RNG so generation is reproducible
/// without external crates.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }
}

/// A board with only the red car at a random column.
pub fn empty_board(rng: &mut Rng) -> Board {
    let col = rng.below(4) as u8;
    Board::new(vec![
        Vehicle::new('A', Orientation::Horizontal, EXIT_ROW, col).unwrap()
    ])
    .expect("single red car is valid")
}

/// Tries to add one random unused vehicle at a random free place.
pub fn add_random(rng: &mut Rng, board: &Board) -> Option<Board> {
    let used: Vec<char> = board.vehicles().iter().map(|v| v.id()).collect();
    let free_cars: Vec<char> = CAR_IDS.iter().copied().filter(|c| !used.contains(c)).collect();
    let free_trucks: Vec<char> = TRUCK_IDS.iter().copied().filter(|c| !used.contains(c)).collect();
    let id = match (free_cars.is_empty(), free_trucks.is_empty()) {
        (true, true) => return None,
        (false, true) => free_cars[rng.below(free_cars.len())],
        (true, false) => free_trucks[rng.below(free_trucks.len())],
        (false, false) => {
            if rng.chance(30) {
                free_trucks[rng.below(free_trucks.len())]
            } else {
                free_cars[rng.below(free_cars.len())]
            }
        }
    };
    for _ in 0..20 {
        let horizontal = rng.chance(50);
        let len = if TRUCK_IDS.contains(&id) { 3 } else { 2 };
        let lane = rng.below(SIZE as usize) as u8;
        if horizontal && lane == EXIT_ROW {
            continue; // would sit on the exit row: useless or unsolvable
        }
        let pos = rng.below((SIZE - len + 1) as usize) as u8;
        let (o, r, c) = if horizontal {
            (Orientation::Horizontal, lane, pos)
        } else {
            (Orientation::Vertical, pos, lane)
        };
        let mut vs = board.vehicles().to_vec();
        vs.push(Vehicle::new(id, o, r, c).unwrap());
        if let Ok(b) = Board::new(vs) {
            return Some(b);
        }
    }
    None
}

/// Removes a random non-target vehicle.
pub fn remove_random(rng: &mut Rng, board: &Board) -> Option<Board> {
    let n = board.vehicles().len();
    if n < 2 {
        return None;
    }
    let victim = 1 + rng.below(n - 1);
    let vs = board
        .vehicles()
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != victim)
        .map(|(_, v)| *v)
        .collect();
    Board::new(vs).ok()
}

/// One random layout mutation.
pub fn mutate(rng: &mut Rng, board: &Board) -> Option<Board> {
    match rng.below(4) {
        0 | 1 => add_random(rng, board),
        2 => remove_random(rng, board),
        _ => remove_random(rng, board).and_then(|b| add_random(rng, &b)),
    }
}

/// A scored layout found during a climb.
#[derive(Clone, Debug)]
pub struct Candidate {
    /// Hardest position of the cluster.
    pub board: Board,
    /// Its par.
    pub par: usize,
}

/// Runs one hill-climb of `steps` mutations, returning every improvement
/// (so callers get a spread of difficulties, not only the final best).
pub fn climb(rng: &mut Rng, steps: usize) -> Vec<Candidate> {
    let mut cur = empty_board(rng);
    for _ in 0..(4 + rng.below(6)) {
        if let Some(b) = add_random(rng, &cur) {
            cur = b;
        }
    }
    let score = |b: &Board| Cluster::explore(b).hardest();
    let mut best = score(&cur);
    let mut found: Vec<Candidate> = Vec::new();
    let mut cur_par = best.as_ref().map_or(0, |(_, d)| *d);
    for _ in 0..steps {
        let Some(next) = mutate(rng, &cur) else { continue };
        let Some((hard, d)) = score(&next) else { continue };
        if d >= cur_par {
            if d > cur_par {
                found.push(Candidate {
                    board: hard.clone(),
                    par: d,
                });
            }
            cur = next;
            cur_par = d;
            best = Some((hard, d));
        }
    }
    if let Some((board, par)) = best {
        found.push(Candidate { board, par });
    }
    found
}
