# Research: Rush Hour Desktop Game

## R1. Rust GUI/game library

| Option | Verdict |
|---|---|
| **macroquad 0.4** | **Chosen.** Immediate-mode 2D drawing, TTF text, input and a game loop in a few lines. Fast compiles, small dependency tree, single binary on Linux/Windows/macOS. Every visual is drawn by hand, so the look is fully under our control. |
| Bevy | Very capable (ECS, UI, bloom), but heavy compile times and a large API that this puzzle doesn't need (Principle VI). Reconsider it for 3D, shaders or large-scale content. |
| egui/eframe | Great for tool-style apps, but it looks like a form, not a game. Animated drag-and-drop pieces fight the widget model. |
| ggez | Similar to macroquad, but less active. |
| iced / slint | Retained-mode GUI toolkits. Custom animated game pieces are awkward to build. |

## R2. State representation and solver

Each vehicle has a fixed axis, lane and length; only its offset along the axis
changes. A state is therefore packed into a `u64` (3 bits × ≤ 16 vehicles) and an
occupancy bitboard (36 bits) is derived from it. In the BFS, each edge is **one
slide of any distance**, so BFS depth equals the official move count. A hint is
the first move of a shortest path.

Solved = the red car's front reaches column 6 (the exit). The same convention is
used in M. Fogleman's Rush Hour database.

## R3. Level source

ThinkFun's 40 cards are copyrighted, so we generate original puzzles:

1. Start from a random layout drawn from the official inventory: the red car,
   up to 11 other cars and up to 4 trucks.
2. Hill-climb on *layouts*. Each mutation adds, removes or relocates a vehicle.
   A layout's score is the par of the hardest position in its whole reachable
   **cluster**, found by enumerating the cluster and then running a multi-source
   BFS from every solved state. Keep the mutation if the score doesn't decrease.
3. Choose the 40 challenges against fixed target pars, one distinct layout each.

**Result** (`levelgen 240 2026`, 1 CPU core, 4 minutes):

| Tier | Pars |
|---|---|
| Beginner | 3–8 |
| Intermediate | 10–17 |
| Advanced | 18–27 |
| Expert | 29–46 |

The hardest known 6×6 position needs 51 moves, so challenge 40 (par 46) is near
the theoretical maximum.

## R4. Visual direction

The theme is a night car park:
- Dark slate gradient, with a raised board frame and rounded cells.
- The exit row is tinted, and animated chevrons sit at the exit. They glow when
  the red car's path is clear.
- Vehicles are drawn top-down: rounded body, chassis depth, roof, glass,
  headlights and taillights. Trucks have a ribbed cargo box. The red car has
  racing stripes.
- Picking up or hovering a vehicle lifts it and grows its shadow. Motion uses an
  exponential ease toward the snapped cell.
- On a win, the red car accelerates out through the exit, confetti bursts from
  the gate, and a card shows the stars popping in.
- Fonts: Lilita One (display) and Nunito ExtraBold (UI), both under the SIL Open
  Font License (OFL) and embedded.
- All shapes are built from triangle fans, so translucent shapes never
  double-blend at corners.

## R5. Persistence

Progress is saved to `directories::ProjectDirs::from("", "", "rush-hour")` → `data_dir/progress.json`.
It is written to a temporary file and then renamed into place. A missing or
corrupt file means progress starts empty. Screenshot mode uses in-memory progress
only.
