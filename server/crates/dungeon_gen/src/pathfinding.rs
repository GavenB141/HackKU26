//! Pathfinding algorithms used in the fitness function (paper §4.2), extended
//! for switch puzzles, enemies, and the boss.
//!
//! ## Traversal state
//!
//! Each state carries:
//! * `key_mask`    — bitmask of collected key IDs.
//! * `switch_mask` — bitmask of activated switch IDs (toggle on revisit).
//! * `cleared`     — bitmask of room IDs whose enemy group has been defeated.
//!   (Room IDs ≤ 63 are tracked; larger IDs are always considered clearable.)
//!
//! ## Passability rules (in order)
//!
//! 1. **Enemy room**: impassable until the enemies are cleared — modelled as
//!    "clearing happens automatically on first entry", so the room is always
//!    enterable but we must record it as cleared.
//! 2. **Locked room** (`RoomKind::Locked`): requires the matching key.
//! 3. **SwitchDoor room**: requires the parity of linked switches to be odd.
//! 4. **Boss room** (goal): treated like an enemy room — the boss is "cleared"
//!    when the player arrives.  The pathfinder simply needs to reach it.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::content::{RoomContent, SwitchState};
use crate::grid::DungeonGrid;
use crate::rng::Rng;
use crate::room::RoomKind;
use crate::tree::DungeonTree;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Collect all switch states from the content layer.
fn all_switches(content: &[RoomContent]) -> Vec<SwitchState> {
    content.iter().filter_map(|c| c.switch.clone()).collect()
}

/// Is `room_id` passable given the current key mask, switch mask, and cleared set?
///
/// Does *not* check enemy presence — that is handled by the caller because
/// enemies are cleared on entry (not a precondition for entry).
fn is_passable(
    room_id: usize,
    tree: &DungeonTree,
    content: &[RoomContent],
    key_mask: u64,
    switch_mask: u64,
    cleared_mask: u64,
) -> bool {
    // Key-locked door
    if let RoomKind::Locked { key_id } = tree.rooms[room_id].kind {
        if key_id >= 64 || (key_mask >> key_id) & 1 == 0 {
            return false;
        }
    }

    // Switch-door: blocked when content carries a switch_door annotation,
    // regardless of RoomKind (structural SwitchDoor evolved by EA, or content-
    // layer door placed post-evolution on a Normal room — both are handled here).
    // When content is empty (EA evolution phase) all switch-doors are open.
    if let Some(door) = content.get(room_id).and_then(|c| c.switch_door.as_ref()) {
        let switches = all_switches(content);
        if !door.is_open(switch_mask, &switches) {
            return false;
        }
    } else if content.is_empty() && tree.rooms[room_id].kind.is_switch_door() {
        // Pre-content phase: structural SwitchDoor treated as open
    }

    // Enemy room: always enterable (enemies are cleared on arrival)
    // Boss room: always enterable (goal)

    // Cleared enemies don't block re-entry
    let _ = cleared_mask;

    true
}

// ── A* ───────────────────────────────────────────────────────────────────────

/// Run A\* from `start` to `goal`, returning the number of key-locks opened.
///
/// The state space includes key mask + switch mask so the planner can reason
/// about which switches to hit to open switch-doors on the critical path.
/// For tractability both masks are limited to 64 bits (≤64 keys / switches).
///
/// `content` may be `None` for plain key-lock dungeons (no switches).
pub fn astar_locks_opened(
    tree: &DungeonTree,
    grid: &DungeonGrid,
    start: usize,
    goal: usize,
) -> Option<u32> {
    astar_locks_opened_with_content(tree, grid, start, goal, &[])
}

