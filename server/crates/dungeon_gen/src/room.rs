//! Room primitives: kinds, directions, and the `Room` data structure.

// ── Direction ───────────────────────────────────────────────────────────────

/// Which cardinal direction a child room is placed relative to its parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Right, // East
    Left,  // West
    Down,  // South
}

impl Direction {
    pub const ALL: [Direction; 3] = [Direction::Right, Direction::Left, Direction::Down];

    #[inline]
    pub fn offset(self) -> (i32, i32) {
        match self {
            Direction::Right => (1, 0),
            Direction::Left => (-1, 0),
            Direction::Down => (0, 1),
        }
    }

    pub fn rotate_for_parent(self, parent_dir: Direction) -> Direction {
        match parent_dir {
            Direction::Down => self,
            Direction::Right => match self {
                Direction::Right => Direction::Down,
                Direction::Left => Direction::Right,
                Direction::Down => Direction::Left,
            },
            Direction::Left => match self {
                Direction::Right => Direction::Down,
                Direction::Left => Direction::Left,
                Direction::Down => Direction::Right,
            },
        }
    }
}

// ── RoomKind ────────────────────────────────────────────────────────────────

/// Structural type of a room.
///
/// The first three variants implement the paper (§3.1).
/// `Switch` and `SwitchDoor` extend the mission layer with parity puzzles;
/// their signal data lives in [`crate::content::RoomContent`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomKind {
    Normal,
    Key { key_id: u32 },
    Locked { key_id: u32 },
    Switch,
    SwitchDoor,
}

impl RoomKind {
    pub fn is_normal(self) -> bool {
        matches!(self, RoomKind::Normal)
    }
    pub fn is_key(self) -> bool {
        matches!(self, RoomKind::Key { .. })
    }
    pub fn is_locked(self) -> bool {
        matches!(self, RoomKind::Locked { .. })
    }
    pub fn is_switch(self) -> bool {
        matches!(self, RoomKind::Switch)
    }
    pub fn is_switch_door(self) -> bool {
        matches!(self, RoomKind::SwitchDoor)
    }
    pub fn is_special(self) -> bool {
        !self.is_normal()
    }

    pub fn key_id(self) -> Option<u32> {
        match self {
            RoomKind::Key { key_id } | RoomKind::Locked { key_id } => Some(key_id),
            _ => None,
        }
    }
}

// ── Room ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Room {
    pub id: usize,
    pub kind: RoomKind,
    pub direction: Option<Direction>,
    pub grid_pos: (i32, i32),
    pub children: Vec<usize>,
    pub depth: u32,
}

impl Room {
    pub fn new(id: usize, kind: RoomKind, direction: Option<Direction>, depth: u32) -> Self {
        Room {
            id,
            kind,
            direction,
            grid_pos: (0, 0),
            children: Vec::new(),
            depth,
        }
    }
}
