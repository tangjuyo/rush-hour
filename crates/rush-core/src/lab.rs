//! Seeded random grids for Bot Lab (feature 002).
//!
//! The generator places random vehicles around a red car that is already
//! *at the exit*, explores the positions reachable from there, and returns
//! one of the farthest from any solved position. Every slide is reversible,
//! so the result is always solvable; the same (size, seed) always gives the
//! same grid.

use crate::engine::{Piece, Puzzle, State, MAX_PIECES};
use crate::generator::Rng;
use crate::search::{best_first, Limits, Observer, Outcome, Strategy};
use crate::solver::FxMap;

/// Exit row for an N×N grid: row 3 (index 2) on the classic 6×6.
pub fn exit_row(size: u8) -> u8 {
    (size - 1) / 2
}

fn random_len(rng: &mut Rng, size: u8) -> u8 {
    let roll = rng.below(100);
    if size >= 7 {
        match roll {
            0..=47 => 2,
            48..=79 => 3,
            _ => 4,
        }
    } else if roll < 72 {
        2
    } else {
        3
    }
}

/// Generates a solvable `size`×`size` grid (6 ≤ size ≤ 10 recommended).
pub fn generate(size: u8, seed: u64) -> (Puzzle, State) {
    let mut attempt = 0u64;
    loop {
        let mut rng = Rng::new(seed ^ ((size as u64) << 48) ^ attempt.wrapping_mul(0x9E37_79B9));
        let found = climb(&mut rng, size);
        if let Some(found) = found {
            return found;
        }
        attempt += 1;
    }
}

/// A vehicle layout with every piece at its position in the *solved* state.
#[derive(Clone)]
struct Layout {
    pieces: Vec<Piece>,
    positions: Vec<u8>,
}

impl Layout {
    fn cells(size: u8, p: Piece, pos: u8) -> impl Iterator<Item = usize> {
        let n = size as usize;
        (0..p.len).map(move |k| {
            if p.horizontal {
                p.lane as usize * n + (pos + k) as usize
            } else {
                (pos + k) as usize * n + p.lane as usize
            }
        })
    }

    fn grid(&self, size: u8) -> Vec<bool> {
        let mut g = vec![false; size as usize * size as usize];
        for (p, &pos) in self.pieces.iter().zip(&self.positions) {
            for c in Self::cells(size, *p, pos) {
                g[c] = true;
            }
        }
        g
    }

    /// Adds one random vehicle in a free spot (not horizontal on the exit row).
    fn add(&mut self, rng: &mut Rng, size: u8) -> bool {
        if self.pieces.len() >= MAX_PIECES {
            return false;
        }
        let grid = self.grid(size);
        let n = size as usize;
        for _ in 0..40 {
            let len = random_len(rng, size);
            let horizontal = rng.below(2) == 0;
            let lane = rng.below(n) as u8;
            if horizontal && lane == exit_row(size) {
                continue;
            }
            let pos = rng.below(n - len as usize + 1) as u8;
            let piece = Piece {
                horizontal,
                len,
                lane,
            };
            if Self::cells(size, piece, pos).all(|c| !grid[c]) {
                self.pieces.push(piece);
                self.positions.push(pos);
                return true;
            }
        }
        false
    }

    fn remove(&mut self, rng: &mut Rng) -> bool {
        if self.pieces.len() <= 2 {
            return false;
        }
        let i = 1 + rng.below(self.pieces.len() - 1);
        self.pieces.remove(i);
        self.positions.remove(i);
        true
    }

    fn filled(&self) -> usize {
        self.pieces.iter().map(|p| p.len as usize).sum()
    }
}

/// Explores the cluster of a layout and returns its farthest position + par.
fn score(layout: &Layout, size: u8, limit: usize, rng: &mut Rng) -> Option<(Puzzle, State, u32)> {
    let puzzle = Puzzle::new(size, exit_row(size), layout.pieces.clone()).ok()?;
    let solved = Puzzle::encode(&layout.positions);
    let (state, d) = farthest(&puzzle, solved, limit, rng)?;
    Some((puzzle, state, d))
}

