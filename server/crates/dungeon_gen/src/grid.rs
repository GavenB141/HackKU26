//! **Phenotype decoder**: translates a [`DungeonTree`] genotype into a 2-D grid
//! of room positions, handling the rotation rule (paper §3.3, Fig. 3) and
//! discarding overlapping nodes (paper §3.3, Fig. 4).

use std::collections::{HashMap, HashSet, VecDeque};

use crate::tree::DungeonTree;

/// A cell in the decoded 2-D map.
#[derive(Clone, Debug)]
pub struct GridCell {
    pub room_id: usize,
    pub pos: (i32, i32),
}

/// The decoded spatial layout of a dungeon.
#[derive(Clone, Debug)]
pub struct DungeonGrid {
    /// All successfully placed cells, keyed by grid position.
    pub cells: HashMap<(i32, i32), GridCell>,
    /// The bounding box, computed lazily.
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

impl DungeonGrid {
    /// Decode a [`DungeonTree`] into a grid, skipping overlapping nodes.
    ///
    /// Translation logic (paper §3.3):
    /// * Root is placed at `(0, 0)`.
    /// * Each child is placed relative to its parent using its `Direction`
    ///   offset, but the direction must first be *rotated* so that the parent
    ///   remains "north" of the child (Fig. 3).
    /// * If the computed cell is already occupied the node is discarded.
    pub fn from_tree(tree: &DungeonTree) -> Self {
        let mut cells: HashMap<(i32, i32), GridCell> = HashMap::new();
        let mut positions: HashMap<usize, (i32, i32)> = HashMap::new();
        // Track which rooms are reachable from root (avoids detached nodes)
        let mut reachable: HashSet<usize> = HashSet::new();

        // Root always occupies (0,0) — insert before BFS so no child can claim it
        cells.insert(
            (0, 0),
            GridCell {
                room_id: tree.root,
                pos: (0, 0),
            },
        );

        // BFS to assign positions
        let mut queue: VecDeque<usize> = VecDeque::new();
        queue.push_back(tree.root);
        positions.insert(tree.root, (0, 0));
        reachable.insert(tree.root);

        while let Some(parent_id) = queue.pop_front() {
            let parent_pos = positions[&parent_id];
            let parent_dir = tree.rooms[parent_id].direction;

            for &child_id in &tree.rooms[parent_id].children {
                if !reachable.contains(&parent_id) {
                    continue;
                }

                let child_dir = match tree.rooms[child_id].direction {
                    Some(d) => d,
                    None => continue,
                };

                // Rotate direction to maintain "parent is north" invariant
                let effective_dir = if let Some(pd) = parent_dir {
                    child_dir.rotate_for_parent(pd)
                } else {
                    child_dir
                };

                let (dx, dy) = effective_dir.offset();
                let child_pos = (parent_pos.0 + dx, parent_pos.1 + dy);

                if cells.contains_key(&child_pos) {
                    // Overlap → discard this node (paper §3.3)
                    continue;
                }

                cells.insert(
                    child_pos,
                    GridCell {
                        room_id: child_id,
                        pos: child_pos,
                    },
                );
                positions.insert(child_id, child_pos);
                reachable.insert(child_id);
                queue.push_back(child_id);
            }
        }

        // Compute bounding box
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (0, 0, 0, 0);
        for &(x, y) in cells.keys() {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }

        DungeonGrid {
            cells,
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    /// IDs of all placed (non-overlapping) rooms.
    pub fn placed_room_ids(&self) -> Vec<usize> {
        self.cells.values().map(|c| c.room_id).collect()
    }

    /// Number of placed rooms in the subtree rooted at `node_id`.
    pub fn rooms_in_subtree(&self, node_id: usize, tree: &DungeonTree) -> usize {
        let subtree_ids: HashSet<usize> = tree.subtree_ids(node_id).into_iter().collect();
        let placed: HashSet<usize> = self.placed_room_ids().into_iter().collect();
        subtree_ids.intersection(&placed).count()
    }

    /// Position of `room_id` in the grid, if it was placed.
    pub fn pos_of(&self, room_id: usize) -> Option<(i32, i32)> {
        self.cells
            .values()
            .find(|c| c.room_id == room_id)
            .map(|c| c.pos)
    }

    /// Neighbours of `room_id` (rooms in adjacent cardinal cells).
    pub fn neighbours(&self, room_id: usize) -> Vec<usize> {
        let Some(pos) = self.pos_of(room_id) else {
            return vec![];
        };
        let offsets = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        offsets
            .iter()
            .filter_map(|&(dx, dy)| {
                let np = (pos.0 + dx, pos.1 + dy);
                self.cells.get(&np).map(|c| c.room_id)
            })
            .collect()
    }

    /// Width of the bounding box (in rooms).
    pub fn width(&self) -> i32 {
        self.max_x - self.min_x + 1
    }
    /// Height of the bounding box (in rooms).
    pub fn height(&self) -> i32 {
        self.max_y - self.min_y + 1
    }

    /// Convert to a flat tile string for debugging / serialisation.
    ///
    /// `S` = spawn, `K` = key room, `L` = locked room, `.` = normal, ` ` = empty.
    pub fn ascii_map(&self, tree: &DungeonTree) -> String {
        let w = (self.max_x - self.min_x + 1) as usize;
        let h = (self.max_y - self.min_y + 1) as usize;
        let mut grid = vec![vec![' '; w]; h];

        for (&(x, y), cell) in &self.cells {
            let col = (x - self.min_x) as usize;
            let row = (y - self.min_y) as usize;
            let ch = if cell.room_id == tree.root {
                'S'
            } else {
                match tree.rooms[cell.room_id].kind {
                    crate::room::RoomKind::Normal => '.',
                    crate::room::RoomKind::Key { .. } => 'K',
                    crate::room::RoomKind::Locked { .. } => 'L',
                    crate::room::RoomKind::Switch => 'W',
                    crate::room::RoomKind::SwitchDoor => 'D',
                }
            };
            grid[row][col] = ch;
        }

        grid.into_iter()
            .map(|row| row.into_iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
