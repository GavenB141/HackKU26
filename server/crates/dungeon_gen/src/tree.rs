//! The **genotype**: a tree of [`Room`] nodes.
//!
//! Implements the tree-structure generation algorithm (paper Fig. 1), the
//! constrained crossover (Fig. 6), and both mutation operators (§4.1).

use std::collections::{HashSet, VecDeque};

use crate::rng::Rng;
use crate::room::{Direction, Room, RoomKind};

// ── Paper hyper-parameters (§6) ────────────────────────────────────────────
const MAX_DEPTH: u32 = 9;
const MAX_CHILDREN: usize = 3;
const PROB_HAS_CHILD: f64 = 0.7; // base probability a room will have children
const PROB_KEY: f64 = 0.15; // 15% key room
const PROB_NORMAL: f64 = 0.62; // 62% normal room
const PROB_SWITCH: f64 = 0.04; //  4% switch room  (new)
                               // PROB_KEY + PROB_NORMAL + PROB_SWITCH + remainder(=locked) = 1.0
                               // 0.15 + 0.62 + 0.04 = 0.81 → 0.19 locked  (slightly more than before)

// ── DungeonTree ────────────────────────────────────────────────────────────

/// Complete genotype: a flat arena of rooms addressed by index, plus a root.
#[derive(Clone, Debug)]
pub struct DungeonTree {
    pub rooms: Vec<Room>,
    pub root: usize, // always 0
}

impl DungeonTree {
    // ── Construction (Fig. 1) ────────────────────────────────────────────

    /// Generate a random dungeon tree (paper Algorithm 1).
    ///
    /// The breadth-first construction ensures that locked rooms are always
    /// reachable (a key must appear *before* the lock in BFS order) and that
    /// the tree itself is the sole guarantee of feasibility.
    pub fn generate(rng: &mut Rng) -> Self {
        let mut rooms: Vec<Room> = Vec::new();
        let mut to_visit: VecDeque<usize> = VecDeque::new();
        let mut next_key_id: u32 = 0;

        // Root = spawn point
        let root = Room::new(0, RoomKind::Normal, None, 0);
        rooms.push(root);
        to_visit.push_back(0);

        while let Some(parent_id) = to_visit.pop_front() {
            let actual_depth = rooms[parent_id].depth;

            if actual_depth > MAX_DEPTH {
                to_visit.clear();
                break;
            }

            // Probability of having children decreases with depth (paper line 9)
            let prob_child = PROB_HAS_CHILD * (1.0 - actual_depth as f64 / 10.0);
            let prob = rng.next_f64();

            if prob > prob_child {
                continue;
            }

            // Number of children: 1..=MAX_CHILDREN
            let n_children = 1 + rng.next_usize(MAX_CHILDREN);

            // Pick distinct directions for each child
            let mut dirs = [Direction::Right, Direction::Left, Direction::Down];
            rng.shuffle(&mut dirs);
            let chosen_dirs = &dirs[..n_children.min(3)];

            for &dir in chosen_dirs {
                // Decide room type
                let p = rng.next_f64();
                let kind = if p < PROB_KEY {
                    let id = next_key_id;
                    next_key_id += 1;
                    RoomKind::Key { key_id: id }
                } else if p < PROB_KEY + PROB_NORMAL {
                    RoomKind::Normal
                } else if p < PROB_KEY + PROB_NORMAL + PROB_SWITCH {
                    RoomKind::Switch
                } else {
                    // Locked – key_id will be fixed in a pass below
                    RoomKind::Locked { key_id: 0 }
                };

                let child_id = rooms.len();
                let child = Room::new(child_id, kind, Some(dir), actual_depth + 1);
                rooms.push(child);
                rooms[parent_id].children.push(child_id);
                to_visit.push_back(child_id);
            }
        }

        let mut tree = DungeonTree { rooms, root: 0 };
        tree.assign_lock_ids(rng);
        tree
    }

