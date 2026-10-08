//! Feature 002: N×N engine and anytime search (FR-102, FR-103, FR-106, FR-107, SC-101).

use rush_core::engine::{Piece, Puzzle};
use rush_core::level::{builtin, parse_board};
use rush_core::search::{anytime, best_first, Limits, Observer, Outcome, Strategy};
use rush_core::{solver, Move};

struct Quiet;
impl Observer for Quiet {}

struct StopAfter(u64);
impl Observer for StopAfter {
    fn expanded(&mut self, _state: u128, _depth: u32, expanded: u64) -> bool {
        expanded < self.0
    }
}

fn replay(p: &Puzzle, start: u128, path: &[Move]) -> u128 {
    path.iter()
        .fold(start, |s, &mv| p.apply(s, mv).expect("legal move"))
}

#[test]
fn engine_matches_classic_board_rules() {
    let b = parse_board("..BBC.\n....C.\nAA..D.\nO...D.\nO.....\nO.....").unwrap();
    let (p, s) = Puzzle::from_board(&b);
    assert_eq!((p.size(), p.exit_row()), (6, 2));
    for i in 0..b.vehicles().len() {
        assert_eq!(p.slide_range(s, i), b.slide_range(i), "vehicle {i}");
    }
    let mut n = 0;
    p.for_each_neighbor(s, |_, _| n += 1);
    assert_eq!(n, b.legal_moves().len());
    assert_eq!(p.positions(s), b.positions());
}

#[test]
fn puzzle_validation() {
    let red = Piece {
        horizontal: true,
        len: 2,
        lane: 3,
    };
    assert!(Puzzle::new(8, 3, vec![red]).is_ok());
    // Red car must be first, horizontal, length 2, on the exit row.
    assert!(Puzzle::new(8, 3, vec![Piece { lane: 2, ..red }]).is_err());
    assert!(Puzzle::new(
        8,
        3,
        vec![Piece {
            horizontal: false,
            ..red
        }]
    )
    .is_err());
    // Lengths 2..=4 only, sizes 4..=11, at most 32 pieces.
    let five = Piece {
        horizontal: false,
        len: 5,
        lane: 0,
    };
    assert!(Puzzle::new(8, 3, vec![red, five]).is_err());
    assert!(Puzzle::new(12, 5, vec![Piece { lane: 5, ..red }]).is_err());
    let many = std::iter::once(red).chain((0..32).map(|i| Piece {
        horizontal: false,
        len: 2,
        lane: i % 8,
    }));
    assert!(Puzzle::new(8, 3, many.collect()).is_err());
}

#[test]
fn buses_slide_on_big_grids() {
    // 8×8: red car at col 0 of row 3; a vertical bus in column 4, rows 0..=3.
    let p = Puzzle::new(
        8,
        3,
        vec![
            Piece {
                horizontal: true,
                len: 2,
                lane: 3,
            },
            Piece {
                horizontal: false,
                len: 4,
                lane: 4,
            },
        ],
    )
    .unwrap();
    let s = Puzzle::encode(&[0, 0]);
    assert!(p.is_valid(s));
    assert_eq!(p.slide_range(s, 1), (0, 4));
    assert_eq!(p.slide_range(s, 0), (0, 2));
    assert_eq!(p.heuristic(s), 2);
    let moved = p.apply(s, Move { vehicle: 1, delta: 4 }).unwrap();
    assert_eq!(p.heuristic(moved), 1);
    let done = p.apply(moved, Move { vehicle: 0, delta: 6 }).unwrap();
    assert!(p.is_goal(done));
    assert!(
        p.apply(s, Move { vehicle: 0, delta: 3 }).is_none(),
        "no jumping over the bus"
    );
}

#[test]
fn astar_is_optimal_on_all_shipped_levels() {
    for c in builtin() {
        let (p, s) = Puzzle::from_board(&c.board);
        match best_first(&p, s, Strategy::AStar, Limits::default(), None, &mut Quiet) {
            Outcome::Solved(path) => {
                assert_eq!(path.len() as u32, c.par, "challenge {}", c.number);
                assert!(p.is_goal(replay(&p, s, &path)));
            }
            other => panic!("challenge {}: {other:?}", c.number),
        }
    }
}

#[test]
fn greedy_and_weighted_find_legal_solutions() {
    for c in builtin().into_iter().step_by(5) {
        let (p, s) = Puzzle::from_board(&c.board);
        for strategy in [Strategy::Greedy, Strategy::Weighted(2)] {
            let Outcome::Solved(path) = best_first(&p, s, strategy, Limits::default(), None, &mut Quiet)
            else {
                panic!("{strategy:?} failed on {}", c.number)
            };
            assert!(p.is_goal(replay(&p, s, &path)));
            assert!(path.len() as u32 >= c.par);
        }
    }
}

#[test]
fn anytime_proves_the_optimum() {
    let c = &builtin()[25];
    let (p, s) = Puzzle::from_board(&c.board);
    let r = anytime(&p, s, Limits::default(), &mut Quiet);
    let best = r.best.expect("solved");
    assert!(r.optimal);
    assert_eq!(best.len() as u32, c.par);
    assert_eq!(Some(best.len()), solver::par(&c.board));
}

#[test]
fn observer_can_stop_the_search() {
    let c = &builtin()[39];
    let (p, s) = Puzzle::from_board(&c.board);
    let out = best_first(
        &p,
        s,
        Strategy::AStar,
        Limits::default(),
        None,
        &mut StopAfter(50),
    );
    assert!(matches!(out, Outcome::Stopped), "{out:?}");
}

#[test]
fn memory_limit_is_reported() {
    let c = &builtin()[39];
    let (p, s) = Puzzle::from_board(&c.board);
    let out = best_first(
        &p,
        s,
        Strategy::AStar,
        Limits { max_states: 100 },
        None,
        &mut Quiet,
    );
    assert!(matches!(out, Outcome::MemoryLimit), "{out:?}");
}