pub fn astar_locks_opened_with_content(
    tree: &DungeonTree,
    grid: &DungeonGrid,
    start: usize,
    goal: usize,
    content: &[RoomContent],
) -> Option<u32> {
    let start_pos = grid.pos_of(start)?;
    let goal_pos = grid.pos_of(goal)?;

    // State: (f, g, room_id, key_mask, switch_mask, locks_opened)
    type State = (Reverse<u32>, u32, usize, u64, u64, u32);
    let mut open: BinaryHeap<State> = BinaryHeap::new();
    // best[(room, key_mask, switch_mask)] = best g seen
    let mut best: HashMap<(usize, u64, u64), u32> = HashMap::new();

    let h = |pos: (i32, i32)| -> u32 {
        (pos.0 - goal_pos.0).unsigned_abs() + (pos.1 - goal_pos.1).unsigned_abs()
    };

    open.push((Reverse(h(start_pos)), 0, start, 0, 0, 0));

    while let Some((_, g, room_id, key_mask, sw_mask, locks)) = open.pop() {
        if room_id == goal {
            return Some(locks);
        }

        let sk = (room_id, key_mask, sw_mask);
        if best.get(&sk).is_some_and(|&prev| prev <= g) {
            continue;
        }
        best.insert(sk, g);

        // Collect items in this room
        let mut km = key_mask;
        let mut swm = sw_mask;

        if let RoomKind::Key { key_id } = tree.rooms[room_id].kind {
            if key_id < 64 {
                km |= 1u64 << key_id;
            }
        }
        // Toggle switch if present
        if tree.rooms[room_id].kind.is_switch() {
            if let Some(sw) = content.get(room_id).and_then(|c| c.switch.as_ref()) {
                if sw.switch_id < 64 {
                    swm ^= 1u64 << sw.switch_id;
                }
            }
        }

        for &nb in &grid.neighbours(room_id) {
            // Goal room is always enterable (boss/switch-door don't block final entry)
            if nb != goal && !is_passable(nb, tree, content, km, swm, 0) {
                continue;
            }

            let mut new_locks = locks;
            if tree.rooms[nb].kind.is_locked() {
                new_locks += 1;
            }

            let new_g = g + 1;
            let np = grid.pos_of(nb).unwrap_or(goal_pos);
            let nsk = (nb, km, swm);
            if best.get(&nsk).is_none_or(|&prev| prev > new_g) {
                open.push((Reverse(new_g + h(np)), new_g, nb, km, swm, new_locks));
            }
        }
    }
    None
}

// ── DFS ──────────────────────────────────────────────────────────────────────

