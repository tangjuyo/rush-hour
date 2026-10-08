# Contract: Level Text Format

A board is 6 lines of 6 characters (36 cells):

- `.` empty
- `A` red target car (horizontal, row index 2)
- `B`–`L`: other cars (length 2), up to 11
- `O`, `P`, `Q`, `R`: trucks (length 3), up to 4

Orientation is inferred from the shape: identical letters in one row are horizontal,
in one column vertical. The number of cells must match the letter's kind.

Levels file (`crates/rush-core/assets/levels.txt`):

```text
# comments start with '#'
@ 1 Beginner par=3
......
....LL
...AAC
.....C
GG.OOO
......

@ 2 Beginner par=4
...
```

Rules:
- A header `@ <number> <Tier> par=<n>` is followed by exactly 6 board rows. Blank
  lines and `#` comments between entries are ignored.
- Numbers run 1..=40 in order. Tiers are contiguous blocks of 10 in tier order.
- `par` MUST equal the solver's optimal move count. `tests/levels.rs` checks this.
