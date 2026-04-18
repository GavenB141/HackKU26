//! Tile-level room layout generation.
//!
//! Each logical room in the dungeon grid is expanded into an **11×11 tile
//! grid**.  The outer ring (row/col 0 and 10) is always wall; the inner 9×9
//! canvas (rows/cols 1–9) is filled by one of three interior templates.
//! Doors always appear at the **centre of the relevant wall**:
//!
//! ```text
//!   north  (5, 0)
//!   south  (5,10)
//!   east  (10, 5)
//!   west   (0, 5)
//! ```
//!
//! Because every room uses the same cell size and door positions are fixed,
//! alignment between neighbours is guaranteed with no negotiation.
//!
//! Content items (keys, switches, signs, enemies, boss, stairs/exit) are
//! placed at *interesting* positions — tiles that are geometrically tucked
//! away (high wall-adjacency, far from doors) rather than dead centre.

use std::collections::{HashSet, VecDeque};

use crate::content::{RoomContent, SignKind};
use crate::grid::DungeonGrid;
use crate::rng::Rng;
use crate::room::RoomKind;
use crate::tree::DungeonTree;

// ── Constants ─────────────────────────────────────────────────────────────────

pub const CELL_SIZE: usize = 11;
pub const INNER: usize = 9; // CELL_SIZE - 2 (wall border on each side)
pub const TOTAL_TILES: usize = CELL_SIZE * CELL_SIZE; // 121

// Fixed door positions on the 11×11 grid (col, row)
const DOOR_NORTH: (usize, usize) = (5, 0);
const DOOR_SOUTH: (usize, usize) = (5, 10);
const DOOR_EAST: (usize, usize) = (10, 5);
const DOOR_WEST: (usize, usize) = (0, 5);

// Interior obstacle density range [min, max] as fraction of 81 interior tiles
const DENSITY_MIN: f64 = 0.10;
const DENSITY_MAX: f64 = 0.25;

// ── Tile vocabulary ───────────────────────────────────────────────────────────

/// Every tile kind the generator can emit.
///
/// The runtime is free to flatten these however it likes — e.g. treat all
/// `*Spawn` tiles as `Floor` + a separate entity list.  The generator emits
/// the richest information available.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileKind {
    // ── Structural ────────────────────────────────────────────────────
    /// Solid impassable wall.
    Wall,
    /// Open floor — walkable, no special meaning.
    Floor,
    /// Passable opening in a wall connecting to an adjacent room.
    Door,
    /// Key-locked door (requires matching key to open).
    LockedDoor,
    /// Parity switch-door (opens when linked switch count parity is odd).
    SwitchDoor,
    /// Impassable decorative pillar / column.
    Pillar,
    /// Impassable water tile (moat, puddle, etc.).
    Water,
    /// Impassable pit / void.
    Pit,

    // ── Interactive / semantic ────────────────────────────────────────
    /// Chest containing a key item.
    Chest,
    /// Activatable switch that toggles linked switch-doors.
    SwitchTile,
    /// Readable sign with flavour text.
    Sign,
    /// Staircase / warp point — the dungeon exit.
    Stairs,
    /// Player spawn position (root room only).
    Spawn,
    /// Enemy spawn marker.
    EnemySpawn,
    /// Boss spawn marker.
    BossSpawn,
}

impl TileKind {
    /// True for tiles the player can stand on (ignoring locks/enemies).
    pub fn is_walkable(self) -> bool {
        matches!(
            self,
            TileKind::Floor
                | TileKind::Door
                | TileKind::LockedDoor
                | TileKind::SwitchDoor
                | TileKind::Chest
                | TileKind::SwitchTile
                | TileKind::Sign
                | TileKind::Stairs
                | TileKind::Spawn
                | TileKind::EnemySpawn
                | TileKind::BossSpawn
        )
    }

    /// True for tiles that block movement.
    pub fn is_solid(self) -> bool {
        !self.is_walkable()
    }
}

// ── Room layout ───────────────────────────────────────────────────────────────

/// The tile layout for one logical room.
#[derive(Clone, Debug)]
pub struct TileRoom {
    /// Which room in the dungeon tree this belongs to.
    pub room_id: usize,
    /// Top-left corner in world-tile coordinates (`grid_pos * CELL_SIZE`).
    pub world_origin: (i32, i32),
    /// Row-major tile data, `CELL_SIZE * CELL_SIZE = 121` entries.
    /// Index with `row * CELL_SIZE + col`.
    pub tiles: [TileKind; TOTAL_TILES],
}

impl TileRoom {
    /// Get tile at `(col, row)` in local room coordinates.
    #[inline]
    pub fn get(&self, col: usize, row: usize) -> TileKind {
        self.tiles[row * CELL_SIZE + col]
    }

    /// Set tile at `(col, row)`.
    #[inline]
    pub fn set(&mut self, col: usize, row: usize, kind: TileKind) {
        self.tiles[row * CELL_SIZE + col] = kind;
    }

