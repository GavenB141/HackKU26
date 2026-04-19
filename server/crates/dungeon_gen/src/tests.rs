//! Unit and integration tests covering both the original paper behaviour and
//! the extended features (switches, enemies, boss, signs).

// ── RNG ──────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod rng_tests {
    use crate::rng::Rng;

    #[test]
    fn deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        assert!(!(0..10).all(|_| a.next_u64() == b.next_u64()));
    }

    #[test]
    fn next_usize_in_range() {
        let mut rng = Rng::new(99);
        for _ in 0..1000 {
            assert!(rng.next_usize(7) < 7);
        }
    }

    #[test]
    fn shuffle_is_permutation() {
        let mut rng = Rng::new(7);
        let mut v: Vec<u32> = (0..20).collect();
        let original = v.clone();
        rng.shuffle(&mut v);
        let mut sorted = v.clone();
        sorted.sort();
        assert_eq!(sorted, original);
    }
}

// ── Tree ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tree_tests {
    use crate::rng::Rng;
    use crate::room::RoomKind;
    use crate::tree::DungeonTree;

    #[test]
    fn root_is_normal() {
        let mut rng = Rng::new(0);
        let tree = DungeonTree::generate(&mut rng);
        assert!(matches!(tree.rooms[tree.root].kind, RoomKind::Normal));
    }

    #[test]
    fn key_lock_pairing_valid() {
        // Every Locked room's key_id must exist as a Key room
        for seed in 0u64..50 {
            let mut rng = Rng::new(seed * 31 + 1);
            let tree = DungeonTree::generate(&mut rng);
            let key_ids: std::collections::HashSet<u32> = tree
                .rooms
                .iter()
                .filter_map(|r| {
                    if let RoomKind::Key { key_id } = r.kind {
                        Some(key_id)
                    } else {
                        None
                    }
                })
                .collect();
            for room in &tree.rooms {
                if let RoomKind::Locked { key_id } = room.kind {
                    assert!(
                        key_ids.contains(&key_id),
                        "Locked room {} references missing key {}",
                        room.id,
                        key_id
                    );
                }
            }
        }
    }

    #[test]
    fn bfs_visits_each_node_once() {
        let mut rng = Rng::new(55);
        let tree = DungeonTree::generate(&mut rng);
        let bfs = tree.bfs_order();
        let mut seen = std::collections::HashSet::new();
        for &id in &bfs {
            assert!(seen.insert(id), "BFS visited {} twice", id);
        }
    }

    #[test]
    fn switch_kinds_generated() {
        // Over many seeds, at least some trees should contain Switch rooms
        let found = (0u64..100).any(|seed| {
            let mut rng = Rng::new(seed);
            let tree = DungeonTree::generate(&mut rng);
            tree.rooms.iter().any(|r| r.kind.is_switch())
        });
        assert!(found, "No Switch rooms found across 100 seeds");
    }
}

// ── Grid ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod grid_tests {
    use crate::grid::DungeonGrid;
    use crate::rng::Rng;
    use crate::tree::DungeonTree;

    #[test]
    fn root_at_origin() {
        let mut rng = Rng::new(11);
        let tree = DungeonTree::generate(&mut rng);
        let grid = DungeonGrid::from_tree(&tree);
        assert_eq!(grid.pos_of(tree.root), Some((0, 0)));
    }

    #[test]
    fn no_duplicate_cells() {
        for seed in 0u64..30 {
            let mut rng = Rng::new(seed);
            let tree = DungeonTree::generate(&mut rng);
            let grid = DungeonGrid::from_tree(&tree);
            let ids = grid.placed_room_ids();
            let unique: std::collections::HashSet<_> = ids.iter().copied().collect();
            assert_eq!(
                ids.len(),
                unique.len(),
                "Duplicate room IDs for seed {}",
                seed
            );
        }
    }
}

// ── Switch / door parity ──────────────────────────────────────────────────────
#[cfg(test)]
mod switch_tests {
    use crate::content::{SwitchDoor, SwitchState};

    fn sw(id: u32, signals: &[u32]) -> SwitchState {
        SwitchState {
            switch_id: id,
            signals: signals.to_vec(),
        }
    }
    fn door(signals: &[u32]) -> SwitchDoor {
        SwitchDoor {
            door_id: 0,
            signals: signals.to_vec(),
        }
    }