    /// Assign valid key IDs to all locked rooms.
    ///
    /// A `Locked` room's key must be in a `Key` room that appears *before* it
    /// in BFS order (so the player can collect it first). We iterate BFS,
    /// collecting available key IDs, and when we encounter a locked room we
    /// consume one of the available IDs at random.
    pub fn assign_lock_ids(&mut self, rng: &mut Rng) {
        let order = self.bfs_order();
        let mut available_keys: Vec<u32> = Vec::new();
        let mut next_id: u32 = 0;

        for &id in &order {
            match self.rooms[id].kind {
                RoomKind::Key { .. } => {
                    self.rooms[id].kind = RoomKind::Key { key_id: next_id };
                    available_keys.push(next_id);
                    next_id += 1;
                }
                RoomKind::Locked { .. } => {
                    if available_keys.is_empty() {
                        // No key available → demote to Normal
                        self.rooms[id].kind = RoomKind::Normal;
                    } else {
                        let idx = rng.next_usize(available_keys.len());
                        let kid = available_keys.remove(idx);
                        self.rooms[id].kind = RoomKind::Locked { key_id: kid };
                    }
                }
                RoomKind::Normal | RoomKind::Switch | RoomKind::SwitchDoor => {}
            }
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    pub fn key_count(&self) -> usize {
        self.rooms.iter().filter(|r| r.kind.is_key()).count()
    }

    pub fn lock_count(&self) -> usize {
        self.rooms.iter().filter(|r| r.kind.is_locked()).count()
    }

    pub fn switch_count(&self) -> usize {
        self.rooms.iter().filter(|r| r.kind.is_switch()).count()
    }

    pub fn switch_door_count(&self) -> usize {
        self.rooms
            .iter()
            .filter(|r| r.kind.is_switch_door())
            .count()
    }

    pub fn placed_switch_count(&self, grid: &crate::grid::DungeonGrid) -> usize {
        let placed: std::collections::HashSet<usize> = grid.placed_room_ids().into_iter().collect();
        self.rooms
            .iter()
            .filter(|r| placed.contains(&r.id) && r.kind.is_switch())
            .count()
    }

    pub fn placed_switch_door_count(&self, grid: &crate::grid::DungeonGrid) -> usize {
        let placed: std::collections::HashSet<usize> = grid.placed_room_ids().into_iter().collect();
        self.rooms
            .iter()
            .filter(|r| placed.contains(&r.id) && r.kind.is_switch_door())
            .count()
    }

    /// BFS traversal order (used throughout the paper).
    pub fn bfs_order(&self) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.rooms.len());
        let mut queue = VecDeque::new();
        queue.push_back(self.root);
        while let Some(id) = queue.pop_front() {
            order.push(id);
            for &child in &self.rooms[id].children {
                queue.push_back(child);
            }
        }
        order
    }

    /// Find the parent of `node_id` by walking the tree.
    pub fn parent_of(&self, node_id: usize) -> Option<usize> {
        if node_id == self.root {
            return None;
        }
        for room in &self.rooms {
            if room.children.contains(&node_id) {
                return Some(room.id);
            }
        }
        None
    }

    /// All node IDs in the subtree rooted at `node_id` (inclusive).
    pub fn subtree_ids(&self, node_id: usize) -> Vec<usize> {
        let mut ids = Vec::new();
        let mut stack = vec![node_id];
        while let Some(id) = stack.pop() {
            ids.push(id);
            for &child in &self.rooms[id].children {
                stack.push(child);
            }
        }
        ids
    }

    /// Count of special rooms (Key + Locked) in the subtree at `node_id`.
    pub fn special_count_in_subtree(&self, node_id: usize) -> usize {
        self.subtree_ids(node_id)
            .iter()
            .filter(|&&id| !self.rooms[id].kind.is_normal())
            .count()
    }