    /// Convert local `(col, row)` to world-tile coordinates.
    pub fn to_world(&self, col: usize, row: usize) -> (i32, i32) {
        (
            self.world_origin.0 + col as i32,
            self.world_origin.1 + row as i32,
        )
    }

    /// All door positions in local coordinates, together with which wall they
    /// are on.
    pub fn door_positions(&self) -> Vec<(usize, usize, crate::room::Direction)> {
        let mut out = Vec::new();
        let (c, r) = DOOR_NORTH;
        if self.get(c, r) != TileKind::Wall {
            out.push((c, r, crate::room::Direction::Down));
        }
        // South = away from parent in the tree, but in grid terms it's "down"
        let (c, r) = DOOR_SOUTH;
        if self.get(c, r) != TileKind::Wall {
            out.push((c, r, crate::room::Direction::Down));
        }
        let (c, r) = DOOR_EAST;
        if self.get(c, r) != TileKind::Wall {
            out.push((c, r, crate::room::Direction::Right));
        }
        let (c, r) = DOOR_WEST;
        if self.get(c, r) != TileKind::Wall {
            out.push((c, r, crate::room::Direction::Left));
        }
        out
    }
}

// ── TileMap ───────────────────────────────────────────────────────────────────

/// The complete tile-level layout for the whole dungeon.
#[derive(Clone, Debug)]
pub struct TileMap {
    pub rooms: Vec<TileRoom>,
    /// Bounding box of the tile map in world-tile coordinates.
    pub world_min: (i32, i32),
    pub world_max: (i32, i32),
}

impl TileMap {
    /// Build a `TileMap` from a fully-generated dungeon.
    ///
    /// `exit_room_id` is used to place the `Stairs` tile.
    pub fn build(
        tree: &DungeonTree,
        grid: &DungeonGrid,
        content: &[RoomContent],
        exit_room_id: usize,
        rng: &mut Rng,
    ) -> Self {
        let placed_ids = grid.placed_room_ids();
        let mut rooms: Vec<TileRoom> = Vec::with_capacity(placed_ids.len());

        for room_id in placed_ids {
            let room = build_room(tree, grid, content, exit_room_id, room_id, rng);
            rooms.push(room);
        }

        // Compute world bounding box
        let min_x = rooms.iter().map(|r| r.world_origin.0).min().unwrap_or(0);
        let min_y = rooms.iter().map(|r| r.world_origin.1).min().unwrap_or(0);
        let max_x = rooms
            .iter()
            .map(|r| r.world_origin.0 + CELL_SIZE as i32)
            .max()
            .unwrap_or(0);
        let max_y = rooms
            .iter()
            .map(|r| r.world_origin.1 + CELL_SIZE as i32)
            .max()
            .unwrap_or(0);

        TileMap {
            rooms,
            world_min: (min_x, min_y),
            world_max: (max_x, max_y),
        }
    }

    /// Find the `TileRoom` for a given logical room ID, if it was placed.
    pub fn room_for(&self, room_id: usize) -> Option<&TileRoom> {
        self.rooms.iter().find(|r| r.room_id == room_id)
    }

    /// Total width in tiles.
    pub fn width(&self) -> i32 {
        self.world_max.0 - self.world_min.0
    }
    /// Total height in tiles.
    pub fn height(&self) -> i32 {
        self.world_max.1 - self.world_min.1
    }

