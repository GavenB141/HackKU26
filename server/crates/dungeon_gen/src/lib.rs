//! # Dungeon Generator
//!
//! Seeded procedural dungeon generator based on the Constrained Evolutionary
//! Algorithm (CEA) from Pereira et al. 2021, extended with:
//!
//! * **Switch/door puzzles** — parity-based: a door opens when the XOR-parity
//!   of activated switches sharing its signal IDs is odd.
//! * **Enemies and boss** — enemy groups block rooms (cleared on first entry);
//!   the boss is placed in the goal room and blocks the final passage.
//! * **Flavour-text signs** — contextual hints near keys/locks, and atmospheric
//!   lines in random rooms.

pub mod content;
pub mod evolution;
pub mod fitness;
pub mod grid;
pub mod layout;
pub mod pathfinding;
pub mod rng;
pub mod room;
pub mod tests;
pub mod tree;

pub use content::{
    ContentConfig, EnemyGroup, RoomContent, Sign, SignKind, SwitchDoor, SwitchState,
};
pub use evolution::{generate, DungeonConfig, GeneratedDungeon};
pub use grid::DungeonGrid;
pub use layout::{TileKind, TileMap, TileRoom};
pub use room::{Room, RoomKind};
pub use tree::DungeonTree;
