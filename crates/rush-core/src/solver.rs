//! Optimal solver (breadth-first search) and whole-cluster analysis for the
//! classic 6×6 game, built on the shared [`crate::engine`].
//!
//! Each BFS edge is one slide of any distance, so BFS depth equals the
//! official move count.

use crate::board::{Board, Move};
use crate::engine::{Puzzle, State};
use std::collections::{HashMap, VecDeque};
use std::hash::{BuildHasherDefault, Hasher};

/// Tiny, fast hasher for integer keys (FxHash-style).
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
    fn write_u128(&mut self, x: u128) {
        self.write_u64(x as u64);
        self.write_u64((x >> 64) as u64);
    }
}

pub(crate) type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

/// Shortest solution (sequence of moves), `Some(vec![])` if already solved,
/// `None` if unsolvable.
pub fn solve(board: &Board) -> Option<Vec<Move>> {
    let (p, start) = Puzzle::from_board(board);
    bfs(&p, start)
}

/// Plain BFS on any [`Puzzle`].
pub fn bfs(p: &Puzzle, start: State) -> Option<Vec<Move>> {
    if p.is_goal(start) {
        return Some(Vec::new());
    }
    let mut parent: FxMap<State, (State, Move)> = FxMap::default();
    parent.insert(start, (start, Move { vehicle: 0, delta: 0 }));
    let mut queue = VecDeque::from([start]);
    while let Some(s) = queue.pop_front() {
        let mut found = None;
        p.for_each_neighbor(s, |n, mv| {
            if found.is_none() && !parent.contains_key(&n) {
                parent.insert(n, (s, mv));
                if p.is_goal(n) {
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
    puzzle: Puzzle,
    states: Vec<State>,
    /// Distance to goal, `u32::MAX` if the goal is unreachable.
    dist: Vec<u32>,
}

impl Cluster {
    pub fn explore(board: &Board) -> Cluster {
        let (puzzle, start) = Puzzle::from_board(board);
        // 1. Enumerate the connected component (moves are reversible).
        let mut index: FxMap<State, u32> = FxMap::default();
        let mut states = vec![start];
        index.insert(start, 0);
        let mut adj_start = vec![0u32];
        let mut adj: Vec<u32> = Vec::new();
        let mut k = 0;
        while k < states.len() {
            let s = states[k];
            puzzle.for_each_neighbor(s, |n, _| {
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
            if puzzle.is_goal(s) {
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
            puzzle,
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
            .map(|(&s, _)| self.template.with_positions(self.puzzle.positions(s).into_iter()))
            .collect()
    }
}
