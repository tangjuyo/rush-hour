# Implementation Plan: Rush Hour Desktop Game

**Branch**: `001-rush-hour-game` | **Date**: 2026-10-07 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-rush-hour-game/spec.md`

## Summary

A native desktop Rush Hour game in Rust. The pure `rush-core` library holds the
rules, an optimal BFS solver (used for par and hints), the level text format and a
puzzle generator. The `rush-hour` binary uses **macroquad** to draw an animated
2D UI by hand. The 40 original challenges are generated offline by `levelgen`,
proven by the solver, and embedded in the binary.

## Technical Context

**Language/Version**: Rust stable (built with 1.99), edition 2021

**Primary Dependencies**: `macroquad 0.4` (window, input, 2D drawing, TTF text;
audio feature disabled), `serde` and `serde_json` (save file), `directories`
(OS data dir). `rush-core` has **zero** dependencies.

**Storage**: JSON at `ProjectDirs("rush-hour").data_dir()/progress.json`
(`~/.local/share/rush-hour/progress.json` on Linux).

**Testing**: `cargo test` (integration tests in `crates/rush-core/tests/`), clippy,
rustfmt, and scripted screenshots via the app's developer flags.

**Target Platform**: Linux, Windows and macOS desktop.

**Project Type**: desktop-app + library.

**Performance Goals**: 60 fps. A full solve of a 6×6 position takes milliseconds in
release builds (all 40 levels re-solved in 0.11 s).

**Constraints**: Offline. A single binary (~1.7 MB) with levels and fonts embedded.

**Scale/Scope**: 40 levels, 2 screens (menu, game) and a win overlay.

## Constitution Check

| Principle | Gate | Status |
|---|---|---|
| I. Pure Core | `rush-core` has no dependencies and its library code does no I/O | ✅ |
| II. Test-First | rule and solver tests were written and seen failing before the implementation | ✅ |
| III. Faithful Rules | rules encoded in `board.rs`, covered by `tests/rules.rs` | ✅ |
| IV. Every Level Is Proven | `tests/levels.rs` re-solves all 40 levels and checks tier ordering | ✅ |
| V. Polish | eased motion, palette, embedded fonts, confetti and win card | ✅ |
| VI. Simplicity | macroquad rather than Bevy (see research.md R1) | ✅ |

Post-design re-check: no violations.

## Project Structure

### Documentation (this feature)

```text
specs/001-rush-hour-game/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md  tasks.md
├── checklists/requirements.md
└── contracts/level-format.md
```

### Source Code (repository root)

```text
Cargo.toml                       # workspace
crates/
├── rush-core/                   # PURE rules library (no deps)
│   ├── assets/levels.txt        # 40 generated challenges (include_str!)
│   ├── src/board.rs             # Vehicle, Orientation, Kind, Board, Move, rules
│   ├── src/solver.rs            # BFS solve/par/hint, Cluster analysis
│   ├── src/level.rs             # text format, Tier, Challenge, stars()
│   ├── src/generator.rs         # xorshift RNG, layout hill-climb
│   ├── src/bin/levelgen.rs      # offline tool → assets/levels.txt
│   └── tests/{rules,solver,levels}.rs
└── rush-hour/                   # desktop app (macroquad)
    ├── assets/                  # Lilita One + Nunito ExtraBold (OFL)
    └── src/{main,theme,draw,game,menu,progress}.rs
docs/screenshots/                # rendered by --screenshot
```

**Structure Decision**: a two-crate Cargo workspace. Splitting the library from the
app enforces Principle I at compile time.

## Complexity Tracking

None.
