# Feature Specification: Rush Hour Desktop Game

**Feature Branch**: `001-rush-hour-game`

**Created**: 2026-10-07

**Status**: Implemented (v0.1.0)

**Input**: User description: "Code the game RUSH HOUR. Get the red car out of the
6x6 grid through an opening on the right side; other vehicles block the path and
must be moved. 40 challenges in 4 levels (beginner, intermediate, advanced,
expert). 12 cars and 4 trucks. Cars are 2 cells, trucks 3 cells, the red car is
horizontal. Vehicles slide in a straight line along their own axis, no rotation,
no jump. One user move = sliding one vehicle 1 or more cells in one slide. Goal:
solve with the fewest moves. Desktop app, written in Rust, nice to look at."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Solve a puzzle (Priority: P1) 🎯 MVP

The player sees a 6×6 parking grid with an exit on the right of the third row.
They drag vehicles along their axis to clear a path, then drive the red car out.

**Why this priority**: This is the game. Without it nothing else matters.

**Independent Test**: Open one challenge, solve it by dragging, and see the win
screen with the move count.

**Acceptance Scenarios**:

1. **Given** a horizontal vehicle with free cells to its right, **When** the player
   drags it right by 2 cells, **Then** it ends 2 cells right and the move counter
   increases by exactly 1.
2. **Given** a vertical vehicle, **When** the player drags horizontally,
   **Then** the vehicle does not move sideways.
3. **Given** a vehicle with an obstacle 1 cell away, **When** the player drags it
   3 cells toward the obstacle, **Then** it stops against the obstacle (no jumping).
4. **Given** a drag that ends where it started, **When** released, **Then** the move
   counter does not change.
5. **Given** a clear path from the red car to the exit, **When** the player drags
   the red car to the right edge, **Then** it drives out and the challenge is won.

---

### User Story 2 - Choose among 40 challenges in 4 levels (Priority: P2)

The player picks a challenge from a menu organised in four tiers (Beginner,
Intermediate, Advanced, Expert) of 10 challenges each, ordered by difficulty.

**Why this priority**: Gives the game its length and progression.

**Independent Test**: Open the menu, see 4 tiers × 10 challenges, then open any
one and play it.

**Acceptance Scenarios**:

1. **Given** the main menu, **When** it is displayed, **Then** exactly 40 challenges
   appear, grouped in the 4 named tiers.
2. **Given** any challenge, **When** it is opened, **Then** its board appears with its
   minimum ("par") move count shown.
3. **Given** any tier after the first, **Then** every challenge in it needs strictly
   more minimum moves than every challenge in the previous tier.

---

### User Story 3 - Fewer moves is better: undo, reset, score, progress (Priority: P3)

The player can undo moves and restart, and is rated on how close they got to the
minimum move count. Best results are remembered between sessions.

**Why this priority**: The stated goal is "do it with fewer moves"; this rewards it.

**Independent Test**: Solve a challenge in more moves than par and get fewer
stars. Retry and beat it. Quit, relaunch, and see the best result kept.

**Acceptance Scenarios**:

1. **Given** at least one move made, **When** Undo is used, **Then** the last move is
   reverted and the counter decreases by 1.
2. **Given** any state, **When** Reset is used, **Then** the board returns to the
   start and the counter goes back to 0.
3. **Given** a solve in exactly par moves, **Then** the player gets 3 stars. Within
   par + ⌈par/2⌉ moves they get 2 stars, otherwise 1 star.
4. **Given** a solved challenge, **When** the game is relaunched, **Then** its best
   move count and stars are still shown in the menu.

---

### User Story 4 - Hint (Priority: P4)

A stuck player can ask for a hint, and the next optimal move is shown.

**Why this priority**: It helps players finish, but the game works without it.

**Independent Test**: In any unsolved state, press Hint. Following the hints
repeatedly solves the puzzle in the remaining optimal number of moves.

**Acceptance Scenarios**:

1. **Given** any solvable state, **When** Hint is pressed, **Then** one vehicle and
   its target position are highlighted, and that move lies on an optimal path.

---

### Edge Cases

- A vehicle dragged beyond the board edge stops at the edge.
- A fast drag "through" another vehicle stops at that vehicle.
- Undo with no moves does nothing. On the win screen the player can choose Retry,
  Menu or Next instead.
- Hint on a won board does nothing.
- A corrupt or missing save file must not crash the game. Progress starts empty.
- When the window is resized, the layout rescales and stays centred.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The board MUST be a 6×6 grid with one exit on the right edge of row 3.
- **FR-002**: Vehicles MUST be cars (2 cells) or trucks (3 cells), each with a fixed
  orientation.
- **FR-003**: The red target car MUST be horizontal and on row 3 in every challenge.
- **FR-004**: Vehicles MUST be drawn from 12 cars (including the red car) and
  4 trucks, each with a distinct colour.
- **FR-005**: A vehicle MUST only slide along its own axis. It MUST never overlap
  another vehicle or leave the grid.
- **FR-006**: One completed slide of one vehicle that changes its position MUST count
  as exactly one move, whatever the distance.
- **FR-007**: The challenge MUST be won when the red car reaches the exit.
- **FR-008**: The game MUST ship 40 challenges in 4 tiers of 10, ordered by
  minimum solution length, each proven solvable.
- **FR-009**: Each challenge MUST display its par.
- **FR-010**: Players MUST be able to undo moves and reset the challenge.
- **FR-011**: The game MUST award 1–3 stars by the rule in US3 scenario 3.
- **FR-012**: The game MUST save the best move count and stars per challenge locally.
- **FR-013**: The game MUST give an optimal next-move hint on request.
- **FR-014**: Vehicle motion MUST be animated smoothly. Winning MUST play a
  celebration (the red car drives out, followed by a visual effect).
- **FR-015**: Keyboard shortcuts: Undo (Z / U / Backspace), Reset (R), Hint (H),
  Menu (Esc), Next (Enter / N on the win screen). In the menu, arrows select a
  challenge and Enter plays it.

### Key Entities

- **Vehicle**: identity (letter and colour), kind (car or truck), orientation, lane, position.
- **Board**: the 6×6 grid and its vehicles. `vehicles[0]` is the red car.
- **Move**: vehicle index and signed displacement in cells.
- **Challenge**: number 1–40, tier, starting board, par.
- **Progress**: best move count and stars for each challenge.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the 40 challenges are proven solvable, and each par equals the
  shortest solution.
- **SC-002**: The move counter follows "one slide = one move" in 100% of the
  acceptance scenarios.
- **SC-003**: The game stays smooth (≈60 fps) during drags and animations.
- **SC-004**: A hint appears in under 200 ms for any reachable position.
- **SC-005**: A new player can solve challenge 1 without instructions in under 2 minutes.

## Assumptions

- "Trunk" in the request means truck (3 cells).
- "12 cars" includes the red car, as in the physical game (12 + 4 = 16 vehicles).
- The challenges are original layouts, generated and verified by the game's own
  solver. ThinkFun's 40 cards are copyrighted, so they are not copied.
- All 40 challenges are unlocked from the start. Progress is shown, not enforced.
- Single player, offline, English UI, mouse input, desktop only.
