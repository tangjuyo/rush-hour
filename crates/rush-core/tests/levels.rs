//! Constitution IV — every shipped level is proven (spec FR-008, US2, SC-001).

use rush_core::level::builtin;
use rush_core::{solver, Tier};

#[test]
fn ships_forty_challenges_in_four_tiers_of_ten() {
    let levels = builtin();
    assert_eq!(levels.len(), 40);
    for (i, c) in levels.iter().enumerate() {
        assert_eq!(c.number as usize, i + 1, "numbers run 1..=40 in order");
        assert_eq!(c.tier, Tier::ALL[i / 10], "challenge {} tier", c.number);
    }
}

#[test]
fn every_par_is_the_proven_optimum() {
    for c in builtin() {
        assert_eq!(
            solver::par(&c.board),
            Some(c.par as usize),
            "challenge {} par",
            c.number
        );
    }
}

#[test]
fn tiers_are_ordered_by_difficulty() {
    let levels = builtin();
    for w in levels.windows(2) {
        assert!(
            w[0].par <= w[1].par,
            "challenge {} > {}",
            w[0].number,
            w[1].number
        );
    }
    for t in 0..3 {
        let max_this = levels[t * 10..t * 10 + 10].iter().map(|c| c.par).max();
        let min_next = levels[(t + 1) * 10..(t + 1) * 10 + 10]
            .iter()
            .map(|c| c.par)
            .min();
        assert!(max_this < min_next, "tier {t} overlaps tier {}", t + 1);
    }
}

#[test]
fn boards_are_unique_and_respect_inventory() {
    let levels = builtin();
    let mut seen = std::collections::HashSet::new();
    for c in &levels {
        assert!(seen.insert(c.board.to_string()), "duplicate board {}", c.number);
        let cars = c.board.vehicles().iter().filter(|v| v.length() == 2).count();
        let trucks = c.board.vehicles().iter().filter(|v| v.length() == 3).count();
        assert!(cars <= 12 && trucks <= 4);
    }
}
