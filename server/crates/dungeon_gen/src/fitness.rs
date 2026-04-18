//! Fitness function (paper §4.2, Eqs. 1-4), extended for switch puzzles.
//!
//! The fitness formula is extended to:
//! ```text
//! f = 2(|ur-dr| + |uk-dk| + |ul-dl| + |us-ds| + |u_lin-d_lin|) + Δl + Δr
//! ```
//! where `us`/`ds` are desired/actual switch-room counts (switch + door pairs
//! counted together as "switch pairs").

use crate::content::RoomContent;
use crate::grid::DungeonGrid;
use crate::pathfinding::{
    astar_locks_opened_with_content, dfs_avg_rooms_visited_with_content, find_goal_full,
};
use crate::rng::Rng;
use crate::tree::DungeonTree;

/// User-supplied targets.
#[derive(Clone, Debug)]
pub struct FitnessTargets {
    pub rooms: f64,
    pub keys: f64,
    pub locks: f64,
    /// Desired number of switch rooms (each switch + its door = 1 pair counted
    /// as 2 special rooms; set to 0 to disable switch evolution).
    pub switches: f64,
    pub linearity: f64,
}

/// Fitness breakdown.
#[derive(Clone, Debug)]
pub struct FitnessResult {
    pub total: f64,
    pub delta_locks: f64,
    pub delta_rooms: f64,
    pub linearity: f64,
}

/// Linearity coefficient (paper Eq. 4), placed-rooms only.
pub fn linearity(tree: &DungeonTree, placed: &std::collections::HashSet<usize>) -> f64 {
    let dr = placed.len() as f64;
    let inner = tree
        .rooms
        .iter()
        .filter(|r| placed.contains(&r.id))
        .filter(|r| r.children.iter().any(|c| placed.contains(c)))
        .count() as f64;
    if inner <= 0.0 {
        return 1.0;
    }
    (dr - 1.0) / inner
}

/// Evaluate fitness.  Pass an empty slice for `content` during evolution;
/// after content placement pass the filled content vec for a final score.
pub fn evaluate(
    tree: &DungeonTree,
    grid: &DungeonGrid,
    targets: &FitnessTargets,
    content: &[RoomContent],
    rng: &mut Rng,
) -> FitnessResult {
    use std::collections::HashSet;
    let placed: HashSet<usize> = grid.placed_room_ids().into_iter().collect();

    let dr = placed.len() as f64;
    let dk = tree
        .rooms
        .iter()
        .filter(|r| placed.contains(&r.id) && r.kind.is_key())
        .count() as f64;
    let dl = tree
        .rooms
        .iter()
        .filter(|r| placed.contains(&r.id) && r.kind.is_locked())
        .count() as f64;
    // Count switch *pairs*: min(switches, switch_doors) placed
    let dsw = tree
        .rooms
        .iter()
        .filter(|r| placed.contains(&r.id) && r.kind.is_switch())
        .count() as f64;
    let dsd = tree
        .rooms
        .iter()
        .filter(|r| placed.contains(&r.id) && r.kind.is_switch_door())
        .count() as f64;
    let ds = dsw.min(dsd); // paired count
    let d_lin = linearity(tree, &placed);

    let start = tree.root;
    // Use find_goal_full so that when content is populated, the exit room
    // becomes the pathfinding target (requiring the player to clear the boss).
    let goal = find_goal_full(tree, Some(grid), content);

    let d_astar =
        astar_locks_opened_with_content(tree, grid, start, goal, content).unwrap_or(0) as f64;
    let d_dfs = dfs_avg_rooms_visited_with_content(tree, grid, start, content, rng);

    let delta_l = (dl - d_astar).max(0.0);
    let delta_r = (dr - d_dfs).max(0.0);

    let total = 2.0
        * ((targets.rooms - dr).abs()
            + (targets.keys - dk).abs()
            + (targets.locks - dl).abs()
            + (targets.switches - ds).abs()
            + (targets.linearity - d_lin).abs())
        + delta_l
        + delta_r;

    FitnessResult {
        total,
        delta_locks: delta_l,
        delta_rooms: delta_r,
        linearity: d_lin,
    }
}