    /// Collect the ordered BFS sequence of special rooms in a subtree.
    pub fn save_special_rooms(&self, node_id: usize) -> Vec<RoomKind> {
        let mut order = VecDeque::new();
        order.push_back(node_id);
        let mut specials = Vec::new();
        while let Some(id) = order.pop_front() {
            if self.rooms[id].kind.is_special() {
                specials.push(self.rooms[id].kind);
            }
            for &child in &self.rooms[id].children {
                order.push_back(child);
            }
        }
        specials
    }

    // ── Fix algorithm (paper §4.1) ───────────────────────────────────────

    /// Redistribute `saved` special rooms into the subtree rooted at
    /// `branch_root` using BFS order, matching the paper's "fix" procedure.
    ///
    /// * If all specials have been placed, remaining rooms become Normal.
    /// * If `n` rooms remain and `ns == n` specials remain, force all to special.
    pub fn fix_branch(&mut self, branch_root: usize, saved: &[RoomKind], rng: &mut Rng) {
        let subtree = self.subtree_ids(branch_root);
        let n_rooms = subtree.len();
        let n_specials = saved.len();

        if n_specials == 0 {
            for &id in &subtree {
                self.rooms[id].kind = RoomKind::Normal;
            }
            return;
        }

        // Walk BFS within the subtree, placing specials
        let mut order = VecDeque::new();
        order.push_back(branch_root);
        let mut placed = 0usize;
        let mut visited = 0usize;
        let mut bfs_ids: Vec<usize> = Vec::new();
        while let Some(id) = order.pop_front() {
            bfs_ids.push(id);
            for &child in &self.rooms[id].children {
                order.push_back(child);
            }
        }

        for (i, &id) in bfs_ids.iter().enumerate() {
            let remaining_rooms = n_rooms - visited;
            let remaining_specials = n_specials - placed;
            visited += 1;

            // Root is always Normal — never assign a special kind to it
            if id == self.root {
                self.rooms[id].kind = RoomKind::Normal;
                continue;
            }

            if remaining_specials == 0 {
                self.rooms[id].kind = RoomKind::Normal;
            } else if remaining_rooms == remaining_specials {
                // Force assign
                self.rooms[id].kind = saved[placed];
                placed += 1;
            } else {
                // Normal probabilistic assignment mirroring Fig. 1
                let p = rng.next_f64();
                if p < PROB_KEY && placed < n_specials {
                    self.rooms[id].kind = saved[placed];
                    placed += 1;
                } else {
                    self.rooms[id].kind = RoomKind::Normal;
                }
            }
            let _ = i; // suppress warning
        }
        // Any leftover specials: append to the last few rooms
        if placed < n_specials {
            let start = bfs_ids.len().saturating_sub(n_specials - placed);
            for (j, &id) in bfs_ids[start..].iter().enumerate() {
                if placed + j < n_specials {
                    self.rooms[id].kind = saved[placed + j];
                }
            }
        }
    }

    // ── Mutation operators (paper §4.1) ──────────────────────────────────

