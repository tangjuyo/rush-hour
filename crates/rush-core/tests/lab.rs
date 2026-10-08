//! Feature 002: seeded big-grid generator (FR-102..FR-104, SC-103).

use rush_core::lab::{exit_row, generate};
use rush_core::search::{best_first, Limits, Observer, Outcome, Strategy};

struct Quiet;
impl Observer for Quiet {}

#[test]
fn exit_row_is_row_three_on_classic_size() {
    assert_eq!(exit_row(6), 2);
    assert_eq!(exit_row(8), 3);
    assert_eq!(exit_row(10), 4);
}

#[test]
fn same_size_and_seed_give_the_same_grid() {
    for size in 6..=10 {
        let a = generate(size, 12345);
        let b = generate(size, 12345);
        assert_eq!(a, b);
        assert_ne!(a, generate(size, 12346), "size {size}");
    }
}

#[test]
fn generated_grids_are_valid_interesting_and_solvable() {
    for size in 6..=10u8 {
        let mut buses = 0;
        for seed in 0..6u64 {
            let (p, s) = generate(size, seed);
            assert_eq!(p.size(), size);
            assert_eq!(p.exit_row(), exit_row(size));
            assert!(p.is_valid(s), "size {size} seed {seed}");
            assert!(!p.is_goal(s), "already solved: size {size} seed {seed}");
            assert!(p.heuristic(s) >= 2, "red car path must be blocked");
            assert!(p.pieces().len() >= 6);
            assert!(p.pieces().iter().all(|q| (2..=4).contains(&q.len)));
            buses += p.pieces().iter().filter(|q| q.len == 4).count();
            let out = best_first(&p, s, Strategy::Greedy, Limits::default(), None, &mut Quiet);
            assert!(
                matches!(out, Outcome::Solved(_)),
                "size {size} seed {seed}: {out:?}"
            );
        }
        if size >= 7 {
            assert!(buses > 0, "size {size} should contain buses");
        }
    }
}
