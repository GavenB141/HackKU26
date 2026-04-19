//! Constrained Evolutionary Algorithm (CEA) — paper Fig. 5 and §4, extended.

use crate::content::{place_content, ContentConfig, RoomContent};
use crate::fitness::{evaluate, FitnessResult, FitnessTargets};
use crate::grid::DungeonGrid;
use crate::layout::TileMap;
use crate::pathfinding::find_goal_in_grid;
use crate::rng::Rng;
use crate::tree::DungeonTree;

// ── EA defaults (paper §6) ─────────────────────────────────────────────────
const POPULATION_SIZE: usize = 100;
const N_GENERATIONS: usize = 100;
const CROSSOVER_RATE: f64 = 0.90;
const MUTATION_RATE: f64 = 0.05;
const TOURNAMENT_SIZE: usize = 2;

// ── Configuration ──────────────────────────────────────────────────────────

/// Full configuration for the dungeon generator.
#[derive(Clone, Debug)]
pub struct DungeonConfig {
    // ── Structural targets (paper) ────────────────────────────────────
    pub target_rooms: u32,
    pub target_keys: u32,
    pub target_locks: u32,
    pub target_linearity: f64,

    // ── Extended targets ──────────────────────────────────────────────
    /// Desired number of switch/switch-door *pairs*.
    pub target_switch_pairs: u32,

    // ── EA parameters ────────────────────────────────────────────────
    pub population_size: usize,
    pub n_generations: usize,
    pub crossover_rate: f64,
    pub mutation_rate: f64,
    pub tournament_size: usize,

    // ── Content layer ─────────────────────────────────────────────────
    /// Controls enemy placement, signs, etc. after evolution.
    pub content: ContentConfig,

    pub seed: u64,
}

impl DungeonConfig {
    /// Construct with paper defaults.
    pub fn new(
        target_rooms: u32,
        target_keys: u32,
        target_locks: u32,
        target_linearity: f64,
        seed: u64,
    ) -> Self {
        DungeonConfig {
            target_rooms,
            target_keys,
            target_locks,
            target_linearity,
            target_switch_pairs: 2,
            population_size: POPULATION_SIZE,
            n_generations: N_GENERATIONS,
            crossover_rate: CROSSOVER_RATE,
            mutation_rate: MUTATION_RATE,
            tournament_size: TOURNAMENT_SIZE,
            content: ContentConfig::default(),
            seed,
        }
    }

    /// Construct with most of the parameters set according to the paper.
    pub fn defaults_with_seed(seed: u64) -> Self {
        Self::new(18, 4, 4, 1.89, seed)
    }
}

// ── Output ─────────────────────────────────────────────────────────────────

/// The best dungeon produced by the CEA, with content fully placed.
#[derive(Debug)]
pub struct GeneratedDungeon {
    pub tree: DungeonTree,
    pub grid: DungeonGrid,
    pub fitness: FitnessResult,
    /// Content annotations indexed by room ID.
    pub content: Vec<RoomContent>,
    /// The exit room: always a child of the boss room, tagged `is_exit`.
    pub exit_room_id: usize,
    /// Full tile-level layout for all placed rooms.
    pub tile_map: TileMap,
    pub best_generation: usize,
}

impl GeneratedDungeon {
    /// ASCII map with a legend.  `E`=enemy, `B`=boss, `W`=switch, `D`=switch-door,
    /// `$`=sign, `S`=spawn, `K`=key, `L`=locked, `.`=normal, ` `=empty.
    pub fn print_map(&self) {
        println!("{}", self.ascii_map_with_content());
        let p = &self.tree;
        let g = &self.grid;
        println!(
            "Rooms:{} Keys:{} Locks:{} Switches:{}/{} Exit:{} Lin:{:.2} Fitness:{:.4}",
            p.placed_room_count(g),
            p.placed_key_count(g),
            p.placed_lock_count(g),
            p.placed_switch_count(g),
            p.placed_switch_door_count(g),
            self.exit_room_id,
            self.fitness.linearity,
            self.fitness.total,
        );
    }