    /// **Play-space mutation**: add or remove a leaf Normal room.
    ///
    /// * 50 % chance → add a Normal child to a random leaf (if it won't
    ///   overlap existing rooms after the grid is rebuilt).
    /// * 50 % chance → remove a random Normal leaf.
    ///
    /// Overlap is checked by the caller via [`crate::grid::DungeonGrid`].
    pub fn mutate_play_space(&mut self, rng: &mut Rng) {
        let leaves: Vec<usize> = self
            .rooms
            .iter()
            .filter(|r| r.children.is_empty())
            .map(|r| r.id)
            .collect();

        if leaves.is_empty() {
            return;
        }

        if rng.next_f64() < 0.5 {
            // Node-addition
            let parent_id = leaves[rng.next_usize(leaves.len())];
            let parent_depth = self.rooms[parent_id].depth;
            if parent_depth < MAX_DEPTH {
                // Find an unused direction
                let existing_dirs: Vec<Direction> = self.rooms[parent_id]
                    .children
                    .iter()
                    .filter_map(|&cid| self.rooms[cid].direction)
                    .collect();
                let free_dirs: Vec<Direction> = Direction::ALL
                    .iter()
                    .filter(|d| !existing_dirs.contains(d))
                    .copied()
                    .collect();
                if !free_dirs.is_empty() {
                    let dir = free_dirs[rng.next_usize(free_dirs.len())];
                    let new_id = self.rooms.len();
                    let child = Room::new(new_id, RoomKind::Normal, Some(dir), parent_depth + 1);
                    self.rooms.push(child);
                    self.rooms[parent_id].children.push(new_id);
                }
            }
        } else {
            // Node-exclusion: only Normal leaves
            let normal_leaves: Vec<usize> = leaves
                .iter()
                .filter(|&&id| self.rooms[id].kind.is_normal())
                .copied()
                .collect();
            if !normal_leaves.is_empty() {
                let victim = normal_leaves[rng.next_usize(normal_leaves.len())];
                if let Some(parent) = self.parent_of(victim) {
                    self.rooms[parent].children.retain(|&c| c != victim);
                    // Mark the room as detached (we keep the vec contiguous
                    // but note it as unreachable; the grid rebuild will skip it).
                    self.rooms[victim].kind = RoomKind::Normal;
                    self.rooms[victim].children.clear();
                }
            }
        }
    }

    /// **Mission mutation**: add a new key-lock pair, or relabel an existing one.
    pub fn mutate_mission(&mut self, rng: &mut Rng) {
        if rng.next_f64() < 0.5 {
            // Add a key-lock pair via BFS
            self.add_key_lock_pair(rng);
        } else {
            // Change-label: remove an existing pair
            self.remove_key_lock_pair(rng);
        }
    }

