//! **Room content layer** — orthogonal annotations that sit on top of the
//! structural [`RoomKind`] from the paper.
//!
//! Three independent features are supported:
//!
//! ## Switch / door puzzle
//!
//! A [`SwitchState`] room contains an activatable switch that carries a set of
//! *signal IDs*.  A [`SwitchDoor`] room has a locked passage to its parent
//! whose open/closed state is parity-based:
//!
//! > A door is **open** when the XOR-parity of all activated switches that
//! > share at least one signal ID with the door is **odd**.
//!
//! This is a strict generalisation of the original key/lock mechanic:
//! * 1:1 — one switch, one door, one shared signal → classic toggle.
//! * 1:N — one switch, many doors on the same signal → master lever.
//! * N:1 — N switches all wired to one door signal; every switch must be hit
//!   an odd total number of times for the door to open (combination lock).
//!
//! ## Enemy groups and the boss
//!
//! Any room may contain an [`EnemyGroup`].  Enemies block passage: a room is
//! impassable until cleared.  We model "clearing" as automatic on first visit
//! (the player defeats enemies to pass).  The boss is a special `EnemyGroup`
//! with `is_boss: true`; it is always placed in the goal room.
//!
//! ## Signs
//!
//! A [`Sign`] carries flavour text.  Two kinds exist:
//! * [`SignKind::Contextual`] — placed near a key or locked door; the text is
//!   a hint generated from a small template set.
//! * [`SignKind::Atmospheric`] — placed in a random normal room; the text is
//!   purely decorative.
//!
//! Signs have no effect on pathfinding.

use crate::grid::GridCell;
use crate::pathfinding::critical_path_rooms;
use crate::rng::Rng;
use crate::room::{Direction, Room, RoomKind};

// ── Switch / SwitchDoor ─────────────────────────────────────────────────────

/// A switch in a room.  Activating it flips the parity of every door that
/// shares at least one signal ID with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchState {
    /// Globally unique switch ID (used by the pathfinder state machine).
    pub switch_id: u32,
    /// Signal IDs that this switch broadcasts when toggled.
    pub signals: Vec<u32>,
}

/// A locked passage controlled by parity of linked switches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchDoor {
    /// Globally unique door ID.
    pub door_id: u32,
    /// Signal IDs this door listens to.
    /// The door is open iff the count of activated switches sharing ≥1 signal
    /// with this door is **odd**.
    pub signals: Vec<u32>,
}

impl SwitchDoor {
    /// Evaluate whether this door is open given a bitmask of activated switches
    /// and the full switch list for the dungeon.
    ///
    /// `switch_mask` — bit `i` set means switch with `switch_id == i` is active.
    pub fn is_open(&self, switch_mask: u64, all_switches: &[SwitchState]) -> bool {
        let parity: u32 = all_switches
            .iter()
            .filter(|sw| {
                // Switch is activated
                sw.switch_id < 64 && (switch_mask >> sw.switch_id) & 1 == 1
            })
            .filter(|sw| {
                // Switch shares ≥1 signal with this door
                sw.signals.iter().any(|s| self.signals.contains(s))
            })
            .count() as u32;
        parity % 2 == 1
    }
}

// ── Enemies / Boss ──────────────────────────────────────────────────────────

/// A group of enemies occupying a room.
#[derive(Clone, Debug)]
pub struct EnemyGroup {
    /// Flavour/game data: how many enemies (or enemy tier, etc.).
    pub count: u32,
    /// True for the final boss.  Boss rooms are always the goal room.
    pub is_boss: bool,
}

impl EnemyGroup {
    pub fn regular(count: u32) -> Self {
        EnemyGroup {
            count,
            is_boss: false,
        }
    }
    pub fn boss(count: u32) -> Self {
        EnemyGroup {
            count,
            is_boss: true,
        }
    }
}

// ── Signs ───────────────────────────────────────────────────────────────────

/// Discriminates between the two sign placement strategies.
#[derive(Clone, Debug)]
pub enum SignKind {
    /// Placed near a key or locked door; the text is a contextual hint.
    Contextual,
    /// Placed in a random normal room; purely atmospheric.
    Atmospheric,
}

/// A readable sign in a room.
#[derive(Clone, Debug)]
pub struct Sign {
    pub kind: SignKind,
    pub text: String,
}

// ── RoomContent ─────────────────────────────────────────────────────────────

/// The orthogonal content annotation attached to every [`Room`].
///
/// All fields are optional — a fully-empty `RoomContent` is always valid.
#[derive(Clone, Debug, Default)]
pub struct RoomContent {
    /// An enemy group blocking this room (cleared on first visit).
    pub enemies: Option<EnemyGroup>,
    /// A switch that can be toggled by the player.
    pub switch: Option<SwitchState>,
    /// A switch-door sealing the passage to this room's parent.
    pub switch_door: Option<SwitchDoor>,
    /// An optional sign with flavour text.
    pub sign: Option<Sign>,
    /// This room is the dungeon exit (staircase / warp point).
    /// There is exactly one exit per dungeon, always a child of the boss room.
    pub is_exit: bool,
}

