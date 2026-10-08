//! # rush-core
//!
//! Pure game logic for Rush Hour: board and movement rules, an optimal BFS
//! solver, the level text format and a puzzle generator.
//!
//! This crate deliberately has **no dependencies** and does no I/O
//! (Constitution, Principle I).

pub mod board;
pub mod generator;
pub mod level;
pub mod solver;

pub use board::{Board, BoardError, Kind, Move, MoveError, Orientation, Vehicle, EXIT_ROW, SIZE};
pub use level::{stars, Challenge, Tier};
