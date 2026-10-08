<!--
Sync Impact Report
- Version: 0.0.0 (template) → 1.0.0 (initial ratification)
- Principles added: I. Pure Core, II. Test-First Core, III. Faithful Rules,
  IV. Every Level Is Proven, V. Polish Is a Feature, VI. Simplicity
- Sections added: Technology Constraints, Development Workflow, Governance
- Templates: plan/spec/tasks templates compatible — no changes required
-->

# Rush Hour Constitution

## Core Principles

### I. Pure Core
All game rules (board, vehicles, moves, win detection, solver, level parsing,
level generation) live in the `rush-core` library crate. `rush-core` MUST NOT
depend on any graphics, windowing, audio or filesystem crate, and its library
code performs no I/O. The desktop app only translates input into core `Move`s
and renders core state.
*Rationale*: the rules are the product. Keeping them pure makes them testable and
reusable (a future TUI or WASM front-end), and the UI can't corrupt them.

### II. Test-First Core (NON-NEGOTIABLE)
Every core behaviour is specified by a failing `cargo test` before it is
implemented (Red → Green → Refactor). UI code is exempt from unit tests but
MUST be verified against the spec's acceptance scenarios (manually or by
scripted screenshots).

### III. Faithful Rules
The official Rush Hour rules are invariant:
- 6×6 grid, single exit on the right edge of row 3 (index 2).
- Cars occupy 2 cells, trucks 3. The red target car is horizontal on the exit row.
- A vehicle slides only along its own axis: no rotation, no jumping, no overlap,
  never outside the grid.
- **One move = one vehicle slid any number of cells in a single slide.**
No feature may bend these rules.

### IV. Every Level Is Proven
No level ships unless the solver proves it solvable and its recorded par equals
the optimal move count. `crates/rush-core/tests/levels.rs` enforces this for all
shipped levels and checks that the tiers are ordered by par. Levels are original
(generated). ThinkFun's copyrighted challenge cards are never copied.

### V. Polish Is a Feature
The game must look and feel good: smooth (eased) vehicle motion, a clear visual
hierarchy, a consistent palette, readable typography and satisfying win
feedback, at a target of 60 fps. Visual regressions count as bugs.

### VI. Simplicity
Use the smallest dependency set that meets the spec. No engine, ECS or framework
unless a requirement demands it. YAGNI. Any added complexity is justified in the
plan's Complexity Tracking table.

## Technology Constraints

- Rust stable, edition 2021, Cargo workspace.
- Native desktop (Linux, Windows, macOS). No web build required.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo test --workspace` MUST pass before every commit.
- Assets (fonts) are embedded in the binary and carry an open licence.

## Development Workflow

Spec Kit flow: constitution → specify → clarify → plan → tasks → analyze →
implement. Each task ends in a passing build. Commits are small and reference
task IDs (e.g. `T012`).

## Governance

This constitution supersedes other practices. Amendments require a version bump
(SemVer: MAJOR for removed or redefined principles, MINOR for additions, PATCH
for wording), an updated Sync Impact Report, and a review of dependent templates.
Every plan includes a Constitution Check gate.

**Version**: 1.0.0 | **Ratified**: 2026-10-07 | **Last Amended**: 2026-10-07