/// Hill-climbs on the layout (add / remove / swap a vehicle), keeping the
/// layout whose farthest position is deepest.
fn climb(rng: &mut Rng, size: u8) -> Option<(Puzzle, State)> {
    let n = size as usize;
    let er = exit_row(size);
    let mut layout = Layout {
        pieces: vec![Piece {
            horizontal: true,
            len: 2,
            lane: er,
        }],
        positions: vec![size - 2],
    };
    // Denser on small grids; big grids gridlock when over-filled.
    let base = 84 - 4 * (n.saturating_sub(6));
    let target = n * n * (base - 10 + rng.below(11)) / 100;
    for _ in 0..400 {
        if layout.filled() >= target || !layout.add(rng, size) {
            if layout.filled() >= target {
                break;
            }
        }
    }
    let (limit, steps) = climb_budget(size);
    let mut best = score(&layout, size, limit, rng)?;
    for _ in 0..steps {
        let mut next = layout.clone();
        let changed = match rng.below(3) {
            0 => next.add(rng, size),
            1 => next.remove(rng),
            _ => next.remove(rng) && next.add(rng, size),
        };
        if !changed {
            continue;
        }
        if let Some(candidate) = score(&next, size, limit, rng) {
            if candidate.2 >= best.2 {
                layout = next;
                best = candidate;
            }
        }
    }
    // Final, deeper look at the winning layout.
    let (puzzle, state, _) = score(&layout, size, EXPLORE_LIMIT, rng).unwrap_or(best);
    // Then push the start position itself further from the exit.
    let state = deepen(&puzzle, state, rng, DEEPEN_STEPS);
    (puzzle.heuristic(state) >= 2 && puzzle.pieces().len() >= 6).then_some((puzzle, state))
}

/// Counts expansions so the whole deepening stage has a fixed work budget.
struct Budget {
    left: u64,
}
impl Observer for Budget {
    fn expanded(&mut self, _s: State, _d: u32, _n: u64) -> bool {
        if self.left == 0 {
            return false;
        }
        self.left -= 1;
        true
    }
}

/// Exact minimum number of moves, if A* finds it within the budget.
fn exact_par(p: &Puzzle, s: State, budget: &mut Budget) -> Option<u32> {
    match best_first(
        p,
        s,
        Strategy::AStar,
        Limits {
            max_states: DEEPEN_CAP,
        },
        None,
        budget,
    ) {
        Outcome::Solved(path) => Some(path.len() as u32),
        _ => None,
    }
}

/// Pieces in the blocking chain: those on the red car's path, the pieces
/// blocking *their* escape, and so on (a few levels deep).
fn chain(p: &Puzzle, s: State) -> Vec<usize> {
    let n = p.size() as u32;
    let occ_owner = |cell: u32| -> Option<usize> {
        (0..p.pieces().len()).find(|&i| {
            let pc = p.pieces()[i];
            let pos = Puzzle::pos(s, i) as u32;
            (0..pc.len as u32).any(|k| {
                let c = if pc.horizontal {
                    pc.lane as u32 * n + pos + k
                } else {
                    (pos + k) * n + pc.lane as u32
                };
                c == cell
            })
        })
    };
    let mut set = vec![0usize];
    let front = Puzzle::pos(s, 0) as u32 + 2;
    let mut frontier: Vec<usize> = (front..n)
        .filter_map(|c| occ_owner(p.exit_row() as u32 * n + c))
        .filter(|&i| i != 0)
        .collect();
    for _ in 0..4 {
        let mut next = Vec::new();
        for i in frontier {
            if set.contains(&i) {
                continue;
            }
            set.push(i);
            // Cells adjacent to both ends along its axis (where it must go).
            let pc = p.pieces()[i];
            let pos = Puzzle::pos(s, i) as i32;
            for along in [pos - 1, pos + pc.len as i32] {
                if along < 0 || along >= n as i32 {
                    continue;
                }
                let cell = if pc.horizontal {
                    pc.lane as u32 * n + along as u32
                } else {
                    along as u32 * n + pc.lane as u32
                };
                if let Some(j) = occ_owner(cell) {
                    next.push(j);
                }
            }
        }
        frontier = next;
    }
    set
}