    /// ASCII grid that overlays content annotations.
    pub fn ascii_map_with_content(&self) -> String {
        use crate::room::RoomKind;
        let grid = &self.grid;
        let tree = &self.tree;
        let content = &self.content;

        let w = (grid.max_x - grid.min_x + 1) as usize;
        let h = (grid.max_y - grid.min_y + 1) as usize;
        let mut map = vec![vec![' '; w]; h];

        for (&(x, y), cell) in &grid.cells {
            let col = (x - grid.min_x) as usize;
            let row = (y - grid.min_y) as usize;
            let id = cell.room_id;

            let ch = if id == tree.root {
                'S'
            } else {
                let is_exit = content.get(id).is_some_and(|c| c.is_exit);
                let has_boss = content
                    .get(id)
                    .and_then(|c| c.enemies.as_ref())
                    .is_some_and(|e| e.is_boss);
                let has_enemy = content
                    .get(id)
                    .and_then(|c| c.enemies.as_ref())
                    .is_some_and(|e| !e.is_boss);
                let has_sign = content.get(id).is_some_and(|c| c.sign.is_some());

                match tree.rooms[id].kind {
                    _ if is_exit => 'X',
                    _ if has_boss => 'B',
                    _ if has_enemy => 'E',
                    RoomKind::Key { .. } => 'K',
                    RoomKind::Locked { .. } => 'L',
                    RoomKind::Switch => 'W',
                    RoomKind::SwitchDoor => 'D',
                    _ if has_sign => '$',
                    RoomKind::Normal => '.',
                }
            };
            map[row][col] = ch;
        }

        map.into_iter()
            .map(|row| row.into_iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// All signs in the dungeon, with their room positions.
    pub fn signs(&self) -> Vec<(i32, i32, &crate::content::Sign)> {
        let mut out = Vec::new();
        for (id, c) in self.content.iter().enumerate() {
            if let Some(sign) = &c.sign {
                if let Some(pos) = self.grid.pos_of(id) {
                    out.push((pos.0, pos.1, sign));
                }
            }
        }
        out
    }

    /// All enemy groups with their room positions.
    pub fn enemies(&self) -> Vec<(i32, i32, &crate::content::EnemyGroup)> {
        let mut out = Vec::new();
        for (id, c) in self.content.iter().enumerate() {
            if let Some(eg) = &c.enemies {
                if let Some(pos) = self.grid.pos_of(id) {
                    out.push((pos.0, pos.1, eg));
                }
            }
        }
        out
    }
}

// ── Internal individual ────────────────────────────────────────────────────

struct Individual {
    tree: DungeonTree,
    grid: DungeonGrid,
    fitness: f64,
}

impl Individual {
    fn new(mut tree: DungeonTree, targets: &FitnessTargets, rng: &mut Rng) -> Self {
        let grid = DungeonGrid::from_tree(&tree);
        tree.repair_orphaned_locks(&grid);
        let fit = evaluate(&tree, &grid, targets, &[], rng);
        Individual {
            tree,
            grid,
            fitness: fit.total,
        }
    }
}

// ── CEA ────────────────────────────────────────────────────────────────────

pub fn generate(config: &DungeonConfig) -> GeneratedDungeon {
    let mut rng = Rng::new(config.seed);

    let targets = FitnessTargets {
        rooms: config.target_rooms as f64,
        keys: config.target_keys as f64,
        locks: config.target_locks as f64,
        switches: config.target_switch_pairs as f64 * 2.0, // switch + door each
        linearity: config.target_linearity,
    };

    // Initialise population
    let mut pop: Vec<Individual> = (0..config.population_size)
        .map(|_| Individual::new(DungeonTree::generate(&mut rng), &targets, &mut rng))
        .collect();

    let mut best_idx = best_index(&pop);
    let mut best_gen = 0usize;

    // Main EA loop
    for gen in 0..config.n_generations {
        let mut new_pop: Vec<Individual> = Vec::with_capacity(config.population_size);

        // Elitism
        new_pop.push(Individual::new(
            pop[best_idx].tree.clone(),
            &targets,
            &mut rng,
        ));

        while new_pop.len() < config.population_size {
            let p1 = tournament_select(&pop, config.tournament_size, &mut rng);
            let p2 = tournament_select(&pop, config.tournament_size, &mut rng);

            let mut c1 = pop[p1].tree.clone();
            let mut c2 = pop[p2].tree.clone();

            if rng.next_f64() < config.crossover_rate {
                DungeonTree::crossover(&mut c1, &mut c2, &mut rng);
            }
            if rng.next_f64() < config.mutation_rate {
                mutate(&mut c1, &mut rng);
            }
            if rng.next_f64() < config.mutation_rate {
                mutate(&mut c2, &mut rng);
            }

            let i1 = Individual::new(c1, &targets, &mut rng);
            let i2 = Individual::new(c2, &targets, &mut rng);
            new_pop.push(i1);
            if new_pop.len() < config.population_size {
                new_pop.push(i2);
            }
        }

        pop = new_pop;
        let nb = best_index(&pop);
        if pop[nb].fitness < pop[best_idx.min(pop.len() - 1)].fitness {
            best_gen = gen;
        }
        best_idx = nb;
    }

    // ── Post-evolution: place content on the winner ───────────────────
    // Clone winner so we can mutably append the exit room.
    let mut tree = pop[best_idx].tree.clone();
    let mut grid = pop[best_idx].grid.clone();
    let goal = find_goal_in_grid(&tree, Some(&grid));
    let (content, exit_room_id) =
        place_content(&mut tree, &mut grid, goal, &config.content, &mut rng);

    // Fitness is evaluated with the exit as the target so the pathfinder scores
    // the full path through the boss room and out the exit.
    let fit_detail = evaluate(&tree, &grid, &targets, &content, &mut rng);
    let tile_map = TileMap::build(&tree, &grid, &content, exit_room_id, &mut rng);

    GeneratedDungeon {
        tree,
        grid,
        fitness: fit_detail,
        content,
        exit_room_id,
        tile_map,
        best_generation: best_gen,
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn best_index(pop: &[Individual]) -> usize {
    pop.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.fitness.partial_cmp(&b.fitness).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn tournament_select(pop: &[Individual], size: usize, rng: &mut Rng) -> usize {
    let mut best = rng.next_usize(pop.len());
    for _ in 1..size {
        let c = rng.next_usize(pop.len());
        if pop[c].fitness < pop[best].fitness {
            best = c;
        }
    }
    best
}

fn mutate(tree: &mut DungeonTree, rng: &mut Rng) {
    // Three equally likely mutation operators
    match rng.next_usize(3) {
        0 => tree.mutate_play_space(rng),
        1 => tree.mutate_mission(rng),
        _ => tree.mutate_switches(rng),
    }
    tree.renumber_keys();
}