    /// Render the entire map as ASCII for debugging.
    ///
    /// Tile chars: `#`=wall `.`=floor `+`=door `L`=locked-door `W`=sw-door
    /// `O`=pillar `~`=water `V`=pit `C`=chest `S`=switch `?`=sign `X`=exit/stairs
    /// `@`=spawn `E`=enemy `B`=boss
    pub fn ascii(&self) -> String {
        let w = self.width() as usize;
        let h = self.height() as usize;
        let mut grid_chars = vec![vec![' '; w]; h];

        for tr in &self.rooms {
            let ox = (tr.world_origin.0 - self.world_min.0) as usize;
            let oy = (tr.world_origin.1 - self.world_min.1) as usize;
            for row in 0..CELL_SIZE {
                for col in 0..CELL_SIZE {
                    let ch = tile_char(tr.get(col, row));
                    grid_chars[oy + row][ox + col] = ch;
                }
            }
        }

        grid_chars
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn tile_char(t: TileKind) -> char {
    match t {
        TileKind::Wall => '#',
        TileKind::Floor => '.',
        TileKind::Door => '+',
        TileKind::LockedDoor => 'L',
        TileKind::SwitchDoor => 'W',
        TileKind::Pillar => 'O',
        TileKind::Water => '~',
        TileKind::Pit => 'V',
        TileKind::Chest => 'C',
        TileKind::SwitchTile => 'S',
        TileKind::Sign => '?',
        TileKind::Stairs => 'X',
        TileKind::Spawn => '@',
        TileKind::EnemySpawn => 'E',
        TileKind::BossSpawn => 'B',
    }
}

// ── Room builder ──────────────────────────────────────────────────────────────

fn build_room(
    tree: &DungeonTree,
    grid: &DungeonGrid,
    content: &[RoomContent],
    exit_room_id: usize,
    room_id: usize,
    rng: &mut Rng,
) -> TileRoom {
    let grid_pos = grid.pos_of(room_id).unwrap_or((0, 0));
    let world_origin = (grid_pos.0 * CELL_SIZE as i32, grid_pos.1 * CELL_SIZE as i32);

    // ── 1. Start with all walls ───────────────────────────────────────────
    let mut tiles = [TileKind::Wall; TOTAL_TILES];

    // ── 2. Fill interior with floor ───────────────────────────────────────
    for row in 1..=9 {
        for col in 1..=9 {
            tiles[row * CELL_SIZE + col] = TileKind::Floor;
        }
    }

    // ── 3. Cut doors for each grid neighbour ──────────────────────────────
    let neighbours_grid = grid.neighbours(room_id);
    let room_kind = tree.rooms[room_id].kind;
    let parent_id_opt = tree.parent_of(room_id);

    // Determine which cardinal directions have neighbours
    for &nb_id in &neighbours_grid {
        let nb_pos = grid.pos_of(nb_id).unwrap_or((0, 0));
        let dx = nb_pos.0 - grid_pos.0;
        let dy = nb_pos.1 - grid_pos.1;

        let (dc, dr) = match (dx, dy) {
            (1, 0) => DOOR_EAST,
            (-1, 0) => DOOR_WEST,
            (0, 1) => DOOR_SOUTH,
            (0, -1) => DOOR_NORTH,
            _ => continue,
        };

        // Special door kinds (LockedDoor, SwitchDoor) only apply to
        // tree parent-child connections, not cross-branch grid adjacencies.
        let door_tile = if Some(nb_id) == parent_id_opt {
            // nb_id is this room's parent; this room is the child.
            door_tile_kind(room_kind, tree.rooms[nb_id].kind)
        } else if tree.rooms[room_id].children.contains(&nb_id) {
            // nb_id is a tree child of this room.
            door_tile_kind(tree.rooms[nb_id].kind, room_kind)
        } else {
            // Cross-branch grid adjacency: no tree relationship, plain door.
            TileKind::Door
        };
        tiles[dr * CELL_SIZE + dc] = door_tile;
    }
    // Fallback: if the parent is placed but not a cardinal grid neighbour,
    // ensure a door still appears on that wall.
    if let Some(parent_id) = parent_id_opt {
        if !neighbours_grid.contains(&parent_id) {
            if let Some(parent_pos) = grid.pos_of(parent_id) {
                let dx = parent_pos.0 - grid_pos.0;
                let dy = parent_pos.1 - grid_pos.1;
                let (dc, dr) = match (dx, dy) {
                    (1, 0) => DOOR_EAST,
                    (-1, 0) => DOOR_WEST,
                    (0, 1) => DOOR_SOUTH,
                    (0, -1) => DOOR_NORTH,
                    _ => (usize::MAX, usize::MAX),
                };
                if dc != usize::MAX {
                    let door_tile = door_tile_kind(room_kind, tree.rooms[parent_id].kind);
                    tiles[dr * CELL_SIZE + dc] = door_tile;
                }
            }
        }
    }

    // ── 4. Choose and apply interior template ─────────────────────────────
    let template = choose_template(room_id, content, tree, rng);
    apply_template(&mut tiles, template, rng);

    // ── 4b. Connectivity repair ───────────────────────────────────────────
    // Remove any obstacles that disconnect interior floor regions from doors,
    // guaranteeing all floor tiles are reachable from at least one door entry.
    repair_connectivity(&mut tiles);

    // ── 5. Collect door positions for interesting-placement scoring ────────
    let door_positions: Vec<(usize, usize)> = [DOOR_NORTH, DOOR_SOUTH, DOOR_EAST, DOOR_WEST]
        .iter()
        .filter(|&&(c, r)| tiles[r * CELL_SIZE + c] != TileKind::Wall)
        .map(|&(c, r)| (c, r))
        .collect();

    // ── 6. Place content items at interesting positions ───────────────────
    let c = content.get(room_id);
    place_content_tiles(
        &mut tiles,
        room_id,
        c,
        exit_room_id,
        tree,
        &door_positions,
        rng,
    );

    TileRoom {
        room_id,
        world_origin,
        tiles,
    }
}

/// Determine the tile kind for a door opening.
///
/// A room with `RoomKind::Locked` has its door (toward its parent) rendered as
/// `LockedDoor`.  A `SwitchDoor` room similarly.  All other connections are
/// plain `Door`.
fn door_tile_kind(entering_room_kind: RoomKind, _current_kind: RoomKind) -> TileKind {
    match entering_room_kind {
        RoomKind::Locked { .. } => TileKind::LockedDoor,
        RoomKind::SwitchDoor => TileKind::SwitchDoor,
        _ => TileKind::Door,
    }
}

// ── Interior templates ────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
enum Template {
    Open,
    Columned,
    Alcove,
}

fn choose_template(
    room_id: usize,
    content: &[RoomContent],
    tree: &DungeonTree,
    rng: &mut Rng,
) -> Template {
    let c = content.get(room_id);
    let is_boss = c.is_some_and(|c| c.enemies.as_ref().is_some_and(|e| e.is_boss));
    let is_enemy = c.is_some_and(|c| c.enemies.as_ref().is_some_and(|e| !e.is_boss));
    let is_exit = c.is_some_and(|c| c.is_exit);
    let is_root = room_id == tree.root;
    let has_sign = c.is_some_and(|c| c.sign.is_some());

    if is_boss || is_exit || is_root {
        // Open layouts for important rooms — clear sight lines
        Template::Open
    } else if is_enemy {
        // Enemies: 60% open (combat space), 40% columned (cover)
        if rng.next_f64() < 0.6 {
            Template::Open
        } else {
            Template::Columned
        }
    } else if has_sign {
        // Signs tucked in alcoves
        Template::Alcove
    } else {
        // Normal distribution
        match rng.next_usize(3) {
            0 => Template::Open,
            1 => Template::Columned,
            _ => Template::Alcove,
        }
    }
}

fn apply_template(tiles: &mut [TileKind; TOTAL_TILES], template: Template, rng: &mut Rng) {
    match template {
        Template::Open => apply_open(tiles, rng),
        Template::Columned => apply_columned(tiles, rng),
        Template::Alcove => apply_alcove(tiles, rng),
    }
}

// ── Open template ─────────────────────────────────────────────────────────────
// Sparse pillar clusters, mostly open floor.
fn apply_open(tiles: &mut [TileKind; TOTAL_TILES], rng: &mut Rng) {
    // Target 0–2 small pillar clusters
    let n_clusters = rng.next_usize(3); // 0, 1, or 2
    let target_obstacles = rng
        .next_usize(((INNER * INNER) as f64 * (DENSITY_MAX - DENSITY_MIN)) as usize + 1)
        + ((INNER * INNER) as f64 * DENSITY_MIN) as usize;

    let mut placed = 0usize;
    for _ in 0..n_clusters {
        if placed >= target_obstacles {
            break;
        }
        // Cluster centre in interior (avoid edges to keep doors clear)
        let cx = 2 + rng.next_usize(INNER - 3);
        let cy = 2 + rng.next_usize(INNER - 3);
        // Cluster radius 1–2
        let r = 1 + rng.next_usize(2);
        for dy in 0..=r * 2 {
            for dx in 0..=r * 2 {
                let col = (cx + dx).wrapping_sub(r);
                let row = (cy + dy).wrapping_sub(r);
                if (1..=9).contains(&col)
                    && (1..=9).contains(&row)
                    && tiles[row * CELL_SIZE + col] == TileKind::Floor
                    && rng.next_f64() < 0.5
                    && !is_near_door(col, row)
                {
                    tiles[row * CELL_SIZE + col] = TileKind::Pillar;
                    placed += 1;
                }
            }
        }
    }
    // Occasionally add 1–2 water or pit tiles for flavour
    if rng.next_f64() < 0.25 {
        let variant = if rng.next_f64() < 0.5 {
            TileKind::Water
        } else {
            TileKind::Pit
        };
        scatter_hazard(tiles, variant, 1 + rng.next_usize(2), rng);
    }
}

// ── Columned template ─────────────────────────────────────────────────────────
// Regular or offset pillar grid creating clear aisles.
fn apply_columned(tiles: &mut [TileKind; TOTAL_TILES], rng: &mut Rng) {
    // Two variants: regular 3×3 grid of pillars, or offset (checkerboard-ish)
    let offset_mode = rng.next_f64() < 0.5;

    // Place pillars on a grid of spacing 2–3 starting at interior positions
    let spacing = 2 + rng.next_usize(2); // 2 or 3
    let start_col = 1 + rng.next_usize(spacing);
    let start_row = 1 + rng.next_usize(spacing);

    let mut col = start_col;
    let mut row_num = 0usize;
    while col <= 9 {
        let mut row = start_row
            + if offset_mode && row_num % 2 == 1 {
                spacing / 2
            } else {
                0
            };
        while row <= 9 {
            if tiles[row * CELL_SIZE + col] == TileKind::Floor {
                // Leave some pillars out randomly for variety;
                // never block the door-entry cells (one step inside each door)
                if rng.next_f64() < 0.75 && !is_near_door(col, row) {
                    tiles[row * CELL_SIZE + col] = TileKind::Pillar;
                }
            }
            row += spacing;
        }
        col += spacing;
        row_num += 1;
    }

    // Density clamp — remove excess pillars if over threshold
    clamp_density(tiles, rng);
}

// ── Alcove template ───────────────────────────────────────────────────────────
// Creates recessed pockets by framing corners/edges with pillars, leaving a
// small nook of clear floor tucked against the interior wall face.
// The outer wall ring (col 0, col 10, row 0, row 10) is NEVER modified.
#[expect(
    clippy::type_complexity,
    reason = "the corner frames type is large but expected"
)]
fn apply_alcove(tiles: &mut [TileKind; TOTAL_TILES], rng: &mut Rng) {
    // Alcove sites: L-shaped pillar frames around interior corners/edges.
    // Each site is defined as a set of pillar positions that create a nook
    // at the specified corner-anchor.
    //
    // Corner anchors (interior corner coords) and their framing pillar offsets
    // relative to the anchor — we place pillars on the "open" sides of the
    // anchor to create a tucked-away recess.
    let corner_frames: &[(usize, usize, &[(i32, i32)])] = &[
        // (anchor_col, anchor_row, pillar_offsets_from_anchor)
        (2, 2, &[(1, 0), (0, 1)]),           // top-left nook
        (8, 2, &[(-1, 0), (0, 1)]),          // top-right nook
        (2, 8, &[(1, 0), (0, -1)]),          // bottom-left nook
        (8, 8, &[(-1, 0), (0, -1)]),         // bottom-right nook
        (5, 2, &[(-1, 0), (1, 0), (0, 1)]),  // top-centre nook
        (5, 8, &[(-1, 0), (1, 0), (0, -1)]), // bottom-centre nook
        (2, 5, &[(1, 0), (0, -1), (0, 1)]),  // left-centre nook
        (8, 5, &[(-1, 0), (0, -1), (0, 1)]), // right-centre nook
    ];

    let n_alcoves = 2 + rng.next_usize(3);
    let mut frames = corner_frames.to_vec();
    rng.shuffle(&mut frames);

    for &(ac, ar, offsets) in frames.iter().take(n_alcoves) {
        // Skip if anchor is near a door (would block access)
        if is_near_door(ac, ar) {
            continue;
        }

        for &(dc, dr) in offsets {
            let pc = (ac as i32 + dc) as usize;
            let pr = (ar as i32 + dr) as usize;
            // Only place pillar in interior (never on outer ring)
            if !(1..=9).contains(&pc) || !(1..=9).contains(&pr) {
                continue;
            }
            if is_near_door(pc, pr) {
                continue;
            }
            if tiles[pr * CELL_SIZE + pc] == TileKind::Floor {
                tiles[pr * CELL_SIZE + pc] = TileKind::Pillar;
            }
        }
    }

    // Sprinkle a few extra pillars in the open space for variety
    if rng.next_f64() < 0.35 {
        scatter_hazard(tiles, TileKind::Pillar, 1 + rng.next_usize(3), rng);
    }
}