/// Hill-climbs on the start position: tries slides of blocking-chain pieces
/// and keeps the one whose exact par is highest (ties allowed).
fn deepen(p: &Puzzle, mut state: State, rng: &mut Rng, steps: usize) -> State {
    let mut budget = Budget { left: DEEPEN_WORK };
    let Some(mut par) = exact_par(p, state, &mut budget) else {
        return state;
    };
    let mut options: Vec<State> = Vec::with_capacity(64);
    for _ in 0..steps {
        if budget.left == 0 {
            break;
        }
        let relevant = chain(p, state);
        options.clear();
        p.for_each_neighbor(state, |n, mv| {
            if relevant.contains(&mv.vehicle) {
                options.push(n);
            }
        });
        if options.is_empty() {
            break;
        }
        let mut best: Option<(State, u32)> = None;
        for _ in 0..DEEPEN_TRIES.min(options.len()) {
            let n = options.swap_remove(rng.below(options.len()));
            if let Some(d) = exact_par(p, n, &mut budget) {
                if d >= par && best.is_none_or(|(_, bd)| d > bd) {
                    best = Some((n, d));
                }
            }
        }
        if let Some((n, d)) = best {
            state = n;
            par = d;
        }
    }
    state
}

/// Start-position hill-climb: steps, candidate slides per step, A* budget
/// per evaluation, and total expansions for the whole stage.
const DEEPEN_STEPS: usize = 60;
const DEEPEN_TRIES: usize = 8;
const DEEPEN_CAP: usize = 40_000;
const DEEPEN_WORK: u64 = 1_500_000;

/// (exploration budget per hill-climb step, number of steps) by grid size:
/// bigger grids need a wider look per step to see deep positions at all.
fn climb_budget(size: u8) -> (usize, usize) {
    match size {
        0..=6 => (25_000, 150),
        7..=8 => (40_000, 100),
        _ => (120_000, 45),
    }
}

/// How many positions the generator explores around the solved one.
pub const EXPLORE_LIMIT: usize = 150_000;

/// Explores up to `limit` *unsolved* positions reachable from `solved`
/// (other solved positions are recorded but not expanded, so the budget is
/// not wasted on traffic shuffling behind a red car that is already out),
/// computes each one's distance to the nearest solved position and returns
/// one of the farthest fully-expanded positions with its distance.
fn farthest(p: &Puzzle, solved: State, limit: usize, rng: &mut Rng) -> Option<(State, u32)> {
    let mut index: FxMap<State, u32> = FxMap::default();
    let mut states = vec![solved];
    index.insert(solved, 0);
    let mut adj_start = vec![0u32];
    let mut adj: Vec<u32> = Vec::new();
    let mut k = 0;
    while k < states.len() && states.len() < limit {
        let s = states[k];
        if k == 0 || !p.is_goal(s) {
            p.for_each_neighbor(s, |n, _| {
                let next_id = states.len() as u32;
                let id = *index.entry(n).or_insert_with(|| {
                    states.push(n);
                    next_id
                });
                adj.push(id);
            });
        }
        adj_start.push(adj.len() as u32);
        k += 1;
    }
    let expanded = k; // states[..expanded] have complete adjacency
    let mut dist = vec![u32::MAX; states.len()];
    let mut queue = std::collections::VecDeque::new();
    for (i, &s) in states.iter().enumerate() {
        if p.is_goal(s) {
            dist[i] = 0;
        }
    }
    // Seeds: unsolved expanded positions with a solved neighbour.
    for i in 0..expanded {
        if dist[i] == 0 {
            continue;
        }
        let nb = &adj[adj_start[i] as usize..adj_start[i + 1] as usize];
        if nb.iter().any(|&j| dist[j as usize] == 0) {
            dist[i] = 1;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        if i >= expanded {
            continue;
        }
        for &j in &adj[adj_start[i] as usize..adj_start[i + 1] as usize] {
            if dist[j as usize] == u32::MAX {
                dist[j as usize] = dist[i] + 1;
                queue.push_back(j as usize);
            }
        }
    }
    let best = (0..expanded)
        .filter(|&i| dist[i] != u32::MAX)
        .map(|i| dist[i])
        .max()?;
    // Among the hardest, prefer the red car as far from the exit as possible.
    let far = (0..expanded)
        .filter(|&i| dist[i] == best)
        .map(|i| Puzzle::pos(states[i], 0))
        .min()?;
    let candidates: Vec<usize> = (0..expanded)
        .filter(|&i| dist[i] == best && Puzzle::pos(states[i], 0) == far)
        .collect();
    Some((states[candidates[rng.below(candidates.len())]], best))
}