    #[test]
    fn door_closed_with_no_switches_activated() {
        let d = door(&[0]);
        let switches = vec![sw(0, &[0])];
        assert!(!d.is_open(0b0, &switches));
    }

    #[test]
    fn door_opens_after_one_activation() {
        let d = door(&[0]);
        let switches = vec![sw(0, &[0])];
        assert!(d.is_open(0b1, &switches)); // switch 0 activated
    }

    #[test]
    fn door_closes_again_after_second_activation() {
        // XOR parity: two activations → even → closed
        let d = door(&[0]);
        let switches = vec![sw(0, &[0]), sw(1, &[0])];
        assert!(!d.is_open(0b11, &switches)); // both active → parity even
    }

    #[test]
    fn n_to_one_combo_lock() {
        // Three switches on signal 5 — door opens only when odd count activated
        let d = door(&[5]);
        let switches = vec![sw(0, &[5]), sw(1, &[5]), sw(2, &[5])];
        assert!(!d.is_open(0b000, &switches)); // 0 active
        assert!(d.is_open(0b001, &switches)); // 1 active
        assert!(!d.is_open(0b011, &switches)); // 2 active
        assert!(d.is_open(0b111, &switches)); // 3 active
    }

    #[test]
    fn one_to_n_master_switch() {
        // One switch, two doors sharing the same signal
        let d1 = door(&[7]);
        let d2 = door(&[7]);
        let switches = vec![sw(0, &[7])];
        let mask = 0b1u64;
        assert!(d1.is_open(mask, &switches));
        assert!(d2.is_open(mask, &switches));
    }

    #[test]
    fn unrelated_switch_does_not_open_door() {
        let d = door(&[3]);
        let switches = vec![sw(0, &[9])]; // signal 9, not 3
        assert!(!d.is_open(0b1, &switches));
    }
}

// ── Content placement ─────────────────────────────────────────────────────────
#[cfg(test)]
mod content_tests {
    use crate::content::{place_content, ContentConfig};
    use crate::grid::DungeonGrid;
    use crate::pathfinding::find_goal_in_grid;
    use crate::rng::Rng;
    use crate::tree::DungeonTree;

    fn make(
        seed: u64,
    ) -> (
        DungeonTree,
        DungeonGrid,
        Vec<crate::content::RoomContent>,
        usize,
    ) {
        let mut rng = Rng::new(seed);
        let tree = DungeonTree::generate(&mut rng);
        let grid = DungeonGrid::from_tree(&tree);
        let goal = find_goal_in_grid(&tree, Some(&grid));
        let cfg = ContentConfig::default();
        let mut tree = tree;
        let mut grid = grid;
        let (content, exit_id) = place_content(&mut tree, &mut grid, goal, &cfg, &mut rng);
        (tree, grid, content, exit_id)
    }

    #[test]
    fn boss_on_goal_room() {
        for seed in 0u64..20 {
            let mut rng = Rng::new(seed);
            let tree = DungeonTree::generate(&mut rng);
            let grid = DungeonGrid::from_tree(&tree);
            let goal = find_goal_in_grid(&tree, Some(&grid));
            let cfg = ContentConfig {
                target_enemy_groups: 2,
                ..Default::default()
            };
            let mut tree = tree;
            let mut grid = grid;
            let (content, _) = place_content(&mut tree, &mut grid, goal, &cfg, &mut rng);
            // goal is always placed (find_goal_in_grid guarantees it)
            let eg = content.get(goal).and_then(|c| c.enemies.as_ref());
            assert!(
                eg.is_some_and(|e| e.is_boss),
                "Goal room {} should have boss (seed {})",
                goal,
                seed
            );
        }
    }

    #[test]
    fn enemies_not_on_root() {
        for seed in 0u64..20 {
            let (tree, _, content, _) = make(seed);
            let has_enemies_on_root = content
                .get(tree.root)
                .and_then(|c| c.enemies.as_ref())
                .is_some_and(|e| !e.is_boss);
            assert!(
                !has_enemies_on_root,
                "Root should never have enemy group (seed {})",
                seed
            );
        }
    }

    #[test]
    fn switch_door_signals_non_empty() {
        for seed in 0u64..30 {
            let (_, _, content, _) = make(seed);
            for c in &content {
                if let Some(door) = &c.switch_door {
                    assert!(!door.signals.is_empty(), "SwitchDoor has empty signal list");
                }
                if let Some(sw) = &c.switch {
                    assert!(!sw.signals.is_empty(), "SwitchState has empty signal list");
                }
            }
        }
    }