// ── Template helpers ──────────────────────────────────────────────────────────

fn scatter_hazard(
    tiles: &mut [TileKind; TOTAL_TILES],
    kind: TileKind,
    count: usize,
    rng: &mut Rng,
) {
    let mut attempts = 0usize;
    let mut placed = 0usize;
    while placed < count && attempts < 50 {
        let col = 1 + rng.next_usize(INNER);
        let row = 1 + rng.next_usize(INNER);
        // Avoid placing on or adjacent to door positions
        let near_door = is_near_door(col, row);
        if tiles[row * CELL_SIZE + col] == TileKind::Floor && !near_door {
            tiles[row * CELL_SIZE + col] = kind;
            placed += 1;
        }
        attempts += 1;
    }
}

fn clamp_density(tiles: &mut [TileKind; TOTAL_TILES], rng: &mut Rng) {
    let obstacle_count = tiles[..]
        .iter()
        .filter(|&&t| matches!(t, TileKind::Pillar | TileKind::Water | TileKind::Pit))
        .count();
    let max_obstacles = ((INNER * INNER) as f64 * DENSITY_MAX) as usize;
    if obstacle_count > max_obstacles {
        // Randomly remove pillars until within range
        let to_remove = obstacle_count - max_obstacles;
        let mut removed = 0;
        for tile in tiles {
            if removed >= to_remove {
                break;
            }
            if *tile == TileKind::Pillar && rng.next_f64() < 0.5 {
                *tile = TileKind::Floor;
                removed += 1;
            }
        }
    }
}

