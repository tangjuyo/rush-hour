//! Acceptance tests for the official movement rules (spec US1, FR-001..FR-007).

use rush_core::level::parse_board;
use rush_core::{Board, Move, Orientation};

fn board(s: &str) -> Board {
    parse_board(s).expect("valid test board")
}

fn idx(b: &Board, id: char) -> usize {
    b.vehicles().iter().position(|v| v.id() == id).unwrap()
}

#[test]
fn parse_infers_kind_orientation_and_position() {
    let b = board(
        "\
..BBC.
....C.
AA....
O.....
O.....
O.....",
    );
    assert_eq!(b.vehicles().len(), 4);
    let a = b.target();
    assert_eq!((a.id(), a.row(), a.col(), a.length()), ('A', 2, 0, 2));
    assert_eq!(a.orientation(), Orientation::Horizontal);
    let c = &b.vehicles()[idx(&b, 'C')];
    assert_eq!(c.orientation(), Orientation::Vertical);
    assert_eq!((c.row(), c.col(), c.length()), (0, 4, 2));
    let o = &b.vehicles()[idx(&b, 'O')];
    assert_eq!((o.orientation(), o.length()), (Orientation::Vertical, 3));
}

#[test]
fn car_letters_must_be_two_long_and_trucks_three_long() {
    // C drawn 3 cells long but C is a car.
    assert!(parse_board("..C...\n..C...\nAAC...\n......\n......\n......").is_err());
    // O drawn 2 cells long but O is a truck.
    assert!(parse_board("OO....\n......\nAA....\n......\n......\n......").is_err());
}

#[test]
fn target_must_be_horizontal_on_exit_row() {
    assert!(parse_board("AA....\n......\n......\n......\n......\n......").is_err());
    assert!(parse_board("A.....\nA.....\n......\n......\n......\n......").is_err());
    assert!(parse_board("......\n......\nBB....\n......\n......\n......").is_err());
}

#[test]
fn slides_only_along_own_axis_and_stops_at_obstacles() {
    let b = board(
        "\
......
......
AA.B..
...B..
......
......",
    );
    // Red car can move 1 right (B blocks at col 3), 0 left.
    assert_eq!(b.slide_range(0), (0, 1));
    // Vertical B: rows 2-3; can go up 2, down 2.
    assert_eq!(b.slide_range(idx(&b, 'B')), (-2, 2));
}

#[test]
fn no_jumping_over_vehicles() {
    let b = board(
        "\
......
......
AA.B..
...B..
......
......",
    );
    assert!(!b.is_legal(Move { vehicle: 0, delta: 3 }));
    assert!(b.applied(Move { vehicle: 0, delta: 3 }).is_err());
    assert!(b.is_legal(Move { vehicle: 0, delta: 1 }));
}

#[test]
fn zero_and_out_of_bounds_moves_are_illegal() {
    let b = board("......\n......\nAA....\n......\n......\n......");
    assert!(!b.is_legal(Move { vehicle: 0, delta: 0 }));
    assert!(!b.is_legal(Move {
        vehicle: 0,
        delta: -1
    }));
    assert!(!b.is_legal(Move { vehicle: 0, delta: 5 }));
    assert!(!b.is_legal(Move { vehicle: 7, delta: 1 }));
}

#[test]
fn one_slide_of_any_distance_is_one_legal_move() {
    let b = board("......\n......\nAA....\n......\n......\n......");
    let moves = b.legal_moves();
    // Red car alone: may slide 1, 2, 3 or 4 cells right = 4 distinct single moves.
    assert_eq!(moves.len(), 4);
    let after = b.applied(Move { vehicle: 0, delta: 4 }).unwrap();
    assert!(after.is_solved());
}

#[test]
fn solved_only_when_red_car_reaches_exit() {
    let b = board("......\n......\n...AA.\n......\n......\n......");
    assert!(!b.is_solved());
    let b = board("......\n......\n....AA\n......\n......\n......");
    assert!(b.is_solved());
}

#[test]
fn display_round_trips() {
    let s = "..BBC.\n....C.\nAA....\nO.....\nO.....\nO.....";
    assert_eq!(board(s).to_string(), s);
}
