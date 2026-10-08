//! Quick benchmark of the Bot Lab search: `cargo run --release -p rush-core --example labprobe`
use rush_core::lab::generate;
use rush_core::search::{anytime, Limits, Observer, Strategy};
use rush_core::Move;
use std::time::{Duration, Instant};

struct Timed {
    start: Instant,
    budget: Duration,
    expanded: u64,
}
impl Observer for Timed {
    fn expanded(&mut self, _s: u128, _d: u32, n: u64) -> bool {
        self.expanded += 1;
        n % 1024 != 0 || self.start.elapsed() < self.budget
    }
    fn solution(&mut self, st: Strategy, path: &[Move], optimal: bool) {
        println!(
            "    {:>6.2}s {:<14} {:>3} moves{}",
            self.start.elapsed().as_secs_f32(),
            st.name(),
            path.len(),
            if optimal { "  (optimal)" } else { "" }
        );
    }
}

fn main() {
    let budget: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    for size in [6u8, 8, 10] {
        for seed in 1..=4 {
            let t0 = Instant::now();
            let (p, s) = generate(size, seed);
            println!("  generated in {:.2}s", t0.elapsed().as_secs_f32());
            println!("size {size} seed {seed}: {} vehicles", p.pieces().len());
            let mut o = Timed {
                start: Instant::now(),
                budget: Duration::from_secs(budget),
                expanded: 0,
            };
            let r = anytime(&p, s, Limits::default(), &mut o);
            println!(
                "    done {:.2}s, {} expanded, best {:?}, optimal {}, mem {}",
                o.start.elapsed().as_secs_f32(),
                o.expanded,
                r.best.map(|b| b.len()),
                r.optimal,
                r.memory_limited
            );
        }
    }
}
