//! Offline level generator.
//!
//! ```text
//! cargo run --release -p rush-core --bin levelgen -- [seconds] [seed] > crates/rush-core/assets/levels.txt
//! ```
//!
//! Runs parallel hill-climbs for `seconds` (default 90), then picks 40
//! original challenges: 10 per tier, ordered by par (Constitution IV).

use rush_core::generator::{climb, Candidate, Rng};
use rush_core::level::write_levels;
use rush_core::solver::{self, Cluster};
use rush_core::{Board, Challenge, Tier};
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const BEGINNER: [usize; 10] = [3, 4, 5, 5, 6, 6, 7, 7, 8, 8];
const INTERMEDIATE: [usize; 10] = [10, 11, 12, 12, 13, 14, 15, 15, 16, 17];
const ADVANCED: [usize; 10] = [18, 19, 20, 21, 22, 23, 24, 25, 26, 27];
/// The 10th Expert challenge is the hardest layout found.
const EXPERT: [usize; 9] = [29, 31, 33, 35, 37, 39, 41, 43, 45];

/// Identity of a layout (which vehicle sits in which lane) — challenges must
/// use distinct layouts so no two feel like the same puzzle.
fn layout_key(b: &Board) -> String {
    b.vehicles()
        .iter()
        .map(|v| format!("{}{:?}{}", v.id(), v.orientation(), v.lane()))
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seconds: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(90);
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let deadline = Instant::now() + Duration::from_secs(seconds);

    eprintln!("levelgen: {threads} threads, {seconds}s, seed {seed}");
    let pool: Arc<Mutex<Vec<Candidate>>> = Arc::default();
    let handles: Vec<_> = (0..threads)
        .map(|t| {
            let pool = Arc::clone(&pool);
            std::thread::spawn(move || {
                let mut rng = Rng::new(seed.wrapping_add(t as u64 * 7919));
                while Instant::now() < deadline {
                    let steps = 150 + rng.below(450);
                    let found = climb(&mut rng, steps);
                    pool.lock().unwrap().extend(found);
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let mut pool = Arc::try_unwrap(pool).unwrap().into_inner().unwrap();

    // Deduplicate by layout, keeping the hardest.
    pool.sort_by_key(|c| std::cmp::Reverse(c.par));
    let mut seen = HashSet::new();
    pool.retain(|c| seen.insert(layout_key(&c.board)));

    let mut hist: BTreeMap<usize, usize> = BTreeMap::new();
    for c in &pool {
        *hist.entry(c.par).or_default() += 1;
    }
    eprintln!("pool: {} layouts; par histogram {:?}", pool.len(), hist);

    let mut rng = Rng::new(seed ^ 0xDEAD_BEEF);
    let mut used: HashSet<String> = HashSet::new();
    let mut challenges: Vec<Challenge> = Vec::new();

    // The finale: the hardest layout found.
    let finale = pool[0].clone();
    assert!(
        finale.par > EXPERT[8],
        "hardest found is par {}; run longer",
        finale.par
    );
    used.insert(layout_key(&finale.board));

    let mut pick = |target: usize, fewest_vehicles: bool| -> Board {
        let mut exact: Vec<&Candidate> = pool
            .iter()
            .filter(|c| c.par == target && !used.contains(&layout_key(&c.board)))
            .collect();
        let board = if !exact.is_empty() {
            if fewest_vehicles {
                exact.sort_by_key(|c| c.board.vehicles().len());
                let min = exact[0].board.vehicles().len();
                exact.retain(|c| c.board.vehicles().len() == min);
            }
            exact[rng.below(exact.len())].board.clone()
        } else {
            // Take a position of par `target` from a harder unused layout.
            let src = pool
                .iter()
                .rev()
                .find(|c| c.par > target && !used.contains(&layout_key(&c.board)))
                .expect("pool has a harder layout");
            let options = Cluster::explore(&src.board).boards_at(target);
            options[rng.below(options.len())].clone()
        };
        used.insert(layout_key(&board));
        board
    };

    let tiers: [(Tier, &[usize]); 4] = [
        (Tier::Beginner, &BEGINNER),
        (Tier::Intermediate, &INTERMEDIATE),
        (Tier::Advanced, &ADVANCED),
        (Tier::Expert, &EXPERT),
    ];
    for (tier, targets) in tiers {
        for &t in targets {
            let board = pick(t, tier == Tier::Beginner);
            challenges.push(Challenge {
                number: 0,
                tier,
                board,
                par: t as u32,
            });
        }
    }
    challenges.push(Challenge {
        number: 0,
        tier: Tier::Expert,
        board: finale.board,
        par: finale.par as u32,
    });
    for (i, c) in challenges.iter_mut().enumerate() {
        c.number = i as u32 + 1;
        // Constitution IV: prove every par again with the plain solver.
        assert_eq!(
            solver::par(&c.board),
            Some(c.par as usize),
            "challenge {}",
            c.number
        );
    }

    println!("# Rush Hour — 40 original challenges.");
    println!("# Generated by `levelgen {seconds} {seed}`; every par is proven optimal by the BFS solver.");
    println!("# Format: specs/001-rush-hour-game/contracts/level-format.md");
    println!();
    print!("{}", write_levels(&challenges));
    eprintln!("pars: {:?}", challenges.iter().map(|c| c.par).collect::<Vec<_>>());
}