    #[test]
    fn signs_have_nonempty_text() {
        for seed in 0u64..20 {
            let (_, _, content, _) = make(seed);
            for c in &content {
                if let Some(sign) = &c.sign {
                    assert!(!sign.text.is_empty(), "Sign has empty text");
                }
            }
        }
    }

    #[test]
    fn content_vec_length_matches_rooms() {
        for seed in 0u64..10 {
            let (tree, _, content, _) = make(seed);
            assert_eq!(
                content.len(),
                tree.rooms.len(),
                "Content vec length mismatch for seed {}",
                seed
            );
        }
    }
}

// ── Pathfinding with switches ─────────────────────────────────────────────────
#[cfg(test)]
mod pathfinding_tests {
    use crate::content::{place_content, ContentConfig};
    use crate::grid::DungeonGrid;
    use crate::pathfinding::{
        astar_locks_opened_with_content, dfs_avg_rooms_visited_with_content, find_goal_in_grid,
    };
    use crate::rng::Rng;
    use crate::tree::DungeonTree;

    #[test]
    fn astar_reaches_goal_when_reachable() {
        for seed in 0u64..30 {
            let mut rng = Rng::new(seed);
            let tree = DungeonTree::generate(&mut rng);
            let grid = DungeonGrid::from_tree(&tree);
            let mut tree = tree;
            let mut grid = grid;
            tree.repair_orphaned_locks(&grid);
            let goal = find_goal_in_grid(&tree, Some(&grid));
            let cfg = ContentConfig::default();
            let (content, exit_id) = place_content(&mut tree, &mut grid, goal, &cfg, &mut rng);
            // A* must reach the exit (through boss room)
            let result =
                astar_locks_opened_with_content(&tree, &grid, tree.root, exit_id, &content);
            assert!(
                result.is_some(),
                "A* could not reach exit for seed {}",
                seed
            );
        }
    }

    #[test]
    fn dfs_visits_positive_rooms() {
        let mut rng = Rng::new(7);
        let tree = DungeonTree::generate(&mut rng);
        let grid = DungeonGrid::from_tree(&tree);
        let avg = dfs_avg_rooms_visited_with_content(&tree, &grid, tree.root, &[], &mut rng);
        assert!(avg >= 1.0, "DFS should visit at least the start room");
    }
}

// ── Fitness ───────────────────────────────────────────────────────────────────
#[cfg(test)]
mod fitness_tests {
    use crate::fitness::{evaluate, FitnessTargets};
    use crate::grid::DungeonGrid;
    use crate::rng::Rng;
    use crate::tree::DungeonTree;

    #[test]
    fn fitness_non_negative() {
        let mut rng = Rng::new(77);
        let tree = DungeonTree::generate(&mut rng);
        let grid = DungeonGrid::from_tree(&tree);
        let targets = FitnessTargets {
            rooms: 15.0,
            keys: 3.0,
            locks: 2.0,
            switches: 2.0,
            linearity: 2.0,
        };
        let r = evaluate(&tree, &grid, &targets, &[], &mut rng);
        assert!(r.total >= 0.0);
    }
}

// ── Evolution (end-to-end) ────────────────────────────────────────────────────
#[cfg(test)]
mod evolution_tests {
    use crate::evolution::{generate, DungeonConfig};

    fn cfg(seed: u64) -> DungeonConfig {
        DungeonConfig::new(15, 3, 2, 2.0, seed)
    }

    #[test]
    fn deterministic_same_seed() {
        let a = generate(&cfg(123));
        let b = generate(&cfg(123));
        assert_eq!(
            a.tree.placed_room_count(&a.grid),
            b.tree.placed_room_count(&b.grid)
        );
        assert!((a.fitness.total - b.fitness.total).abs() < 1e-9);
    }

    #[test]
    fn fitness_reasonable_small_config() {
        let d = generate(&cfg(42));
        assert!(
            d.fitness.total < 15.0,
            "Fitness {} unexpectedly high for small config",
            d.fitness.total
        );
    }

    #[test]
    fn room_count_near_target() {
        let d = generate(&DungeonConfig::new(20, 4, 4, 1.5, 7));
        let dr = d.tree.placed_room_count(&d.grid) as i32;
        assert!((dr - 20).abs() <= 5, "Expected ~20 rooms, got {}", dr);
    }

