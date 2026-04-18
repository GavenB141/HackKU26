use axum::{extract::Path, response::IntoResponse};
use dungeon_gen::{TileKind, layout::CELL_SIZE};
use tracing::trace;

pub async fn post_new() -> impl IntoResponse {}

pub async fn get_id(Path(id): Path<u64>) -> impl IntoResponse {
    // TODO: this should probably take a DB ID and return more than one dungeon worth of configuration
    let config = dungeon_gen::DungeonConfig::defaults_with_seed(id);

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
                    TileKind::Door => '.',
                    TileKind::LockedDoor => 'l',
                    TileKind::SwitchDoor => 'd',
                    TileKind::Pillar | TileKind::Water | TileKind::Pit => '#',
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
        s.push('\n');
    }

    s
}
