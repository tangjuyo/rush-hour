//! Solver and level-format tests (spec FR-008, FR-009, FR-011, FR-013).

use rush_core::level::{parse_board, parse_levels, write_levels};
use rush_core::solver::{self, Cluster};
use rush_core::{stars, Challenge, Tier};

#[test]
fn already_solved_board_needs_zero_moves() {
    let b = parse_board("......\n......\n....AA\n......\n......\n......").unwrap();
    assert_eq!(solver::par(&b), Some(0));
    assert_eq!(solver::hint(&b), None);
}

#[test]
fn clear_path_is_one_move() {
    let b = parse_board("......\n......\nAA....\n......\n......\n......").unwrap();
    assert_eq!(solver::par(&b), Some(1));
}

#[test]
fn blocker_needs_two_moves() {
    // B blocks the exit row; move it up or down (1 move) then drive out (1 move).
    let b = parse_board("......\n......\nAA.B..\n...B..\n......\n......").unwrap();
    assert_eq!(solver::par(&b), Some(2));
}

#[test]
fn unsolvable_board_has_no_par() {
    // Horizontal truck on the exit row to the right of the red car can never move away.
    let b = parse_board("......\n......\nAA.BB.\n......\n......\n......");
    // B on the exit row is a legal board, but unsolvable.
    let b = b.unwrap();
    assert_eq!(solver::par(&b), None);
}

#[test]
fn following_hints_solves_in_par_moves() {
    let start = parse_board(
        "\
OOO...
.....P
AA...P
.....P
...QQQ
......",
    )
    .unwrap();
    let par = solver::par(&start).expect("solvable");
    let mut b = start.clone();
    let mut n = 0;
    while !b.is_solved() {
        let mv = solver::hint(&b).expect("hint available");
        b.apply(mv).unwrap();
        n += 1;
        assert!(n <= par, "hints must follow an optimal path");
    }
    assert_eq!(n, par);
}

#[test]
fn solution_path_is_legal_and_optimal_length() {
    let b = parse_board("..BBC.\n....C.\nAA..D.\nO...D.\nO.....\nO.....").unwrap();
    let path = solver::solve(&b).unwrap();
    let mut cur = b.clone();
    for mv in &path {
        cur.apply(*mv).unwrap();
    }
    assert!(cur.is_solved());
    assert_eq!(Some(path.len()), solver::par(&b));
}

#[test]
fn cluster_hardest_state_matches_solver() {
    let b = parse_board("..BBC.\n....C.\nAA..D.\nO...D.\nO.....\nO.....").unwrap();
    let cluster = Cluster::explore(&b);
    let (hardest, d) = cluster.hardest().unwrap();
    assert_eq!(solver::par(&hardest), Some(d));
    assert!(d >= solver::par(&b).unwrap());
    for s in cluster.boards_at(1) {
        assert_eq!(solver::par(&s), Some(1));
    }
}

#[test]
fn stars_rule() {
    assert_eq!(stars(10, 10), 3);
    assert_eq!(stars(9, 10), 3);
    assert_eq!(stars(11, 10), 2);
    assert_eq!(stars(15, 10), 2); // par + ceil(10/2)
    assert_eq!(stars(16, 10), 1);
    assert_eq!(stars(5, 3), 2); // par + ceil(3/2) = 5
    assert_eq!(stars(6, 3), 1);
}

#[test]
fn levels_file_round_trip() {
    let text = "\
# sample
@ 1 Beginner par=2
......
......
AA.B..
...B..
......
......
";
    let levels = parse_levels(text).unwrap();
    assert_eq!(levels.len(), 1);
    let Challenge {
        number, tier, par, ..
    } = levels[0].clone();
    assert_eq!((number, tier, par), (1, Tier::Beginner, 2));
    let again = parse_levels(&write_levels(&levels)).unwrap();
    assert_eq!(again, levels);
}

#[test]
fn malformed_levels_are_rejected() {
    assert!(parse_levels("@ 1 Beginner par=2\n......\n").is_err());
    assert!(parse_levels("@ 1 Wizard par=2\n......\n......\nAA....\n......\n......\n......\n").is_err());
    assert!(parse_levels("@ x Beginner par=2\n......\n......\nAA....\n......\n......\n......\n").is_err());
}
