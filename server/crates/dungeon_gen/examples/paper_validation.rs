//! Reproduce Table 2–5 from the paper across all 8 parameter configurations.
//! N_RUNS executions each; reports mean ± std-dev.

use dungeon_gen::evolution::{generate, DungeonConfig};
use dungeon_gen::fitness::linearity;
use dungeon_gen::pathfinding::{astar_locks_opened, dfs_avg_rooms_visited, find_goal};
use dungeon_gen::rng::Rng;
use std::collections::HashSet;

const CONFIGS: [(u32, u32, u32, f64); 7] = [
    (15, 3, 2, 2.0),
    (20, 4, 4, 1.0),
    (20, 4, 4, 2.0),
    (25, 8, 8, 1.0),
    (30, 4, 4, 2.0),
    (30, 6, 6, 1.5),
    (100, 20, 20, 1.5),
    // (500, 100, 100, 1.5) -- omitted; takes many minutes at 100 runs
];

const N_RUNS: usize = 10;

fn mean_std(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let m = v.iter().sum::<f64>() / n;
    let var = v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n;
    (m, var.sqrt())
}

fn main() {
    println!(
        "{:<22} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "Config (ur,uk,ul,lin)", "dr", "dk", "dl", "d_lin", "Δl", "Δr", "f"
    );
    println!("{}", "-".repeat(108));

    for &(ur, uk, ul, ulin) in &CONFIGS {
        let mut rooms_v = vec![];
        let mut keys_v = vec![];
        let mut locks_v = vec![];
        let mut lin_v = vec![];
        let mut dl_v = vec![];
        let mut dr_v = vec![];
        let mut fit_v = vec![];

        for run in 0..N_RUNS {
            let cfg = DungeonConfig::new(ur, uk, ul, ulin, run as u64 * 1000 + 1);
            let d = generate(&cfg);

            let placed: HashSet<usize> = d.grid.placed_room_ids().into_iter().collect();
            let dr = placed.len() as f64;
            let dk = d.tree.placed_key_count(&d.grid) as f64;
            let dl = d.tree.placed_lock_count(&d.grid) as f64;
            let dlin = linearity(&d.tree, &placed);

            let mut rng = Rng::new(run as u64 * 999 + 7);
            let goal = find_goal(&d.tree);
            let delta_l = (dl
                - astar_locks_opened(&d.tree, &d.grid, d.tree.root, goal).unwrap_or(0) as f64)
                .max(0.0);
            let delta_r =
                (dr - dfs_avg_rooms_visited(&d.tree, &d.grid, d.tree.root, &mut rng)).max(0.0);

            rooms_v.push(dr);
            keys_v.push(dk);
            locks_v.push(dl);
            lin_v.push(dlin);
            dl_v.push(delta_l);
            dr_v.push(delta_r);
            fit_v.push(d.fitness.total);
        }

        let (mr, sr) = mean_std(&rooms_v);
        let (mk, sk) = mean_std(&keys_v);
        let (ml, sl) = mean_std(&locks_v);
        let (mlin, slin) = mean_std(&lin_v);
        let (mdl, sdl) = mean_std(&dl_v);
        let (mdr, sdr) = mean_std(&dr_v);
        let (mf, sf) = mean_std(&fit_v);

        let label = format!("({},{},{},{:.1})", ur, uk, ul, ulin);
        println!("{:<22} {:>5.1}±{:<4.1} {:>4.1}±{:<3.1} {:>4.1}±{:<3.1} {:>5.2}±{:<4.2} {:>4.2}±{:<4.2} {:>5.2}±{:<5.2} {:>6.2}±{:<5.2}",
            label,
            mr, sr, mk, sk, ml, sl, mlin, slin, mdl, sdl, mdr, sdr, mf, sf);
    }
}