fn is_near_door(col: usize, row: usize) -> bool {
    for &(dc, dr) in &[DOOR_NORTH, DOOR_SOUTH, DOOR_EAST, DOOR_WEST] {
        let dist =
            (col as i32 - dc as i32).unsigned_abs() + (row as i32 - dr as i32).unsigned_abs();
        if dist <= 2 {
            return true;
        }
    }
    false
}

// ── Interesting position scoring ──────────────────────────────────────────────

/// Score every walkable interior floor tile for "interestingness".
///
/// Higher score = more tucked away:
/// * `wall_adj`: count of adjacent (4-directional) wall/pillar tiles (0–4)
/// * `door_dist`: Manhattan distance from nearest door
///
/// Score = `wall_adj * 3 + door_dist`
///
/// Returns positions sorted best-first.
fn interesting_positions(
    tiles: &[TileKind; TOTAL_TILES],
    door_positions: &[(usize, usize)],
    exclude: &HashSet<(usize, usize)>,
) -> Vec<(usize, usize)> {
    let mut scored: Vec<(i32, usize, usize)> = Vec::new();

    for row in 1..=9 {
        for col in 1..=9 {
            if !tiles[row * CELL_SIZE + col].is_walkable() {
                continue;
            }
            if tiles[row * CELL_SIZE + col] == TileKind::Wall {
                continue;
            }
            if exclude.contains(&(col, row)) {
                continue;
            }

            // Wall adjacency score
            let wall_adj = [(0i32, 1i32), (0, -1), (1, 0), (-1, 0)]
                .iter()
                .filter(|&&(dc, dr)| {
                    let nc = col as i32 + dc;
                    let nr = row as i32 + dr;
                    if nc < 0 || nr < 0 || nc >= CELL_SIZE as i32 || nr >= CELL_SIZE as i32 {
                        return true; // edge counts as wall
                    }
                    let t = tiles[nr as usize * CELL_SIZE + nc as usize];
                    t.is_solid()
                })
                .count() as i32;

            // Distance from nearest door
            let door_dist = door_positions
                .iter()
                .map(|&(dc, dr)| {
                    (col as i32 - dc as i32).unsigned_abs()
                        + (row as i32 - dr as i32).unsigned_abs()
                })
                .min()
                .unwrap_or(10) as i32;

            let score = wall_adj * 3 + door_dist;
            scored.push((score, col, row));
        }
    }

    // Sort descending by score
    scored.sort_by_key(|b| std::cmp::Reverse(b.0));
    scored.into_iter().map(|(_, c, r)| (c, r)).collect()
}

