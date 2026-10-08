# Quickstart

## Prerequisites

- Rust stable from https://rustup.rs
- Linux only: install the development headers for X11 and OpenGL.
  - Fedora: `sudo dnf install libX11-devel libXi-devel mesa-libGL-devel`
  - Debian/Ubuntu: `sudo apt install libx11-dev libxi-dev libgl1-mesa-dev`
- Windows and macOS: no extra packages are needed.

## Play

```bash
cargo run --release -p rush-hour
```

## Develop

```bash
cargo test --workspace --release            # rules, solver, level proofs
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

# Regenerate the 40 challenges (seconds, seed):
cargo run --release -p rush-core --bin levelgen -- 240 2026 > crates/rush-core/assets/levels.txt
```

## Screenshots without clicking

```bash
cargo run --release -p rush-hour -- --level 12 --hint --screenshot game.png --frames 40
cargo run --release -p rush-hour -- --level 3 --solve --screenshot win.png --frames 150
```

Screenshot mode never touches your saved progress.

## Validate against the spec

1. Open challenge 1 and drag a blocker: the counter goes up by 1 per slide (US1-1).
2. Drag a car into another one: it stops and doesn't jump (US1-3).
3. The menu shows 4 tiers × 10 challenges (US2). Undo, Reset and Hint work (US3, US4).
4. Solve a challenge, quit, relaunch: the stars are still there (US3-4).
