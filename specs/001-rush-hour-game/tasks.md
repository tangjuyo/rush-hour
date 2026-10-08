---
description: "Task list for Rush Hour desktop game"
---

# Tasks: Rush Hour Desktop Game

**Input**: Design documents from `/specs/001-rush-hour-game/`

**Tests**: Required for `rush-core` (Constitution II).

## Phase 1: Setup

- [X] T001 Create the Cargo workspace (`Cargo.toml`) with `crates/rush-core` and `crates/rush-hour`
- [X] T002 [P] Add `rustfmt.toml`, `.gitignore` and `README.md`
- [X] T003 [P] Embed OFL fonts in `crates/rush-hour/assets/` (Lilita One, Nunito ExtraBold) with their licences

## Phase 2: Foundational (core rules; blocks everything)

- [X] T004 Write the rules tests first in `crates/rush-core/tests/rules.rs` (axis-only, no overlap, no jump, bounds, win) and confirm they fail
- [X] T005 Implement `Orientation`, `Kind`, `Vehicle`, `Board`, `Move`, `slide_range`, `legal_moves`, `apply`, `is_solved` in `crates/rush-core/src/board.rs`
- [X] T006 Write tests for the level parser and serializer, then implement them in `crates/rush-core/src/level.rs` (contract: contracts/level-format.md)
- [X] T007 Write tests for the BFS solver (`solve`, `par`, `hint`) and `Cluster`, then implement them in `crates/rush-core/src/solver.rs`

**Checkpoint**: the core can parse, play and solve any board (19 tests green).

## Phase 3: US1 Solve a puzzle (P1) 🎯 MVP

- [X] T008 [US1] macroquad window, screen state machine and dev flags in `crates/rush-hour/src/main.rs`
- [X] T009 [P] [US1] Palette, vehicle colours, fonts and shape primitives in `crates/rush-hour/src/theme.rs`
- [X] T010 [US1] Board and vehicle rendering in `crates/rush-hour/src/draw.rs`
- [X] T011 [US1] Drag input clamped by `slide_range`, snapping, eased animation and move counter in `crates/rush-hour/src/game.rs`
- [X] T012 [US1] Win detection: the red car drives out, confetti, win card

## Phase 4: US2 40 challenges / 4 tiers (P2)

- [X] T013 [US2] Generator (xorshift RNG, layout hill-climb on cluster hardness) in `crates/rush-core/src/generator.rs`
- [X] T014 [US2] `levelgen` binary writing `crates/rush-core/assets/levels.txt`
- [X] T015 [US2] `tests/levels.rs`: 40 levels, 4×10 tiers, par equals the solver result, tiers strictly ordered, unique boards, inventory respected
- [X] T016 [US2] Level select menu with live preview in `crates/rush-hour/src/menu.rs`

## Phase 5: US3 Undo / Reset / Stars / Progress (P3)

- [X] T017 [US3] Write the tests for `stars()`, then implement it in `crates/rush-core/src/level.rs`
- [X] T018 [US3] Undo and Reset buttons and keyboard shortcuts in `game.rs`
- [X] T019 [US3] Progress JSON save/load in `crates/rush-hour/src/progress.rs`; stars shown in the menu and the panel

## Phase 6: US4 Hint (P4)

- [X] T020 [US4] Hint button and shortcut: pulsing glow on the vehicle plus a ghost at its target position

## Phase 7: Polish

- [X] T021 Resolution-independent layout (`ui_scale`), hover lift, exit glow when the path is clear, keyboard navigation in the menu
- [X] T022 `cargo fmt --check`, `clippy -D warnings`, `cargo test`; screenshots of the menu, game, hint and win checked visually

## Backlog (not in v0.1)

- [ ] T023 Sound effects (slide, bump, win). Needs the macroquad `audio` feature and CC0 samples.
- [ ] T024 Move history / replay of the optimal solution
- [ ] T025 Window icon and packaged installers (cargo-bundle / cargo-dist)

## Dependencies

Phase 1 → Phase 2 → US1 → US2 → US3 → US4 → Polish. T013–T015 can run in
parallel with T008–T012.