/// Find positions that are reachable from all door tiles via flood-fill.
fn reachable_from_doors(
    tiles: &[TileKind; TOTAL_TILES],
    door_positions: &[(usize, usize)],
) -> HashSet<(usize, usize)> {
    if door_positions.is_empty() {
        // No doors (isolated room) — reachable from centre
        let start = (5usize, 5usize);
        return flood_fill(tiles, start);
    }
    // BFS union from all door entry points
    let mut reachable = HashSet::new();
    for &(dc, dr) in door_positions {
        // Step one tile inward from door
        let inner_steps: &[(i32, i32)] = &[(0, 1), (0, -1), (1, 0), (-1, 0)];
        for &(dx, dy) in inner_steps {
            let nc = dc as i32 + dx;
            let nr = dr as i32 + dy;
            if (1..=9).contains(&nc) && (1..=9).contains(&nr) {
                let t = tiles[nr as usize * CELL_SIZE + nc as usize];
                if t.is_walkable() {
                    reachable.extend(flood_fill(tiles, (nc as usize, nr as usize)));
                    break;
                }
            }
        }
    }
    reachable
}

fn flood_fill(tiles: &[TileKind; TOTAL_TILES], start: (usize, usize)) -> HashSet<(usize, usize)> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);
    visited.insert(start);
    while let Some((col, row)) = queue.pop_front() {
        for (dc, dr) in [(0i32, 1), (0, -1), (1, 0), (-1, 0)] {
            let nc = col as i32 + dc;
            let nr = row as i32 + dr;
            if nc < 0 || nr < 0 || nc >= CELL_SIZE as i32 || nr >= CELL_SIZE as i32 {
                continue;
            }
            let pos = (nc as usize, nr as usize);
            if visited.contains(&pos) {
                continue;
            }
            if tiles[nr as usize * CELL_SIZE + nc as usize].is_walkable() {
                visited.insert(pos);
                queue.push_back(pos);
            }
        }
    }
    visited
}

// ── Content tile placement ────────────────────────────────────────────────────

