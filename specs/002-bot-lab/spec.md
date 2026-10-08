# Feature Specification: Bot Lab

**Feature Branch**: `002-bot-lab`

**Created**: 2026-10-08

**Status**: Clarified

**Input**: User description: "A bot section in the app. Grids are bigger, and a
little bot tries to find the best solution (fewest moves). We press Play and watch
the bot move cars while it searches, with a 30 s timer. Grids are randomly
generated."

## Clarifications

### Session 2026-10-08

- Q: Grid size? → A: chosen per run with a slider, 6×6 to 10×10.
- Q: What do we watch? → A: both. A live exploration view while the bot thinks,
  then a replay of its best solution.
- Q: Vehicles? → A: cars (2), trucks (3) **and buses (4)**.
- Q: Extras? → A: an animation speed slider and a replayable/shareable seed. **Later
  (feature 003):** several bots with different strategies racing side by side.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Watch the bot solve a random big grid (Priority: P1) 🎯 MVP

The player opens Bot Lab, sees a random N×N grid, presses Play, and watches the
bot "think" for up to 30 s. The vehicles shuffle as the bot explores and the
statistics update live. Then the bot replays the shortest solution it found, and
the red car drives out.

**Independent Test**: Open Bot Lab, press Play, wait at most 30 s, and watch the
replay end with the red car exiting. The displayed move count must equal the
number of replayed slides.

**Acceptance Scenarios**:

1. **Given** Bot Lab with a grid, **When** Play is pressed, **Then** a 30 s countdown
   starts, the vehicles move to show the positions being explored, and the explored
   state count rises.
2. **Given** the bot finds a solution, **Then** its length appears immediately, and
   it is replaced whenever a shorter one is found.
3. **Given** the bot proves its solution optimal before 30 s, **Then** thinking stops
   early and the result is marked "optimal".
4. **Given** thinking has ended, **Then** the best solution replays one slide at a time
   and the red car exits.
5. **Given** time runs out before any solution is found, **Then** the bot says so and
   offers a new grid.

### User Story 2 - Random grids from a seed and size (Priority: P2)

**Acceptance Scenarios**:

1. **Given** the size slider at N (6–10), **When** New grid is pressed, **Then** a new
   random N×N grid appears with its seed shown.
2. **Given** a seed is typed in, **Then** exactly the same grid is generated again
   (same size + same seed ⇒ identical grid). The seed can be copied to the
   clipboard.
3. Every generated grid is solvable. It contains cars, trucks and (when N ≥ 7)
   buses, and the red car is on the exit row.

### User Story 3 - Animation speed (Priority: P3)

1. **Given** the speed slider, **When** it is moved, **Then** both the exploration view
   and the replay speed up or slow down immediately.

### Edge Cases

- Pressing Stop while the bot is thinking ends the search and replays the best
  solution found so far, if any.
- Pressing New grid or Back during a search cancels it without freezing the UI.
- Very large searches stop at a memory cap and report it as "memory limit".

## Requirements *(mandatory)*

- **FR-101**: Bot Lab MUST be reachable from the main menu, and Back MUST return to it.
- **FR-102**: Grid size MUST be selectable from 6 to 10. The exit is on the right
  edge of row ⌊(N−1)/2⌋, which is row 3 for 6×6.
- **FR-103**: Vehicles MUST be cars (2), trucks (3) or buses (4). The red car (2) is
  horizontal on the exit row. The game rules are unchanged.
- **FR-104**: Generation MUST be deterministic from (size, seed) and MUST always
  produce a solvable grid.
- **FR-105**: The search MUST run without blocking rendering, MUST respect a 30 s
  budget, and MUST be cancellable.
- **FR-106**: The bot MUST be *anytime*: it reports better solutions as it finds them,
  and it reports when it has proven the optimum.
- **FR-107**: Every solution the bot shows MUST be a legal sequence of slides that
  ends with the red car at the exit.
- **FR-108**: The UI MUST show the countdown, explored states, states per second,
  current strategy, best length, and an optimal or unproven badge.
- **FR-109**: A speed slider MUST control the exploration and replay animation speed.
- **FR-110**: An animated bot character MUST show its mood: idle, thinking, found,
  optimal, or failed.

## Success Criteria *(mandatory)*

- **SC-101**: On 6×6, the bot's "optimal" result equals the classic solver's par.
  This is checked on all 40 shipped levels.
- **SC-102**: The UI stays smooth (≈60 fps) while the bot searches.
- **SC-103**: 100% of generated grids for sizes 6–10 are solvable. A test checks
  many seeds.

## Assumptions

- Search runs on a background thread. The 30 s budget is fixed. Memory is capped
  at about 4 million stored positions.
- A seed is a number from 0 to 999 999 999.