fn dfs_rooms_visited(
    tree: &DungeonTree,
    grid: &DungeonGrid,
    start: usize,
    content: &[RoomContent],
    rng: &mut Rng,
) -> usize {
    let mut visited: HashSet<usize> = HashSet::new();
    let mut key_mask: u64 = 0;
    let mut sw_mask: u64 = 0;
    let mut stack: Vec<usize> = vec![start];
    let mut closed: HashSet<usize> = HashSet::new();

    while let Some(&top) = stack.last() {
        visited.insert(top);

        // Collect key
        if let RoomKind::Key { key_id } = tree.rooms[top].kind {
            if key_id < 64 {
                let was_new = (key_mask >> key_id) & 1 == 0;
                key_mask |= 1u64 << key_id;
                if was_new {
                    // Re-open parents of newly unlocked rooms
                    for room in &tree.rooms {
                        if let RoomKind::Locked { key_id: lid } = room.kind {
                            if lid == key_id {
                                if let Some(parent) = tree.parent_of(room.id) {
                                    closed.remove(&parent);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Toggle switch
        if tree.rooms[top].kind.is_switch() {
            if let Some(sw) = content.get(top).and_then(|c| c.switch.as_ref()) {
                if sw.switch_id < 64 {
                    let was_closed = (sw_mask >> sw.switch_id) & 1 == 0;
                    sw_mask ^= 1u64 << sw.switch_id;
                    if was_closed {
                        // Re-open parents of switch-doors whose parity flipped open
                        let switches = all_switches(content);
                        for room in &tree.rooms {
                            if room.kind.is_switch_door() {
                                if let Some(door) =
                                    content.get(room.id).and_then(|c| c.switch_door.as_ref())
                                {
                                    if door.is_open(sw_mask, &switches) {
                                        if let Some(parent) = tree.parent_of(room.id) {
                                            closed.remove(&parent);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Find an unvisited, accessible neighbour
        let mut neighbours = grid.neighbours(top);
        rng.shuffle(&mut neighbours);

        // Goal room is always enterable (boss/final barrier never blocks arrival)
        let goal_id = find_goal(tree);
        let next = neighbours
            .iter()
            .find(|&&nid| {
                if closed.contains(&nid) || visited.contains(&nid) {
                    return false;
                }
                nid == goal_id || is_passable(nid, tree, content, key_mask, sw_mask, 0)
            })
            .copied();

        match next {
            Some(nid) => stack.push(nid),
            None => {
                closed.insert(top);
                stack.pop();
            }
        }
    }

    visited.len()
}

/// Run DFS three times and return the average rooms visited (paper §4.2).
pub fn dfs_avg_rooms_visited(
    tree: &DungeonTree,
    grid: &DungeonGrid,
    start: usize,
    rng: &mut Rng,
) -> f64 {
    dfs_avg_rooms_visited_with_content(tree, grid, start, &[], rng)
}

pub fn dfs_avg_rooms_visited_with_content(
    tree: &DungeonTree,
    grid: &DungeonGrid,
    start: usize,
    content: &[RoomContent],
    rng: &mut Rng,
) -> f64 {
    let total: usize = (0..3)
        .map(|_| dfs_rooms_visited(tree, grid, start, content, rng))
        .sum();
    total as f64 / 3.0
}

// ── Goal finder ──────────────────────────────────────────────────────────────

/// Find the goal room among *placed* rooms: last Locked or SwitchDoor in BFS
/// order that was actually placed in the grid, else the deepest placed leaf.
pub fn find_goal(tree: &DungeonTree) -> usize {
    find_goal_full(tree, None, &[])
}

pub fn find_goal_in_grid(tree: &DungeonTree, grid: Option<&crate::grid::DungeonGrid>) -> usize {
    find_goal_full(tree, grid, &[])
}

/// Full goal-finder: prefers the explicit exit room (tagged `is_exit` in
/// `content`) when content is populated.  Falls back to the last structural
/// barrier among placed rooms, then to the deepest placed leaf.
pub fn find_goal_full(
    tree: &DungeonTree,
    grid: Option<&crate::grid::DungeonGrid>,
    content: &[crate::content::RoomContent],
) -> usize {
    let placed: Option<HashSet<usize>> = grid.map(|g| g.placed_room_ids().into_iter().collect());
    let is_placed = |id: usize| -> bool { placed.as_ref().is_none_or(|p| p.contains(&id)) };

    // When content is populated, the exit room is the true goal
    if !content.is_empty() {
        if let Some(exit_id) = content
            .iter()
            .enumerate()
            .find(|(id, c)| c.is_exit && is_placed(*id))
            .map(|(id, _)| id)
        {
            return exit_id;
        }
    }

    // Fallback: last structural barrier in placed rooms
    let order = tree.bfs_order();
    if let Some(&id) = order.iter().rev().find(|&&id| {
        is_placed(id)
            && matches!(
                tree.rooms[id].kind,
                RoomKind::Locked { .. } | RoomKind::SwitchDoor
            )
    }) {
        return id;
    }
    order
        .into_iter()
        .filter(|&id| is_placed(id) && tree.rooms[id].children.iter().all(|&c| !is_placed(c)))
        .max_by_key(|&id| tree.rooms[id].depth)
        .unwrap_or(tree.root)
}

// ── Critical path ─────────────────────────────────────────────────────────────

/// Return the set of room IDs that lie on *any* shortest BFS path from `start`
/// to `goal`, ignoring switch-door and enemy content (structural locks are
/// respected so the set is accurate for the tree's key/lock logic).
///
/// Used by content placement to avoid putting switch-doors on rooms that the
/// player *must* pass through.
pub fn critical_path_rooms(
    tree: &DungeonTree,
    grid: &DungeonGrid,
    start: usize,
    goal: usize,
) -> std::collections::HashSet<usize> {
    use std::collections::VecDeque;

    // Forward BFS: find shortest distances from start (ignoring content locks)
    let mut dist_from_start: HashMap<usize, u32> = HashMap::new();
    let mut queue: VecDeque<usize> = VecDeque::new();
    dist_from_start.insert(start, 0);
    queue.push_back(start);
    while let Some(id) = queue.pop_front() {
        let d = dist_from_start[&id];
        for &nb in &grid.neighbours(id) {
            if dist_from_start.contains_key(&nb) {
                continue;
            }
            // Only respect structural key-locks (no switch-door or content here)
            if let RoomKind::Locked { key_id } = tree.rooms[nb].kind {
                // Treat locked as passable for path-coverage purposes
                let _ = key_id;
            }
            dist_from_start.insert(nb, d + 1);
            queue.push_back(nb);
        }
    }

    // Backward BFS: find shortest distances from goal
    let mut dist_from_goal: HashMap<usize, u32> = HashMap::new();
    let mut queue2: VecDeque<usize> = VecDeque::new();
    dist_from_goal.insert(goal, 0);
    queue2.push_back(goal);
    while let Some(id) = queue2.pop_front() {
        let d = dist_from_goal[&id];
        for &nb in &grid.neighbours(id) {
            if dist_from_goal.contains_key(&nb) {
                continue;
            }
            dist_from_goal.insert(nb, d + 1);
            queue2.push_back(nb);
        }
    }

    // A room is on a shortest path iff
    //   dist_start[r] + dist_goal[r] == dist_start[goal]
    let total = match dist_from_start.get(&goal) {
        Some(&d) => d,
        None => return HashSet::new(), // goal unreachable
    };

    dist_from_start
        .keys()
        .filter(|&&id| {
            dist_from_start.get(&id).copied().unwrap_or(u32::MAX)
                + dist_from_goal.get(&id).copied().unwrap_or(u32::MAX)
                == total
        })
        .copied()
        .collect()
}