impl RoomContent {
    pub fn is_empty(&self) -> bool {
        self.enemies.is_none()
            && self.switch.is_none()
            && self.switch_door.is_none()
            && self.sign.is_none()
            && !self.is_exit
    }
}

// ── Placement helpers ────────────────────────────────────────────────────────

/// Parameters controlling how content is scattered across a dungeon.
#[derive(Clone, Debug)]
pub struct ContentConfig {
    /// Target number of enemy groups (not counting boss).
    pub target_enemy_groups: u32,
    /// Maximum enemies per group.
    pub max_enemies_per_group: u32,
    /// Target number of switch/door pairs.
    pub target_switch_pairs: u32,
    /// Maximum number of signals per switch/door pair (≥1).
    /// A value of 1 gives 1:1 pairs; higher values create shared-signal puzzles.
    pub max_signals_per_pair: u32,
    /// Probability [0,1] that any Normal room near a key/lock gets a contextual sign.
    pub prob_contextual_sign: f64,
    /// Probability [0,1] that any remaining Normal room gets an atmospheric sign.
    pub prob_atmospheric_sign: f64,
}

impl Default for ContentConfig {
    fn default() -> Self {
        ContentConfig {
            target_enemy_groups: 4,
            max_enemies_per_group: 3,
            target_switch_pairs: 2,
            max_signals_per_pair: 2,
            prob_contextual_sign: 0.5,
            prob_atmospheric_sign: 0.08,
        }
    }
}