fn place_content_tiles(
    tiles: &mut [TileKind; TOTAL_TILES],
    room_id: usize,
    content: Option<&RoomContent>,
    exit_room_id: usize,
    tree: &DungeonTree,
    door_positions: &[(usize, usize)],
    _rng: &mut Rng,
) {
    // Build a set of already-used positions so items don't stack
    let mut used: HashSet<(usize, usize)> = HashSet::new();

    // Spawn tile for root room
    if room_id == tree.root {
        let pos = centre_floor(tiles, door_positions, &used);
        if let Some((col, row)) = pos {
            tiles[row * CELL_SIZE + col] = TileKind::Spawn;
            used.insert((col, row));
        }
    }

    // Exit / stairs tile
    if room_id == exit_room_id {
        let pos = best_interesting_pos(tiles, door_positions, &used);
        if let Some((col, row)) = pos {
            tiles[row * CELL_SIZE + col] = TileKind::Stairs;
            used.insert((col, row));
        }
        return; // exit rooms have nothing else
    }

    let Some(c) = content else { return };

    // Boss spawn
    if let Some(eg) = &c.enemies {
        if eg.is_boss {
            let pos = centre_floor(tiles, door_positions, &used);
            if let Some((col, row)) = pos {
                tiles[row * CELL_SIZE + col] = TileKind::BossSpawn;
                used.insert((col, row));
            }
            return; // boss rooms get only the boss marker
        }

        // Regular enemies — spread across interesting positions
        let positions = interesting_positions(tiles, door_positions, &used);
        let reachable = reachable_from_doors(tiles, door_positions);
        let enemy_positions: Vec<_> = positions
            .iter()
            .filter(|p| reachable.contains(*p))
            .take(eg.count as usize)
            .copied()
            .collect();
        for (col, row) in enemy_positions {
            tiles[row * CELL_SIZE + col] = TileKind::EnemySpawn;
            used.insert((col, row));
        }
    }

    // Key chest — most interesting reachable position
    if tree.rooms[room_id].kind.is_key() {
        let pos = best_interesting_pos(tiles, door_positions, &used);
        if let Some((col, row)) = pos {
            tiles[row * CELL_SIZE + col] = TileKind::Chest;
            used.insert((col, row));
        }
    }

    // Switch tile — second-most interesting position (after chest if any)
    if let Some(_sw) = &c.switch {
        let pos = best_interesting_pos(tiles, door_positions, &used);
        if let Some((col, row)) = pos {
            tiles[row * CELL_SIZE + col] = TileKind::SwitchTile;
            used.insert((col, row));
        }
    }

    // Sign — third position; contextual signs lean toward alcoves,
    // atmospheric can go anywhere interesting
    if let Some(sign) = &c.sign {
        let pos = match sign.kind {
            SignKind::Contextual => {
                // Prefer tiles near walls (high wall-adj) — alcove-like spots
                interesting_positions(tiles, door_positions, &used)
                    .into_iter()
                    .find(|p| reachable_from_doors(tiles, door_positions).contains(p))
            }
            SignKind::Atmospheric => best_interesting_pos(tiles, door_positions, &used),
        };
        if let Some((col, row)) = pos {
            tiles[row * CELL_SIZE + col] = TileKind::Sign;
            used.insert((col, row));
        }
    }
}

/// The single best interesting reachable position (not in `used`).
///
/// Falls back to any interior walkable tile if the scored list finds nothing
/// reachable (e.g. the exit room which has only one door and a fresh interior).
fn best_interesting_pos(
    tiles: &[TileKind; TOTAL_TILES],
    door_positions: &[(usize, usize)],
    used: &HashSet<(usize, usize)>,
) -> Option<(usize, usize)> {
    let reachable = reachable_from_doors(tiles, door_positions);
    // First try: scored interesting positions that are reachable
    let scored = interesting_positions(tiles, door_positions, used)
        .into_iter()
        .find(|p| reachable.contains(p));
    if scored.is_some() {
        return scored;
    }
    // Fallback: any *reachable* interior floor tile not in used
    for row in 1..=9 {
        for col in 1..=9 {
            let pos = (col, row);
            if used.contains(&pos) {
                continue;
            }
            let t = tiles[row * CELL_SIZE + col];
            if t == TileKind::Floor && reachable.contains(&pos) {
                return Some(pos);
            }
        }
    }
    // Last resort: any floor tile (isolated room, shouldn't normally happen)
    for row in 1..=9 {
        for col in 1..=9 {
            let pos = (col, row);
            if !used.contains(&pos) && tiles[row * CELL_SIZE + col] == TileKind::Floor {
                return Some(pos);
            }
        }
    }
    None
}

/// The floor tile nearest the centre of the room (5,5), for spawn/boss.
fn centre_floor(
    tiles: &[TileKind; TOTAL_TILES],
    door_positions: &[(usize, usize)],
    used: &HashSet<(usize, usize)>,
) -> Option<(usize, usize)> {
    let reachable = reachable_from_doors(tiles, door_positions);
    // Spiral outward from centre
    for radius in 0..5usize {
        for dr in -(radius as i32)..=(radius as i32) {
            for dc in -(radius as i32)..=(radius as i32) {
                if dc.unsigned_abs() as usize != radius && dr.unsigned_abs() as usize != radius {
                    continue;
                }
                let col = (5i32 + dc) as usize;
                let row = (5i32 + dr) as usize;
                if !(1..=9).contains(&col) || !(1..=9).contains(&row) {
                    continue;
                }
                let pos = (col, row);
                if used.contains(&pos) {
                    continue;
                }
                if !reachable.contains(&pos) {
                    continue;
                }
                if tiles[row * CELL_SIZE + col].is_walkable()
                    && tiles[row * CELL_SIZE + col] == TileKind::Floor
                {
                    return Some(pos);
                }
            }
        }
    }
    None
}

// ── Connectivity repair ───────────────────────────────────────────────────────