    /// **Switch mutation**: add or remove a switch/switch-door pair.
    ///
    /// Mirrors the mission mutation but for the switch mechanic.  A
    /// `Switch` room is placed before its paired `SwitchDoor` in BFS order
    /// (same guarantee as key/lock).
    pub fn mutate_switches(&mut self, rng: &mut Rng) {
        if rng.next_f64() < 0.5 {
            // Add a switch/door pair (BFS order guarantees switch precedes door)
            let order = self.bfs_order();
            'outer: for (i, &id) in order.iter().enumerate() {
                if id == self.root {
                    continue;
                }
                if self.rooms[id].kind.is_normal() && rng.next_f64() < PROB_SWITCH * 2.0 {
                    self.rooms[id].kind = RoomKind::Switch;
                    for &did in &order[i + 1..] {
                        if self.rooms[did].kind.is_normal()
                            && rng.next_f64() >= PROB_KEY + PROB_NORMAL
                        {
                            self.rooms[did].kind = RoomKind::SwitchDoor;
                            break;
                        }
                    }
                    break 'outer;
                }
            }
        } else {
            // Remove a switch/door — both become Normal
            let switches: Vec<usize> = self
                .rooms
                .iter()
                .filter(|r| r.kind.is_switch())
                .map(|r| r.id)
                .collect();
            if !switches.is_empty() {
                let victim = switches[rng.next_usize(switches.len())];
                self.rooms[victim].kind = RoomKind::Normal;
                // Also remove one SwitchDoor (arbitrary)
                let doors: Vec<usize> = self
                    .rooms
                    .iter()
                    .filter(|r| r.kind.is_switch_door())
                    .map(|r| r.id)
                    .collect();
                if !doors.is_empty() {
                    let d = doors[rng.next_usize(doors.len())];
                    self.rooms[d].kind = RoomKind::Normal;
                }
            }
        }
    }

    fn add_key_lock_pair(&mut self, rng: &mut Rng) {
        let new_key_id = self.key_count() as u32;
        let order = self.bfs_order();

        // Two-pass: first place a Key, then a Lock after it in BFS order.
        let mut key_bfs_idx: Option<usize> = None;
        for (i, &id) in order.iter().enumerate() {
            if id == self.root {
                continue;
            }
            if self.rooms[id].kind.is_normal() && rng.next_f64() < PROB_KEY {
                self.rooms[id].kind = RoomKind::Key { key_id: new_key_id };
                key_bfs_idx = Some(i);
                break;
            }
        }
        if let Some(after) = key_bfs_idx {
            for &id in order[after + 1..].iter() {
                if self.rooms[id].kind.is_normal() && rng.next_f64() >= PROB_KEY + PROB_NORMAL {
                    self.rooms[id].kind = RoomKind::Locked { key_id: new_key_id };
                    break;
                }
            }
        }
        // If only the key was placed but no lock, that's acceptable per §4.1
    }

    fn remove_key_lock_pair(&mut self, rng: &mut Rng) {
        let keys: Vec<usize> = self
            .rooms
            .iter()
            .filter(|r| r.kind.is_key())
            .map(|r| r.id)
            .collect();
        if keys.is_empty() {
            return;
        }

        let chosen_key_id = self.rooms[keys[rng.next_usize(keys.len())]]
            .kind
            .key_id()
            .unwrap();

        for room in self.rooms.iter_mut() {
            if room.kind.key_id() == Some(chosen_key_id) {
                room.kind = RoomKind::Normal;
            }
        }
    }

    // ── Crossover (paper Fig. 6) ─────────────────────────────────────────

    /// Perform one constrained subtree crossover between `self` and `other`.
    ///
    /// Returns `true` if the crossover succeeded; both trees are modified
    /// in-place (paper lines 21-25 of Fig. 6 are handled by the caller that
    /// already works on clones).
    ///
    /// The algorithm:
    /// 1. Repeatedly draw random cut-points (without replacement) until a
    ///    feasible pair is found, or give up if all pairs exhausted.
    /// 2. "Feasible" means the receiving subtree has ≥ special_count rooms
    ///    after overlap removal on the grid.
    /// 3. Swap the subtrees, then call `fix_branch` on the new acquisitions.
    pub fn crossover(a: &mut DungeonTree, b: &mut DungeonTree, rng: &mut Rng) -> bool {
        use crate::grid::DungeonGrid;

        // Build pools of candidate cut nodes (non-root)
        let mut candidates_a: Vec<usize> = (1..a.rooms.len()).collect();
        let mut candidates_b: Vec<usize> = (1..b.rooms.len()).collect();
        rng.shuffle(&mut candidates_a);
        rng.shuffle(&mut candidates_b);

        // We try each pair at most once (paper: "without replacement")
        let max_tries = candidates_a.len().min(candidates_b.len()).min(20);

        for i in 0..max_tries {
            let cut_a = candidates_a[i % candidates_a.len()];
            let cut_b = candidates_b[i % candidates_b.len()];

            let spc_a = a.special_count_in_subtree(cut_a);
            let spc_b = b.special_count_in_subtree(cut_b);

            // Clone and attempt swap
            let mut clone_a = a.clone();
            let mut clone_b = b.clone();

            // Swap: detach subtrees and reattach cross-wise
            let saved_a = clone_a.save_special_rooms(cut_a);
            let saved_b = clone_b.save_special_rooms(cut_b);

            swap_subtrees(&mut clone_a, cut_a, &mut clone_b, cut_b);

            // Rebuild grids to check overlaps
            let grid_a = DungeonGrid::from_tree(&clone_a);
            let grid_b = DungeonGrid::from_tree(&clone_b);

            let rooms_in_branch_a = grid_a.rooms_in_subtree(cut_a, &clone_a);
            let rooms_in_branch_b = grid_b.rooms_in_subtree(cut_b, &clone_b);

            // Feasibility check (paper lines 14-15)
            if rooms_in_branch_a >= spc_b && rooms_in_branch_b >= spc_a {
                // Apply overlap removals
                clone_a.remove_overlapping(&grid_a);
                clone_b.remove_overlapping(&grid_b);

                // Fix special room distribution
                clone_a.fix_branch(cut_a, &saved_b, rng);
                clone_b.fix_branch(cut_b, &saved_a, rng);

                // Renumber key IDs globally to avoid collisions
                clone_a.renumber_keys();
                clone_b.renumber_keys();

                *a = clone_a;
                *b = clone_b;
                return true;
            }
        }
        false
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    /// Remove rooms that the grid flagged as overlapping.
    pub fn remove_overlapping(&mut self, grid: &crate::grid::DungeonGrid) {
        let valid: HashSet<usize> = grid.placed_room_ids().into_iter().collect();
        // Remove children that are no longer valid
        for room in self.rooms.iter_mut() {
            room.children.retain(|c| valid.contains(c));
        }
    }

    /// Re-assign sequential key IDs 0..n while preserving KR→LR pairing.
    pub fn renumber_keys(&mut self) {
        let mut old_to_new: std::collections::HashMap<u32, u32> = Default::default();
        let mut next = 0u32;
        for room in self.rooms.iter_mut() {
            match room.kind {
                RoomKind::Key { key_id } => {
                    let new_id = *old_to_new.entry(key_id).or_insert_with(|| {
                        let id = next;
                        next += 1;
                        id
                    });
                    room.kind = RoomKind::Key { key_id: new_id };
                }
                RoomKind::Locked { key_id } => {
                    // Locked rooms get the mapped id; if no mapping exists the
                    // key was removed, so demote to Normal.
                    if let Some(&new_id) = old_to_new.get(&key_id) {
                        room.kind = RoomKind::Locked { key_id: new_id };
                    } else {
                        room.kind = RoomKind::Normal;
                    }
                }
                RoomKind::Normal | RoomKind::Switch | RoomKind::SwitchDoor => {}
            }
        }
    }
}

