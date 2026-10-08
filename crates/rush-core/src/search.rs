//! Anytime best-first search for Bot Lab (feature 002).
//!
//! One engine, three strategies:
//! * `Greedy`      : priority `h`. Finds *a* solution fast, usually long.
//! * `Weighted(w)` : priority `g + w·h`. A good solution, not guaranteed optimal.
//! * `AStar`       : priority `g + h`. Optimal, because `h` is admissible and consistent.
//!
//! [`anytime`] chains them. Each later phase only looks for solutions shorter
//! than the best so far (branch & bound), and if A* exhausts that space the
//! current best is **proven optimal**. The core never reads a clock: the
//! [`Observer`] decides when to stop.

use crate::board::Move;
use crate::engine::{Puzzle, State};
use crate::solver::FxMap;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Strategy {
    Greedy,
    /// Integer heuristic weight (≥ 2).
    Weighted(u32),
    AStar,
}

impl Strategy {
    pub fn name(self) -> &'static str {
        match self {
            Strategy::Greedy => "Greedy dash",
            Strategy::Weighted(_) => "Weighted A*",
            Strategy::AStar => "A* (optimal)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Stop with [`Outcome::MemoryLimit`] beyond this many stored positions.
    pub max_states: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_states: 4_000_000,
        }
    }
}

/// Receives progress. Every method has a no-op default.
pub trait Observer {
    /// Called for every expanded position. Return `false` to stop.
    fn expanded(&mut self, _state: State, _depth: u32, _expanded: u64) -> bool {
        true
    }
    /// A new phase starts.
    fn phase(&mut self, _strategy: Strategy) {}
    /// A strictly better solution was found.
    fn solution(&mut self, _strategy: Strategy, _path: &[Move], _optimal: bool) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Solved(Vec<Move>),
    /// The whole (bounded) space was searched without a solution.
    Exhausted,
    Stopped,
    MemoryLimit,
}

struct Node {
    parent: State,
    mv: Move,
    g: u32,
    closed: bool,
}

fn priority(strategy: Strategy, g: u32, h: u32) -> u64 {
    match strategy {
        Strategy::Greedy => ((h as u64) << 20) | g as u64,
        Strategy::Weighted(w) => (g + w * h) as u64,
        Strategy::AStar => (g + h) as u64,
    }
}

/// Best-first search. With `bound = Some(b)`, only solutions shorter than `b`
/// are searched for.
pub fn best_first(
    p: &Puzzle,
    start: State,
    strategy: Strategy,
    limits: Limits,
    bound: Option<usize>,
    obs: &mut dyn Observer,
) -> Outcome {
    if p.is_goal(start) {
        return Outcome::Solved(Vec::new());
    }
    let bound = bound.map_or(u32::MAX, |b| b as u32);
    let mut nodes: FxMap<State, Node> = FxMap::default();
    let dummy = Move { vehicle: 0, delta: 0 };
    nodes.insert(
        start,
        Node {
            parent: start,
            mv: dummy,
            g: 0,
            closed: false,
        },
    );
    // (priority, h, g, state). Min-heap via Reverse; ties prefer lower h.
    let mut heap = BinaryHeap::new();
    let h0 = p.heuristic(start);
    heap.push(Reverse((priority(strategy, 0, h0), h0, 0u32, start)));
    let mut expanded = 0u64;
    let mut children: Vec<(State, Move)> = Vec::with_capacity(64);

    while let Some(Reverse((_, _, g, s))) = heap.pop() {
        let node = nodes.get_mut(&s).expect("queued nodes are stored");
        if node.g != g || node.closed {
            continue; // stale entry
        }
        node.closed = true;
        expanded += 1;
        if !obs.expanded(s, g, expanded) {
            return Outcome::Stopped;
        }
        children.clear();
        p.for_each_neighbor(s, |n, mv| children.push((n, mv)));
        let cg = g + 1;
        for &(n, mv) in &children {
            let h = p.heuristic(n);
            if cg + h >= bound {
                continue; // cannot beat the best known solution
            }
            match nodes.get_mut(&n) {
                Some(existing) => {
                    // Reopen only for A* would never trigger (consistent h);
                    // for the other strategies we keep the first path found.
                    if cg >= existing.g || existing.closed {
                        continue;
                    }
                    existing.g = cg;
                    existing.parent = s;
                    existing.mv = mv;
                }
                None => {
                    nodes.insert(
                        n,
                        Node {
                            parent: s,
                            mv,
                            g: cg,
                            closed: false,
                        },
                    );
                }
            }
            if p.is_goal(n) {
                // Safe for A*: f(parent) >= g(parent) + 1 = cg, the open minimum.
                return Outcome::Solved(path_to(&nodes, start, n));
            }
            heap.push(Reverse((priority(strategy, cg, h), h, cg, n)));
        }
        if nodes.len() > limits.max_states {
            return Outcome::MemoryLimit;
        }
    }
    Outcome::Exhausted
}

fn path_to(nodes: &FxMap<State, Node>, start: State, mut cur: State) -> Vec<Move> {
    let mut path = Vec::new();
    while cur != start {
        let n = &nodes[&cur];
        path.push(n.mv);
        cur = n.parent;
    }
    path.reverse();
    path
}

#[derive(Debug, Clone, Default)]
pub struct AnytimeResult {
    pub best: Option<Vec<Move>>,
    /// The best solution is proven to be the shortest possible.
    pub optimal: bool,
    pub stopped: bool,
    pub memory_limited: bool,
}

/// Greedy → Weighted A* (w = 3, then 2) → A*, keeping the best solution.
pub fn anytime(p: &Puzzle, start: State, limits: Limits, obs: &mut dyn Observer) -> AnytimeResult {
    let mut r = AnytimeResult::default();
    let phases = [
        Strategy::Greedy,
        Strategy::Weighted(3),
        Strategy::Weighted(2),
        Strategy::AStar,
    ];
    for strategy in phases {
        obs.phase(strategy);
        let bound = r.best.as_ref().map(|b| b.len());
        match best_first(p, start, strategy, limits, bound, obs) {
            Outcome::Solved(path) => {
                let optimal = strategy == Strategy::AStar;
                obs.solution(strategy, &path, optimal);
                r.best = Some(path);
                if optimal {
                    r.optimal = true;
                    break;
                }
            }
            Outcome::Exhausted => {
                if strategy == Strategy::AStar {
                    // Nothing shorter than the best exists (or nothing at all).
                    r.optimal = r.best.is_some();
                    if let Some(best) = &r.best {
                        obs.solution(strategy, best, true);
                    }
                    break;
                }
            }
            Outcome::Stopped => {
                r.stopped = true;
                break;
            }
            Outcome::MemoryLimit => r.memory_limited = true,
        }
    }
    r
}