    #[test]
    fn content_populated_after_generate() {
        let d = generate(&cfg(5));
        // Boss must be present somewhere
        let has_boss = d
            .content
            .iter()
            .any(|c| c.enemies.as_ref().is_some_and(|e| e.is_boss));
        assert!(has_boss, "Generated dungeon should have a boss");
    }

    #[test]
    fn exit_room_exists_and_is_placed() {
        for seed in 0u64..10 {
            let d = generate(&DungeonConfig::new(15, 3, 2, 2.0, seed));
            // exit_room_id must be a real room in the arena
            assert!(
                d.exit_room_id < d.tree.rooms.len(),
                "exit_room_id {} out of range (seed {})",
                d.exit_room_id,
                seed
            );
            // it must be placed in the grid
            assert!(
                d.grid.pos_of(d.exit_room_id).is_some(),
                "Exit room {} not placed in grid (seed {})",
                d.exit_room_id,
                seed
            );
            // it must be tagged is_exit in content
            assert!(
                d.content[d.exit_room_id].is_exit,
                "Exit room {} not tagged is_exit (seed {})",
                d.exit_room_id, seed
            );
        }
    }

    #[test]
    fn exit_is_child_of_boss_room() {
        for seed in 0u64..10 {
            let d = generate(&DungeonConfig::new(15, 3, 2, 2.0, seed));
            // The boss room must be the parent of the exit room
            let parent = d.tree.parent_of(d.exit_room_id);
            let parent_is_boss = parent.is_some_and(|pid| {
                d.content
                    .get(pid)
                    .and_then(|c| c.enemies.as_ref())
                    .is_some_and(|e| e.is_boss)
            });
            // When exit_room_id != goal fallback (boss always placed), parent must be boss
            if d.exit_room_id != d.tree.root {
                assert!(
                    parent_is_boss,
                    "Exit room {}'s parent is not the boss (seed {})",
                    d.exit_room_id, seed
                );
            }
        }
    }

    #[test]
    fn exactly_one_exit_per_dungeon() {
        for seed in 0u64..20 {
            let d = generate(&DungeonConfig::new(15, 3, 2, 2.0, seed));
            let exits = d.content.iter().filter(|c| c.is_exit).count();
            assert_eq!(
                exits, 1,
                "Expected exactly 1 exit, found {} (seed {})",
                exits, seed
            );
        }
    }

    #[test]
    fn ascii_map_contains_exit_and_boss_markers() {
        let d = generate(&cfg(9));
        let map = d.ascii_map_with_content();
        assert!(map.contains('B'), "ASCII map should show boss 'B'");
        assert!(map.contains('X'), "ASCII map should show exit 'X'");
    }

    #[test]
    fn signs_accessible_via_helper() {
        let d = generate(&cfg(3));
        let signs = d.signs();
        for (_, _, sign) in &signs {
            assert!(!sign.text.is_empty());
        }
    }
}

// ── Layout / tile generation ──────────────────────────────────────────────────
#[cfg(test)]
mod layout_tests {
    use crate::evolution::{generate, DungeonConfig};
    use crate::layout::{TileKind, CELL_SIZE, TOTAL_TILES};
    use std::collections::HashSet;

    fn gen(seed: u64) -> crate::evolution::GeneratedDungeon {
        generate(&DungeonConfig::new(15, 3, 2, 2.0, seed))
    }

    // ── Structural invariants ────────────────────────────────────────────

    #[test]
    fn tile_room_count_matches_placed_rooms() {
        let d = gen(1);
        assert_eq!(
            d.tile_map.rooms.len(),
            d.grid.placed_room_ids().len(),
            "TileMap should have one TileRoom per placed room"
        );
    }

