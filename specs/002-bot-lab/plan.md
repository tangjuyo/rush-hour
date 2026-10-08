# Implementation Plan: Bot Lab

**Branch**: `002-bot-lab` | **Date**: 2026-10-08 | **Spec**: [spec.md](./spec.md)

## Summary

Generalise the core to N×N grids, add a seeded generator for big grids, and add an
anytime search (greedy → weighted A* → A*). The app gets a Bot Lab screen that
runs the search on a background thread, renders the positions being explored live,
and then replays the best solution.

## Technical Context

Same stack as feature 001 (Rust, macroquad). No new dependencies.

- **Engine**: a new `rush_core::engine` module describes a puzzle of any size
  (≤ 11×11, ≤ 32 vehicles, lengths 2–4). A state is a `u128` holding 4 bits per
  vehicle, and occupancy is a `u128` bitboard. The classic `solver` is rebuilt on
  this engine (one move generator for both modes, per Constitution VI).
- **Generator** (`rush_core::lab`): deterministic xorshift from the seed. It
  places random vehicles around a *solved* red car, then makes a biased random
  walk of legal slides that favours positions with more blockers. Moves are
  reversible, so the result is **always solvable**.
- **Search** (`rush_core::search`): best-first search with a pluggable priority:
  - Greedy: `h` only.
  - Weighted A*: `g + w·h` with w = 2.
  - A*: `g + h`.

  The heuristic `h` = (red car not at the exit) + (number of distinct vehicles
  between the red car and the exit). Each of those vehicles must move at least
  once, so `h` is admissible and consistent, and A* with it is optimal. The
  observer callback receives every expansion and solution and may stop the search.
  The core reads no clocks; the app's observer handles time.
- **App** (`crates/rush-hour/src/lab.rs`): a `std::thread` runs `anytime()` and
  publishes snapshots behind an `Arc<Mutex<…>>`: current explored position, counts,
  depth histogram, solutions. An `AtomicBool` cancels the search.

## Constitution Check

| Principle | Status |
|---|---|
| I. Pure Core | engine, generator and search are in rush-core with no I/O and no clock ✅ |
| II. Test-First | `tests/engine.rs` and `tests/lab.rs` are written first ✅ |
| III. Faithful Rules | same movement rules; only size and vehicle length are generalised ✅ |
| IV. Proven | A* is checked against BFS par on all 40 levels; generated grids are solvable by construction and by test ✅ |
| V. Polish | live exploration, a bot character, a timer ring and a replay with confetti ✅ |
| VI. Simplicity | one engine shared by both modes; no new dependencies ✅ |

## Structure

```text
crates/rush-core/src/engine.rs    # N×N puzzle, u128 states, moves, heuristic
crates/rush-core/src/solver.rs    # classic API on top of engine
crates/rush-core/src/lab.rs       # seeded big-grid generator
crates/rush-core/src/search.rs    # anytime best-first search + observer
crates/rush-core/tests/{engine,lab}.rs
crates/rush-hour/src/draw.rs      # generalised to N×N and buses
crates/rush-hour/src/lab.rs       # Bot Lab screen + search thread + robot
crates/rush-hour/src/theme.rs     # slider widget
```