/// Place all content onto the rooms of a decoded dungeon.
///
/// Returns `(content, exit_room_id)`.  The exit room is appended as a new
/// child of the boss room (the structural goal room); it is the true
/// pathfinding target — one step past the boss.
///
/// Guarantees:
/// * Exactly one room in the returned content vec has `is_exit == true`.
/// * `exit_room_id` always indexes a placed, `is_exit`-tagged room.
pub fn place_content(
    tree: &mut crate::tree::DungeonTree,
    grid: &mut crate::grid::DungeonGrid,
    goal_id: usize,
    cfg: &ContentConfig,
    rng: &mut Rng,
) -> (Vec<RoomContent>, usize) {
    use std::collections::HashSet;

    // Re-derive the placed boss room from the *current* grid to guard against
    // the caller passing an unplaced goal_id (e.g. after a crossover removed it).
    let placed_set: HashSet<usize> = grid.placed_room_ids().into_iter().collect();
    let boss_id: usize = if placed_set.contains(&goal_id) {
        goal_id
    } else {
        // goal_id was discarded by overlap removal — find the deepest placed
        // structural barrier, falling back to the deepest placed leaf.
        let order = tree.bfs_order();
        order
            .iter()
            .rev()
            .find(|&&id| {
                placed_set.contains(&id)
                    && matches!(
                        tree.rooms[id].kind,
                        crate::room::RoomKind::Locked { .. } | crate::room::RoomKind::SwitchDoor
                    )
            })
            .copied()
            .unwrap_or_else(|| {
                order
                    .iter()
                    .filter(|&&id| {
                        placed_set.contains(&id)
                            && tree.rooms[id]
                                .children
                                .iter()
                                .all(|c| !placed_set.contains(c))
                    })
                    .max_by_key(|&&id| tree.rooms[id].depth)
                    .copied()
                    .unwrap_or(tree.root)
            })
    };

    let n = tree.rooms.len();
    let mut content: Vec<RoomContent> = (0..n).map(|_| RoomContent::default()).collect();

    // Re-derive placed_bfs after potential boss_id correction
    let placed: HashSet<usize> = grid.placed_room_ids().into_iter().collect();
    let bfs = tree.bfs_order();
    let placed_bfs: Vec<usize> = bfs
        .iter()
        .copied()
        .filter(|id| placed.contains(id))
        .collect();

    // ── 1. Boss on boss_id, exit room appended adjacent to it ────────────
    content[boss_id].enemies = Some(EnemyGroup::boss(1));

    // Search for a free cell: first try direct cardinal neighbours of boss,
    // then fall back to any placed room that has a free adjacent cell.
    let exit_room_id = append_exit_room(tree, grid, &mut content, boss_id, rng);

    // ── 2. Enemy groups on Normal rooms (not root, not goal) ─────────────
    let mut enemy_candidates: Vec<usize> = placed_bfs
        .iter()
        .copied()
        .filter(|&id| {
            id != tree.root
                && id != boss_id
                && id != exit_room_id
                && tree.rooms[id].kind.is_normal()
                && content[id].enemies.is_none()
        })
        .collect();
    rng.shuffle(&mut enemy_candidates);

    let n_enemies = (cfg.target_enemy_groups as usize).min(enemy_candidates.len());
    for &id in enemy_candidates[..n_enemies].iter() {
        let count = 1 + rng.next_usize(cfg.max_enemies_per_group as usize);
        content[id].enemies = Some(EnemyGroup::regular(count as u32));
    }

    // ── 3. Switch / door pairs ────────────────────────────────────────────
    // For each pair: pick a room for the switch, then pick a later (in BFS)
    // Normal room for the switch-door.  Share one or more signal IDs.
    let mut switch_candidates: Vec<usize> = placed_bfs
        .iter()
        .copied()
        .filter(|&id| {
            id != tree.root
                && id != boss_id
                && id != exit_room_id
                && tree.rooms[id].kind.is_normal()
                && content[id].switch.is_none()
                && content[id].switch_door.is_none()
        })
        .collect();

    // Compute critical path (root → exit, ignoring content) so we never place
    // a switch-door on a room that the player *must* pass through.  Switch
    // puzzles should always be optional detours, not mandatory blockers.
    // Use the already-computed exit_room_id (appended above)
    let crit_path = critical_path_rooms(tree, grid, tree.root, exit_room_id);

    // Remove any candidate that lies on the critical path
    switch_candidates.retain(|id| !crit_path.contains(id));

    let n_pairs = (cfg.target_switch_pairs as usize).min(switch_candidates.len() / 2);
    let mut next_switch_id = 0u32;
    let mut next_door_id = 0u32;
    let mut next_signal_id = 0u32;

    for _ in 0..n_pairs {
        if switch_candidates.len() < 2 {
            break;
        }

        // Pick switch room from first half of remaining candidates
        let sw_idx = rng.next_usize((switch_candidates.len() / 2).max(1));
        let sw_room = switch_candidates.remove(sw_idx);

        // Pick door room from what remains
        if switch_candidates.is_empty() {
            break;
        }
        let dr_idx = rng.next_usize(switch_candidates.len());
        let door_room = switch_candidates.remove(dr_idx);

        // Build signal set
        let n_signals = 1 + rng.next_usize(cfg.max_signals_per_pair as usize);
        let signals: Vec<u32> = (0..n_signals)
            .map(|_| {
                let s = next_signal_id;
                next_signal_id += 1;
                s
            })
            .collect();

        content[sw_room].switch = Some(SwitchState {
            switch_id: next_switch_id,
            signals: signals.clone(),
        });
        next_switch_id += 1;

        content[door_room].switch_door = Some(SwitchDoor {
            door_id: next_door_id,
            signals,
        });
        next_door_id += 1;
    }

    // ── 4. Signs ──────────────────────────────────────────────────────────

    // 4a. Contextual: Normal rooms that are adjacent (grid-neighbour) to a
    //     Key room or a Locked room get a hint sign with some probability.
    let special_ids: HashSet<usize> = placed_bfs
        .iter()
        .copied()
        .filter(|&id| !tree.rooms[id].kind.is_normal())
        .collect();

    for &id in &placed_bfs {
        if !tree.rooms[id].kind.is_normal() {
            continue;
        }
        if content[id].sign.is_some() {
            continue;
        }
        let neighbours = grid.neighbours(id);
        let near_special = neighbours.iter().any(|n| special_ids.contains(n));
        if near_special && rng.next_f64() < cfg.prob_contextual_sign {
            content[id].sign = Some(Sign {
                kind: SignKind::Contextual,
                text: contextual_hint(tree, grid, id, rng),
            });
        }
    }

    // 4b. Atmospheric: remaining Normal rooms get a random flavour line.
    for &id in &placed_bfs {
        if !tree.rooms[id].kind.is_normal() {
            continue;
        }
        if content[id].sign.is_some() {
            continue;
        }
        if content[id].enemies.is_some() {
            continue;
        } // enemies occupy the room
        if rng.next_f64() < cfg.prob_atmospheric_sign {
            content[id].sign = Some(Sign {
                kind: SignKind::Atmospheric,
                text: atmospheric_line(rng).to_string(),
            });
        }
    }

    (content, exit_room_id)
}

// ── Exit room appender ──────────────────────────────────────────────────────