    #[test]
    fn outer_ring_is_all_wall() {
        // The outer ring is walls except at door positions (5,0), (5,10), (0,5), (10,5)
        // which become Door/LockedDoor/SwitchDoor tiles when there is a neighbour.
        let door_cols_rows: &[(usize, usize)] = &[(5, 0), (5, 10), (0, 5), (10, 5)];
        let d = gen(2);
        for tr in &d.tile_map.rooms {
            for col in 0..CELL_SIZE {
                for row in [0usize, 10] {
                    let t = tr.get(col, row);
                    let is_door_pos = door_cols_rows.contains(&(col, row));
                    if is_door_pos {
                        // Must be Wall or a door-kind
                        assert!(
                            matches!(
                                t,
                                TileKind::Wall
                                    | TileKind::Door
                                    | TileKind::LockedDoor
                                    | TileKind::SwitchDoor
                            ),
                            "Room {} bad tile {:?} at ({},{})",
                            tr.room_id,
                            t,
                            col,
                            row
                        );
                    } else {
                        assert_eq!(
                            t,
                            TileKind::Wall,
                            "Room {} non-door outer tile {:?} at ({},{})",
                            tr.room_id,
                            t,
                            col,
                            row
                        );
                    }
                }
            }
            for row in 0..CELL_SIZE {
                for col in [0usize, 10] {
                    let t = tr.get(col, row);
                    let is_door_pos = door_cols_rows.contains(&(col, row));
                    if is_door_pos {
                        assert!(
                            matches!(
                                t,
                                TileKind::Wall
                                    | TileKind::Door
                                    | TileKind::LockedDoor
                                    | TileKind::SwitchDoor
                            ),
                            "Room {} bad tile {:?} at ({},{})",
                            tr.room_id,
                            t,
                            col,
                            row
                        );
                    } else {
                        assert_eq!(
                            t,
                            TileKind::Wall,
                            "Room {} non-door outer tile {:?} at ({},{})",
                            tr.room_id,
                            t,
                            col,
                            row
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn interior_has_no_outer_wall_tiles() {
        // After template application, inner tiles should not be Wall
        // (they can be Pillar/Water/Pit, but not Wall)
        let d = gen(3);
        for tr in &d.tile_map.rooms {
            for row in 1..=9 {
                for col in 1..=9 {
                    assert_ne!(
                        tr.get(col, row),
                        TileKind::Wall,
                        "Room {} has Wall at interior ({},{})",
                        tr.room_id,
                        col,
                        row
                    );
                }
            }
        }
    }

    #[test]
    fn obstacle_density_in_range() {
        let d = gen(4);
        let interior = (crate::layout::INNER * crate::layout::INNER) as f64;
        for tr in &d.tile_map.rooms {
            let obstacles = (1..=9usize)
                .flat_map(|row| (1..=9usize).map(move |col| (col, row)))
                .filter(|&(col, row)| {
                    matches!(
                        tr.get(col, row),
                        TileKind::Pillar | TileKind::Water | TileKind::Pit
                    )
                })
                .count();
            let density = obstacles as f64 / interior;
            assert!(
                density <= 0.30,
                "Room {} obstacle density {:.2} exceeds max 0.25+margin",
                tr.room_id,
                density
            );
        }
    }

    // ── Door alignment ───────────────────────────────────────────────────

    #[test]
    fn door_tiles_align_between_adjacent_rooms() {
        let d = gen(5);
        for tr in &d.tile_map.rooms {
            let grid_pos = d.grid.pos_of(tr.room_id).unwrap();
            let neighbours = d.grid.neighbours(tr.room_id);
            for nb_id in neighbours {
                let nb_pos = d.grid.pos_of(nb_id).unwrap();
                let dx = nb_pos.0 - grid_pos.0;
                let dy = nb_pos.1 - grid_pos.1;

                // Which wall of this room connects to neighbour
                let (my_dc, my_dr, nb_dc, nb_dr) = match (dx, dy) {
                    (1, 0) => (10, 5, 0, 5),  // east → neighbour's west
                    (-1, 0) => (0, 5, 10, 5), // west → neighbour's east
                    (0, 1) => (5, 10, 5, 0),  // south → neighbour's north
                    (0, -1) => (5, 0, 5, 10), // north → neighbour's south
                    _ => continue,
                };

                let my_tile = tr.get(my_dc, my_dr);
                let nb_room = d.tile_map.room_for(nb_id).unwrap();
                let nb_tile = nb_room.get(nb_dc, nb_dr);

                let my_is_door = !matches!(my_tile, TileKind::Wall);
                let nb_is_door = !matches!(nb_tile, TileKind::Wall);
                assert_eq!(
                    my_is_door, nb_is_door,
                    "Door mismatch between rooms {} and {}: my={:?} nb={:?}",
                    tr.room_id, nb_id, my_tile, nb_tile
                );
            }
        }
    }

    /// The tile kind must be identical on both sides of every connection.
    ///
    /// A wall between rooms A and B carries exactly one door (or no door);
    /// whichever tile kind is chosen, both A's wall and B's mirror wall must
    /// agree.  Failures here indicate that the door-kind selection logic uses
    /// different rules depending on which room is being built, producing an
    /// asymmetric pair such as (SwitchDoor, Door) or (LockedDoor, Door).
    #[test]
    fn door_tile_kinds_match_between_adjacent_rooms() {
        for seed in 0u64..50 {
            let d = gen(seed);
            for tr in &d.tile_map.rooms {
                let grid_pos = d.grid.pos_of(tr.room_id).unwrap();
                for nb_id in d.grid.neighbours(tr.room_id) {
                    let nb_pos = d.grid.pos_of(nb_id).unwrap();
                    let dx = nb_pos.0 - grid_pos.0;
                    let dy = nb_pos.1 - grid_pos.1;
                    let (my_dc, my_dr, nb_dc, nb_dr) = match (dx, dy) {
                        (1, 0) => (10usize, 5usize, 0usize, 5usize),
                        (-1, 0) => (0, 5, 10, 5),
                        (0, 1) => (5, 10, 5, 0),
                        (0, -1) => (5, 0, 5, 10),
                        _ => continue,
                    };
                    let my_tile = tr.get(my_dc, my_dr);
                    let nb_tile = d.tile_map.room_for(nb_id).unwrap().get(nb_dc, nb_dr);
                    assert_eq!(
                        my_tile,
                        nb_tile,
                        "Door kind mismatch between rooms {} ({:?}) and {} ({:?}): \
                         {:?} vs {:?} (seed {})",
                        tr.room_id,
                        d.tree.rooms[tr.room_id].kind,
                        nb_id,
                        d.tree.rooms[nb_id].kind,
                        my_tile,
                        nb_tile,
                        seed
                    );
                }
            }
        }
    }

    // ── Content placement ────────────────────────────────────────────────

    #[test]
    fn exactly_one_spawn_tile_in_root_room() {
        for seed in 0u64..10 {
            let d = gen(seed);
            let root_tr = d.tile_map.room_for(d.tree.root).unwrap();
            let spawns = (0..TOTAL_TILES)
                .filter(|&i| root_tr.tiles[i] == TileKind::Spawn)
                .count();
            assert_eq!(
                spawns, 1,
                "Expected 1 Spawn in root room, got {} (seed {})",
                spawns, seed
            );
        }
    }

    #[test]
    fn exactly_one_stairs_tile_in_exit_room() {
        for seed in 0u64..10 {
            let d = gen(seed);
            let exit_tr = d.tile_map.room_for(d.exit_room_id).unwrap();
            let stairs = (0..TOTAL_TILES)
                .filter(|&i| exit_tr.tiles[i] == TileKind::Stairs)
                .count();
            assert_eq!(
                stairs, 1,
                "Expected 1 Stairs in exit room, got {} (seed {})",
                stairs, seed
            );
        }
    }

    #[test]
    fn boss_room_has_boss_spawn() {
        for seed in 0u64..10 {
            let d = gen(seed);
            let boss_id = d
                .content
                .iter()
                .enumerate()
                .find(|(_, c)| c.enemies.as_ref().is_some_and(|e| e.is_boss))
                .map(|(id, _)| id);
            if let Some(bid) = boss_id {
                if let Some(boss_tr) = d.tile_map.room_for(bid) {
                    let bosses = (0..TOTAL_TILES)
                        .filter(|&i| boss_tr.tiles[i] == TileKind::BossSpawn)
                        .count();
                    assert_eq!(
                        bosses, 1,
                        "Expected 1 BossSpawn in boss room, got {} (seed {})",
                        bosses, seed
                    );
                }
            }
        }
    }

    #[test]
    fn key_rooms_have_chest_tile() {
        for seed in 0u64..10 {
            let d = gen(seed);
            let placed: HashSet<usize> = d.grid.placed_room_ids().into_iter().collect();
            for room in &d.tree.rooms {
                if !placed.contains(&room.id) {
                    continue;
                }
                if !room.kind.is_key() {
                    continue;
                }
                let tr = d.tile_map.room_for(room.id).unwrap();
                let chests = (0..TOTAL_TILES)
                    .filter(|&i| tr.tiles[i] == TileKind::Chest)
                    .count();
                assert_eq!(
                    chests, 1,
                    "Key room {} should have exactly 1 Chest, got {} (seed {})",
                    room.id, chests, seed
                );
            }
        }
    }

    #[test]
    fn all_content_tiles_are_reachable() {
        // Every interactive tile must be reachable from a door tile
        let content_kinds: &[TileKind] = &[
            TileKind::Chest,
            TileKind::SwitchTile,
            TileKind::Sign,
            TileKind::Stairs,
            TileKind::EnemySpawn,
            TileKind::BossSpawn,
            TileKind::Spawn,
        ];
        for seed in 0u64..10 {
            let d = gen(seed);
            for tr in &d.tile_map.rooms {
                let doors: Vec<(usize, usize)> = [(5usize, 0usize), (5, 10), (10, 5), (0, 5)]
                    .iter()
                    .filter(|&&(c, r)| tr.get(c, r) != TileKind::Wall)
                    .copied()
                    .collect();

                // Use root room's spawn as starting point if no doors (isolated)
                let start_region = if doors.is_empty() {
                    // Flood from centre
                    let mut h = HashSet::new();
                    h.insert((5, 5));
                    h
                } else {
                    // Flood from one step inside each door
                    let mut reachable = HashSet::new();
                    for &(dc, dr) in &doors {
                        for (dx, dy) in [(0i32, 1), (0, -1), (1, 0), (-1, 0)] {
                            let nc = dc as i32 + dx;
                            let nr = dr as i32 + dy;
                            if (1..=9).contains(&nc)
                                && (1..=9).contains(&nr)
                                && tr.get(nc as usize, nr as usize).is_walkable()
                            {
                                reachable.extend(crate::tests::layout_tests_helper::flood(
                                    &tr.tiles,
                                    (nc as usize, nr as usize),
                                ));
                                break;
                            }
                        }
                    }
                    reachable
                };

                for i in 0..TOTAL_TILES {
                    if content_kinds.contains(&tr.tiles[i]) {
                        let col = i % CELL_SIZE;
                        let row = i / CELL_SIZE;
                        assert!(
                            start_region.contains(&(col, row)),
                            "Room {} tile {:?} at ({},{}) is unreachable (seed {})",
                            tr.room_id,
                            tr.tiles[i],
                            col,
                            row,
                            seed
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn world_origin_matches_grid_position() {
        let d = gen(7);
        for tr in &d.tile_map.rooms {
            let gp = d.grid.pos_of(tr.room_id).unwrap();
            let expected = (gp.0 * CELL_SIZE as i32, gp.1 * CELL_SIZE as i32);
            assert_eq!(
                tr.world_origin, expected,
                "Room {} world_origin mismatch",
                tr.room_id
            );
        }
    }

    // ── Locked / SwitchDoor tile correctness ─────────────────────────────

    /// A Locked room's tile on the wall facing its tree parent must be
    /// `LockedDoor`, not a plain `Door`.  This is the regression test for the
    /// bug where the neighbour loop used the parent's (Normal) kind instead of
    /// the child's (Locked) kind when setting that wall tile.
    #[test]
    fn locked_room_has_locked_door_toward_parent() {
        let placed_any = std::cell::Cell::new(false);
        for seed in 0u64..50 {
            let d = gen(seed);
            let placed: HashSet<usize> = d.grid.placed_room_ids().into_iter().collect();
            for room in &d.tree.rooms {
                if !placed.contains(&room.id) || !room.kind.is_locked() {
                    continue;
                }
                let Some(parent_id) = d.tree.parent_of(room.id) else {
                    continue;
                };
                if !placed.contains(&parent_id) {
                    continue;
                }
                let room_pos = d.grid.pos_of(room.id).unwrap();
                let parent_pos = d.grid.pos_of(parent_id).unwrap();
                let dx = parent_pos.0 - room_pos.0;
                let dy = parent_pos.1 - room_pos.1;
                let (dc, dr) = match (dx, dy) {
                    (1, 0) => (10usize, 5usize),
                    (-1, 0) => (0, 5),
                    (0, 1) => (5, 10),
                    (0, -1) => (5, 0),
                    _ => continue,
                };
                placed_any.set(true);
                let tr = d.tile_map.room_for(room.id).unwrap();
                let tile = tr.get(dc, dr);
                assert_eq!(
                    tile,
                    TileKind::LockedDoor,
                    "Locked room {} toward parent {}: expected LockedDoor, got {:?} (seed {})",
                    room.id,
                    parent_id,
                    tile,
                    seed
                );
            }
        }
        assert!(placed_any.get(), "No placed Locked rooms found across 50 seeds");
    }

    /// Both sides of a parent↔Locked-child passage must show `LockedDoor`.
    #[test]
    fn locked_door_tiles_are_symmetric() {
        for seed in 0u64..50 {
            let d = gen(seed);
            let placed: HashSet<usize> = d.grid.placed_room_ids().into_iter().collect();
            for room in &d.tree.rooms {
                if !placed.contains(&room.id) || !room.kind.is_locked() {
                    continue;
                }
                let Some(parent_id) = d.tree.parent_of(room.id) else {
                    continue;
                };
                if !placed.contains(&parent_id) {
                    continue;
                }
                let room_pos = d.grid.pos_of(room.id).unwrap();
                let parent_pos = d.grid.pos_of(parent_id).unwrap();
                let dx = parent_pos.0 - room_pos.0;
                let dy = parent_pos.1 - room_pos.1;
                let (child_dc, child_dr, par_dc, par_dr) = match (dx, dy) {
                    (1, 0) => (10usize, 5usize, 0usize, 5usize),
                    (-1, 0) => (0, 5, 10, 5),
                    (0, 1) => (5, 10, 5, 0),
                    (0, -1) => (5, 0, 5, 10),
                    _ => continue,
                };
                let child_tile = d.tile_map.room_for(room.id).unwrap().get(child_dc, child_dr);
                let par_tile = d
                    .tile_map
                    .room_for(parent_id)
                    .unwrap()
                    .get(par_dc, par_dr);
                assert_eq!(
                    child_tile,
                    TileKind::LockedDoor,
                    "Locked room {} child-side tile should be LockedDoor, got {:?} (seed {})",
                    room.id,
                    child_tile,
                    seed
                );
                assert_eq!(
                    par_tile,
                    TileKind::LockedDoor,
                    "Parent {} toward Locked child {}: expected LockedDoor, got {:?} (seed {})",
                    parent_id,
                    room.id,
                    par_tile,
                    seed
                );
            }
        }
    }

    /// A `RoomKind::SwitchDoor` room's tile on the wall facing its tree parent
    /// must be `TileKind::SwitchDoor`.
    #[test]
    fn switch_door_room_has_switch_door_toward_parent() {
        for seed in 0u64..50 {
            let d = gen(seed);
            let placed: HashSet<usize> = d.grid.placed_room_ids().into_iter().collect();
            for room in &d.tree.rooms {
                if !placed.contains(&room.id) || !room.kind.is_switch_door() {
                    continue;
                }
                let Some(parent_id) = d.tree.parent_of(room.id) else {
                    continue;
                };
                if !placed.contains(&parent_id) {
                    continue;
                }
                let room_pos = d.grid.pos_of(room.id).unwrap();
                let parent_pos = d.grid.pos_of(parent_id).unwrap();
                let dx = parent_pos.0 - room_pos.0;
                let dy = parent_pos.1 - room_pos.1;
                let (dc, dr) = match (dx, dy) {
                    (1, 0) => (10usize, 5usize),
                    (-1, 0) => (0, 5),
                    (0, 1) => (5, 10),
                    (0, -1) => (5, 0),
                    _ => continue,
                };
                let tr = d.tile_map.room_for(room.id).unwrap();
                let tile = tr.get(dc, dr);
                assert_eq!(
                    tile,
                    TileKind::SwitchDoor,
                    "SwitchDoor room {} toward parent {}: expected SwitchDoor, got {:?} (seed {})",
                    room.id,
                    parent_id,
                    tile,
                    seed
                );
            }
        }
    }

    #[test]
    fn ascii_map_has_correct_dimensions() {
        let d = gen(8);
        let ascii = d.tile_map.ascii();
        let lines: Vec<&str> = ascii.lines().collect();
        let expected_h = d.tile_map.height() as usize;
        let expected_w = d.tile_map.width() as usize;
        assert_eq!(
            lines.len(),
            expected_h,
            "ASCII height {} != expected {}",
            lines.len(),
            expected_h
        );
        for line in &lines {
            assert_eq!(
                line.len(),
                expected_w,
                "ASCII line width {} != expected {}",
                line.len(),
                expected_w
            );
        }
    }
}

#[cfg(test)]
mod layout_tests_helper {
    use crate::layout::{TileKind, CELL_SIZE, TOTAL_TILES};
    use std::collections::{HashSet, VecDeque};

    pub fn flood(
        tiles: &[TileKind; TOTAL_TILES],
        start: (usize, usize),
    ) -> HashSet<(usize, usize)> {
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
}
