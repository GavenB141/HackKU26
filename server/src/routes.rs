use axum::{extract::Path, response::IntoResponse};
use dungeon_gen::{RoomContent, RoomKind, TileKind, layout::CELL_SIZE};
use tracing::trace;

pub async fn post_new() -> impl IntoResponse {}

pub async fn generate_dungeon_from_seed(Path(seed): Path<u64>) -> impl IntoResponse {
    let config = dungeon_gen::DungeonConfig::defaults_with_seed(seed);

    let dungeon = tokio::task::spawn_blocking(move || dungeon_gen::generate(&config))
        .await
        .expect("failed to generate dungeon configuration");

    trace!(?dungeon, "generated dungeon");

    let mut s = String::new();
    for room in dungeon.tile_map.rooms {
        let (tile_col, tile_row) = room.world_origin;
        let tile_col = (tile_col - (dungeon.grid.min_x * CELL_SIZE as i32)) as usize / CELL_SIZE;
        let tile_row = (tile_row - (dungeon.grid.min_y * CELL_SIZE as i32)) as usize / CELL_SIZE;

        s.push_str(&format!("{}\n", tile_col));
        s.push_str(&format!("{}\n", tile_row));
        for col in 0..CELL_SIZE {
            for row in 0..CELL_SIZE {
                s.push(match room.get(col, row) {
                    TileKind::Wall => '#',
                    TileKind::Floor => '.',
                    TileKind::Door => 'D',
                    TileKind::LockedDoor => 'l',
                    TileKind::SwitchDoor => 'd',
                    TileKind::Pillar | TileKind::Water  => '#',
                    TileKind::Pit => 'p',
                    TileKind::Chest => 'k',
                    TileKind::SwitchTile => 's',
                    TileKind::Sign => '?',
                    TileKind::Stairs => 'X',
                    TileKind::Spawn => '@',
                    TileKind::EnemySpawn => 'e',
                    TileKind::BossSpawn => 'e',
                });
            }
            s.push('\n');
        }
        match &dungeon.tree.rooms[room.room_id].kind {
            RoomKind::Normal => {}
            RoomKind::Key { key_id } => {
                s.push_str(&format!("key : {key_id}\n"));
            }
            RoomKind::Locked { key_id } => {
                s.push_str(&format!("locked : {key_id}\n"));
            }
            RoomKind::Switch | RoomKind::SwitchDoor => {
                /* do nothing; this information is handled by room content */
            }
        }
        let RoomContent {
            enemies,
            switch,
            switch_door,
            sign,
            is_exit: _, // exit state is revealed by having an exit
        } = &dungeon.content[room.room_id];
        if let Some(enemies) = enemies {
            if enemies.is_boss {
                s.push_str("boss\n");
            }
            s.push_str(&format!("enemies {}\n", enemies.count));
        }
        if let Some(switch) = switch {
            s.push_str(&format!("switch {} :", switch.switch_id));
            for signal in &switch.signals {
                s.push_str(&format!(" {}", signal));
            }
            s.push('\n');
        }
        if let Some(switch_door) = switch_door {
            s.push_str(&format!("door {} :", switch_door.door_id));
            for signal in &switch_door.signals {
                s.push_str(&format!(" {}", signal));
            }
            s.push('\n');
        }
        if let Some(sign) = sign {
            s.push_str(&format!("sign {}\n", sign.text));
        }
        s.push('\n');
    }

    s
}