/// Append an exit room parented to `boss_id` in the tree.
///
/// The exit is *always* a tree-child of the boss room, so the player must
/// enter the boss room before reaching the exit.  For the grid position we
/// BFS outward from the boss cell until we find a free slot, which guarantees
/// placement even when all four cardinal neighbours are occupied.
fn append_exit_room(
    tree: &mut crate::tree::DungeonTree,
    grid: &mut crate::grid::DungeonGrid,
    content: &mut Vec<RoomContent>,
    boss_id: usize,
    _rng: &mut Rng,
) -> usize {
    let boss_pos = match grid.pos_of(boss_id) {
        Some(p) => p,
        None => {
            // Boss not placed (should not happen after boss_id validation).
            content[boss_id].is_exit = true;
            return boss_id;
        }
    };

    // BFS outward from boss_pos to find nearest free grid cell.
    let exit_pos = find_nearest_free_cell(grid, boss_pos);

    let exit_id = tree.rooms.len();
    let dir = Direction::ALL
        .iter()
        .copied()
        .find(|&d| {
            let (dx, dy) = d.offset();
            (boss_pos.0 + dx, boss_pos.1 + dy) == exit_pos
        })
        .unwrap_or(Direction::Down); // non-cardinal offset → Down as token direction

    let mut exit_room = Room::new(
        exit_id,
        RoomKind::Normal,
        Some(dir),
        tree.rooms[boss_id].depth + 1,
    );
    exit_room.grid_pos = exit_pos;
    tree.rooms[boss_id].children.push(exit_id); // boss always parent in tree
    tree.rooms.push(exit_room);

    grid.cells.insert(
        exit_pos,
        GridCell {
            room_id: exit_id,
            pos: exit_pos,
        },
    );
    grid.min_x = grid.min_x.min(exit_pos.0);
    grid.max_x = grid.max_x.max(exit_pos.0);
    grid.min_y = grid.min_y.min(exit_pos.1);
    grid.max_y = grid.max_y.max(exit_pos.1);

    content.push(RoomContent {
        is_exit: true,
        ..Default::default()
    });
    exit_id
}

/// BFS outward from `origin` to find the nearest free grid cell.
///
/// This always terminates because the dungeon grid is finite; the free cell
/// found may be non-adjacent to the boss but the boss is still the tree parent.
fn find_nearest_free_cell(grid: &crate::grid::DungeonGrid, origin: (i32, i32)) -> (i32, i32) {
    use std::collections::{HashSet, VecDeque};
    let offsets = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
    queue.push_back(origin);
    visited.insert(origin);
    while let Some(pos) = queue.pop_front() {
        for &(dx, dy) in &offsets {
            let next = (pos.0 + dx, pos.1 + dy);
            if !grid.cells.contains_key(&next) {
                return next;
            }
            if visited.insert(next) {
                queue.push_back(next);
            }
        }
    }
    (origin.0 + 1000, origin.1 + 1000) // unreachable in practice
}

// ── Sign text generators ─────────────────────────────────────────────────────

fn contextual_hint(
    tree: &crate::tree::DungeonTree,
    grid: &crate::grid::DungeonGrid,
    room_id: usize,
    rng: &mut Rng,
) -> String {
    // Identify what's nearby
    let neighbours = grid.neighbours(room_id);
    let has_key_near = neighbours
        .iter()
        .any(|&n| matches!(tree.rooms[n].kind, RoomKind::Key { .. }));
    let has_lock_near = neighbours
        .iter()
        .any(|&n| matches!(tree.rooms[n].kind, RoomKind::Locked { .. }));

    let key_hints = [
        "Something glints behind that door…",
        "A useful tool lies just ahead.",
        "The brave are rewarded.",
        "What you seek is close.",
        "Those who search shall find.",
    ];
    let lock_hints = [
        "This door will not yield without a key.",
        "Seek the matching key before you proceed.",
        "A locked path — look elsewhere first.",
        "Only the prepared may pass.",
        "Turn back; you are not yet ready.",
    ];
    let generic = [
        "Many have walked this path.",
        "Beware what lurks ahead.",
        "The dungeon remembers all who enter.",
    ];

    let pool: &[&str] = if has_lock_near {
        &lock_hints
    } else if has_key_near {
        &key_hints
    } else {
        &generic
    };

    pool[rng.next_usize(pool.len())].to_string()
}

fn atmospheric_line(rng: &mut Rng) -> &'static str {
    const LINES: &[&str] = &[
        "The torches flicker in a sourceless wind.",
        "Scratched into the stone: 'Do not open the red chest.'",
        "Water drips somewhere in the darkness.",
        "The air smells of old iron.",
        "Footprints lead in — none lead out.",
        "Someone has been here recently.",
        "The walls are warm to the touch.",
        "An eerie silence fills the chamber.",
        "Strange symbols cover the floor.",
        "This room feels watched.",
        "A faint humming emanates from the walls.",
        "The dust is undisturbed here.",
        "Old bones rest in the corner.",
        "The ceiling is higher than it looks.",
        "You feel a chill that has nothing to do with the cold.",
    ];
    LINES[rng.next_usize(LINES.len())]
}