// ── Subtree swap helper ────────────────────────────────────────────────────

/// Detach the subtree at `cut_a` from `tree_a` and graft it onto `tree_b` at
/// the position of `cut_b`, and vice-versa.
///
/// Because both trees use index-based arenas, we re-index the moved nodes.
fn swap_subtrees(tree_a: &mut DungeonTree, cut_a: usize, tree_b: &mut DungeonTree, cut_b: usize) {
    // We perform the swap by moving the subtree nodes into new indices.
    move_subtree(tree_a, cut_a, tree_b, cut_b);
    // The above also handled cut_b → tree_a.  But we did both in one pass;
    // see the implementation below for the full detail.
}

/// Move the subtree rooted at `src_root` in `src` into `dst`, replacing the
/// subtree rooted at `dst_root`.  Then move the old `dst_root` subtree into
/// `src` at `src_root`.
fn move_subtree(src: &mut DungeonTree, src_root: usize, dst: &mut DungeonTree, dst_root: usize) {
    // Collect both subtrees
    let src_ids = src.subtree_ids(src_root);
    let dst_ids = dst.subtree_ids(dst_root);

    // Clone the nodes we're moving
    let src_nodes: Vec<Room> = src_ids.iter().map(|&id| src.rooms[id].clone()).collect();
    let dst_nodes: Vec<Room> = dst_ids.iter().map(|&id| dst.rooms[id].clone()).collect();

    // Append dst_nodes into src (re-indexing)
    let src_offset = src.rooms.len();
    for mut node in dst_nodes.into_iter() {
        let old_id = node.id;
        let new_id = src_offset + dst_ids.iter().position(|&i| i == old_id).unwrap();
        node.id = new_id;
        node.children = node
            .children
            .iter()
            .map(|&c| src_offset + dst_ids.iter().position(|&i| i == c).unwrap())
            .collect();
        src.rooms.push(node);
    }

    // Replace src_root's children to point to the new subtree root
    let new_dst_root_id = src_offset; // dst_root was first in dst_ids
    if let Some(parent) = src.parent_of(src_root) {
        let dir = src.rooms[src_root].direction;
        let pos = src.rooms[parent]
            .children
            .iter()
            .position(|&c| c == src_root)
            .unwrap();
        src.rooms[parent].children[pos] = new_dst_root_id;
        src.rooms[new_dst_root_id].direction = dir;
        src.rooms[new_dst_root_id].depth = src.rooms[src_root].depth;
    }

    // Mirror: append src_nodes into dst
    let dst_offset = dst.rooms.len();
    for mut node in src_nodes.into_iter() {
        let old_id = node.id;
        let new_id = dst_offset + src_ids.iter().position(|&i| i == old_id).unwrap();
        node.id = new_id;
        node.children = node
            .children
            .iter()
            .map(|&c| dst_offset + src_ids.iter().position(|&i| i == c).unwrap())
            .collect();
        dst.rooms.push(node);
    }

    let new_src_root_id = dst_offset;
    if let Some(parent) = dst.parent_of(dst_root) {
        let dir = dst.rooms[dst_root].direction;
        let pos = dst.rooms[parent]
            .children
            .iter()
            .position(|&c| c == dst_root)
            .unwrap();
        dst.rooms[parent].children[pos] = new_src_root_id;
        dst.rooms[new_src_root_id].direction = dir;
        dst.rooms[new_src_root_id].depth = dst.rooms[dst_root].depth;
    }

    // Remove original subtree nodes (mark children empty so the rest of the
    // code sees them as detached; they will be filtered out on next grid build)
    for &id in &src_ids {
        src.rooms[id].children.clear();
    }
    for &id in &dst_ids {
        dst.rooms[id].children.clear();
    }
}