/// Remove obstacles that create floor tiles unreachable from any door entry.
///
/// Algorithm:
/// 1. BFS from one tile inside each door to find the "main" reachable set.
/// 2. Any interior floor tile NOT in the main set is isolated.
/// 3. For each isolated tile, find the nearest obstacle that, if removed,
///    reconnects it — remove obstacles along the shortest obstacle-crossing
///    path from the isolated tile to the main reachable set.
fn repair_connectivity(tiles: &mut [TileKind; TOTAL_TILES]) {
    // Identify door entry points (one step inward from each door wall position)
    let door_entries: Vec<(usize, usize)> = {
        let mut v = Vec::new();
        for &(dc, dr) in &[DOOR_NORTH, DOOR_SOUTH, DOOR_EAST, DOOR_WEST] {
            if tiles[dr * CELL_SIZE + dc] == TileKind::Wall {
                continue;
            }
            // Step inward
            let steps: &[(i32, i32)] = &[(0, 1), (0, -1), (1, 0), (-1, 0)];
            for &(dx, dy) in steps {
                let nc = dc as i32 + dx;
                let nr = dr as i32 + dy;
                if (1..=9).contains(&nc)
                    && (1..=9).contains(&nr)
                    && tiles[nr as usize * CELL_SIZE + nc as usize].is_walkable()
                {
                    v.push((nc as usize, nr as usize));
                    break;
                }
            }
        }
        v
    };

    if door_entries.is_empty() {
        // No doors — flood from centre as fallback
        // Nothing to repair without a reference point
        return;
    }

    // Build reachable set from all door entries
    let mut reachable: HashSet<(usize, usize)> = HashSet::new();
    for &entry in &door_entries {
        reachable.extend(flood_fill(tiles, entry));
    }

    // Find all INTERIOR floor tiles that are NOT reachable (rows/cols 1-9 only)
    let mut isolated: Vec<(usize, usize)> = Vec::new();
    for row in 1..=(CELL_SIZE - 2) {
        for col in 1..=(CELL_SIZE - 2) {
            let pos = (col, row);
            let t = tiles[row * CELL_SIZE + col];
            if t.is_walkable()
                && t != TileKind::Door
                && t != TileKind::LockedDoor
                && t != TileKind::SwitchDoor
                && !reachable.contains(&pos)
            {
                isolated.push(pos);
            }
        }
    }

    if isolated.is_empty() {
        return;
    }

    // For each isolated tile, carve the shortest path to the reachable set
    // by removing obstacles (BFS that can step through obstacles at cost 1,
    // floor at cost 0 — essentially a 0/1 BFS).
    for iso in isolated {
        carve_path_to_reachable(tiles, iso, &reachable);
        // Rebuild reachable set after each repair so subsequent isolated tiles
        // benefit from newly opened paths
        let mut new_reach: HashSet<(usize, usize)> = HashSet::new();
        for &entry in &door_entries {
            new_reach.extend(flood_fill(tiles, entry));
        }
        reachable = new_reach;
    }
}

/// BFS (0-cost floor, 1-cost obstacle) from `start` toward the nearest
/// `reachable` tile; remove all obstacles along the found path.
fn carve_path_to_reachable(
    tiles: &mut [TileKind; TOTAL_TILES],
    start: (usize, usize),
    reachable: &HashSet<(usize, usize)>,
) {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;

    // Dijkstra: cost = number of obstacles to remove
    let mut dist: std::collections::HashMap<(usize, usize), u32> = std::collections::HashMap::new();
    let mut prev: std::collections::HashMap<(usize, usize), (usize, usize)> =
        std::collections::HashMap::new();
    let mut heap: BinaryHeap<(Reverse<u32>, usize, usize)> = BinaryHeap::new();

    dist.insert(start, 0);
    heap.push((Reverse(0), start.0, start.1));

    let mut goal: Option<(usize, usize)> = None;

    while let Some((Reverse(cost), col, row)) = heap.pop() {
        let pos = (col, row);
        if reachable.contains(&pos) {
            goal = Some(pos);
            break;
        }
        if dist.get(&pos).is_none_or(|&d| cost > d) {
            continue;
        }

        for (dc, dr) in [(0i32, 1), (0, -1), (1, 0), (-1, 0)] {
            let nc = col as i32 + dc;
            let nr = row as i32 + dr;
            // Stay strictly within the interior (never touch outer wall ring)
            if nc < 1 || nr < 1 || nc > (CELL_SIZE as i32 - 2) || nr > (CELL_SIZE as i32 - 2) {
                continue;
            }
            let npos = (nc as usize, nr as usize);
            let is_obstacle = tiles[nr as usize * CELL_SIZE + nc as usize].is_solid();
            let new_cost = cost + if is_obstacle { 1 } else { 0 };
            if dist.get(&npos).is_none_or(|&d| new_cost < d) {
                dist.insert(npos, new_cost);
                prev.insert(npos, pos);
                heap.push((Reverse(new_cost), nc as usize, nr as usize));
            }
        }
    }

    // Trace back path and remove obstacles
    if let Some(mut cur) = goal {
        while cur != start {
            if let Some(&p) = prev.get(&cur) {
                if tiles[cur.1 * CELL_SIZE + cur.0].is_solid() {
                    tiles[cur.1 * CELL_SIZE + cur.0] = TileKind::Floor;
                }
                cur = p;
            } else {
                break;
            }
        }
    }
}
