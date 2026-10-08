# Data Model

## Orientation
`Horizontal | Vertical`

## Kind
`Car` (length 2, ids `A`–`L`, 12 total incl. red `A`) | `Truck` (length 3, ids `O`–`R`, 4 total)

## Vehicle
| Field | Type | Notes |
|---|---|---|
| id | `char` | `'A'` = red target car. The id determines kind and colour. |
| orientation | `Orientation` | fixed |
| lane (`fixed`) | `u8` | row (H) or column (V); never changes |
| pos | `u8` | column (H) or row (V) of the top-left cell, `0..=6-len` |

## Board
`vehicles: Vec<Vehicle>`. `vehicles[0]` is always the red car; the rest follow in
id order, so indices are stable during a game.
Invariants checked by `Board::new`: known and unique ids, inside the grid, no
overlap, red car horizontal on row index 2.

## Move
`{ vehicle: usize, delta: i8 }` where `delta != 0` and every cell crossed is empty.

## Tier
`Beginner | Intermediate | Advanced | Expert` (10 challenges each).

## Challenge
`{ number: 1..=40, tier: Tier, board: Board, par: u32 }`

## Progress (app only)
`BTreeMap<u32 /*number*/, Record { best_moves: u32, stars: u8 }>`

## Stars
`stars(moves, par) = 3 if moves <= par; 2 if moves <= par + ceil(par/2); else 1`