// ── Placed-room-aware accessors ──────────────────────────────────────────

impl DungeonTree {
    /// Number of placed (non-detached) rooms according to the given grid.
    pub fn placed_room_count(&self, grid: &crate::grid::DungeonGrid) -> usize {
        grid.placed_room_ids().len()
    }

    /// Number of Key rooms among placed rooms.
    pub fn placed_key_count(&self, grid: &crate::grid::DungeonGrid) -> usize {
        let placed: std::collections::HashSet<usize> = grid.placed_room_ids().into_iter().collect();
        self.rooms
            .iter()
            .filter(|r| placed.contains(&r.id) && r.kind.is_key())
            .count()
    }

    /// Number of Locked rooms among placed rooms.
    pub fn placed_lock_count(&self, grid: &crate::grid::DungeonGrid) -> usize {
        let placed: std::collections::HashSet<usize> = grid.placed_room_ids().into_iter().collect();
        self.rooms
            .iter()
            .filter(|r| placed.contains(&r.id) && r.kind.is_locked())
            .count()
    }
}

// ── Post-grid repair ─────────────────────────────────────────────────────────

impl DungeonTree {
    /// After building a grid, some Key rooms may have been discarded (overlap
    /// removal).  Any `Locked` room whose key isn't in the placed set is
    /// permanently unreachable — demote it to `Normal` so the pathfinder never
    /// encounters an unopenable door.
    ///
    /// Call this immediately after [`crate::grid::DungeonGrid::from_tree`].
    pub fn repair_orphaned_locks(&mut self, grid: &crate::grid::DungeonGrid) {
        let placed: HashSet<usize> = grid.placed_room_ids().into_iter().collect();

        // Collect key IDs that are actually reachable (placed in the grid)
        let live_keys: HashSet<u32> = self
            .rooms
            .iter()
            .filter(|r| placed.contains(&r.id))
            .filter_map(|r| {
                if let crate::room::RoomKind::Key { key_id } = r.kind {
                    Some(key_id)
                } else {
                    None
                }
            })
            .collect();

        // Demote any placed Locked room whose key isn't live
        for room in self.rooms.iter_mut() {
            if !placed.contains(&room.id) {
                continue;
            }
            if let crate::room::RoomKind::Locked { key_id } = room.kind {
                if !live_keys.contains(&key_id) {
                    room.kind = crate::room::RoomKind::Normal;
                }
            }
        }
    }
}
