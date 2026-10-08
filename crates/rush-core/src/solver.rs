//! Optimal solver (breadth-first search) and whole-cluster analysis.
//!
//! A state packs every vehicle's position into a `u64` (3 bits each, ≤ 16
//! vehicles). Each BFS edge is one slide of any distance, so BFS depth is the
//! official move count.

use crate::board::{Board, Move, Orientation, SIZE};
use std::collections::{HashMap, VecDeque};
use std::hash::{BuildHasherDefault, Hasher};

/// Tiny, fast hasher for `u64` keys (FxHash-style).
#[derive(Default)]
pub(crate) struct FxHasher(u64);

impl Hasher for FxHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }
    fn write_u64(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

pub(crate) type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

type State = u64;

/// Fixed (never changing) part of every vehicle.
struct Layout {
    horizontal: Vec<bool>,
    len: Vec<u8>,
    lane: Vec<u8>,
}

impl Layout {
    fn of(board: &Board) -> (Layout, State) {
        let vs = board.vehicles();
        let layout = Layout {
            horizontal: vs
                .iter()
                .map(|v| v.orientation() == Orientation::Horizontal)
                .collect(),
            len: vs.iter().map(|v| v.length()).collect(),
            lane: vs.iter().map(|v| v.lane()).collect(),
        };
        let state = vs
            .iter()
            .enumerate()
            .fold(0u64, |s, (i, v)| s | (v.pos() as u64) << (3 * i));
        (layout, state)
    }

    fn n(&self) -> usize {
        self.len.len()
    }

    #[inline]
    fn pos(state: State, i: usize) -> u8 {
        ((state >> (3 * i)) & 7) as u8
    }

    #[inline]
    fn bit(&self, i: usize, p: u8) -> u64 {
        if self.horizontal[i] {
            1 << (self.lane[i] * SIZE + p)
        } else {
            1 << (p * SIZE + self.lane[i])
        }
    }

    fn occupancy(&self, s: State) -> u64 {
        let mut occ = 0;
        for i in 0..self.n() {
            let p = Self::pos(s, i);
            for k in 0..self.len[i] {
                occ |= self.bit(i, p + k);
            }
        }
        occ
    }

    fn is_goal(&self, s: State) -> bool {
        Self::pos(s, 0) + self.len[0] == SIZE
    }

    /// Calls `f(next_state, move)` for every single-slide successor.
    #[inline]
    fn for_each_neighbor(&self, s: State, mut f: impl FnMut(State, Move)) {
        let occ = self.occupancy(s);
        for i in 0..self.n() {
            let p = Self::pos(s, i);
            let cleared = s & !(7u64 << (3 * i));
            let mut d = 1u8;
            while d <= p && occ & self.bit(i, p - d) == 0 {
                f(
                    cleared | ((p - d) as u64) << (3 * i),
                    Move {
                        vehicle: i,
                        delta: -(d as i8),
                    },
                );
                d += 1;
            }
            let end = p + self.len[i] - 1;
            let mut d = 1u8;
            while end + d < SIZE && occ & self.bit(i, end + d) == 0 {
                f(
                    cleared | ((p + d) as u64) << (3 * i),
                    Move {
                        vehicle: i,
                        delta: d as i8,
                    },
                );
                d += 1;
            }
        }
    }

    fn board(&self, template: &Board, s: State) -> Board {
        template.with_positions((0..self.n()).map(|i| Self::pos(s, i)))
    }
}

/// Shortest solution (sequence of moves), `Some(vec![])` if already solved,
/// `None` if unsolvable.
pub fn solve(board: &Board) -> Option<Vec<Move>> {
    let (layout, start) = Layout::of(board);
    if layout.is_goal(start) {
        return Some(Vec::new());
    }
    let mut parent: FxMap<State, (State, Move)> = FxMap::default();
    parent.insert(start, (start, Move { vehicle: 0, delta: 0 }));
    let mut queue = VecDeque::from([start]);
    while let Some(s) = queue.pop_front() {
        let mut found = None;
        layout.for_each_neighbor(s, |n, mv| {
            if found.is_none() && !parent.contains_key(&n) {
                parent.insert(n, (s, mv));
                if layout.is_goal(n) {
                    found = Some(n);
                } else {
                    queue.push_back(n);
                }
            }
        });
        if let Some(mut cur) = found {
            let mut path = Vec::new();
            while cur != start {
                let (prev, mv) = parent[&cur];
                path.push(mv);
                cur = prev;
            }
            path.reverse();
            return Some(path);
        }
    }
    None
}

/// Minimum number of moves to solve, or `None` if unsolvable.
pub fn par(board: &Board) -> Option<usize> {
    solve(board).map(|p| p.len())
}

/// First move of an optimal solution (`None` if solved or unsolvable).
pub fn hint(board: &Board) -> Option<Move> {
    solve(board).and_then(|p| p.first().copied())
}

/// Every position reachable from a board, with its distance to the nearest
/// solved position. Used by the generator to pick puzzles of a given par.
pub struct Cluster {
    template: Board,
    layout: Layout,
    states: Vec<State>,
    /// Distance to goal, `u32::MAX` if the goal is unreachable.
    dist: Vec<u32>,
}

impl Cluster {
    pub fn explore(board: &Board) -> Cluster {
        let (layout, start) = Layout::of(board);
        // 1. Enumerate the connected component (moves are reversible).
        let mut index: FxMap<State, u32> = FxMap::default();
        let mut states = vec![start];
        index.insert(start, 0);
        let mut adj_start = vec![0u32];
        let mut adj: Vec<u32> = Vec::new();
        let mut k = 0;
        while k < states.len() {
            let s = states[k];
            layout.for_each_neighbor(s, |n, _| {
                let next_id = states.len() as u32;
                let id = *index.entry(n).or_insert_with(|| {
                    states.push(n);
                    next_id
                });
                adj.push(id);
            });
            adj_start.push(adj.len() as u32);
            k += 1;
        }
        // 2. Multi-source BFS from every solved state.
        let mut dist = vec![u32::MAX; states.len()];
        let mut queue: VecDeque<u32> = VecDeque::new();
        for (i, &s) in states.iter().enumerate() {
            if layout.is_goal(s) {
                dist[i] = 0;
                queue.push_back(i as u32);
            }
        }
        while let Some(i) = queue.pop_front() {
            let d = dist[i as usize] + 1;
            for &j in &adj[adj_start[i as usize] as usize..adj_start[i as usize + 1] as usize] {
                if dist[j as usize] == u32::MAX {
                    dist[j as usize] = d;
                    queue.push_back(j);
                }
            }
        }
        Cluster {
            template: board.clone(),
            layout,
            states,
            dist,
        }
    }

    /// Number of reachable positions.
    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Largest distance-to-goal in the cluster, `None` if unsolvable.
    pub fn max_distance(&self) -> Option<usize> {
        self.dist
            .iter()
            .filter(|&&d| d != u32::MAX)
            .max()
            .map(|&d| d as usize)
    }

    /// The hardest position of the cluster and its par.
    pub fn hardest(&self) -> Option<(Board, usize)> {
        let d = self.max_distance()?;
        self.boards_at(d).into_iter().next().map(|b| (b, d))
    }

    /// Every position whose par is exactly `d`.
    pub fn boards_at(&self, d: usize) -> Vec<Board> {
        self.states
            .iter()
            .zip(&self.dist)
            .filter(|(_, &x)| x as usize == d)
            .map(|(&s, _)| self.layout.board(&self.template, s))
            .collect()
    }
}
