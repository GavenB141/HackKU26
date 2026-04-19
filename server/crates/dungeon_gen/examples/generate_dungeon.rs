//! Generates a dungeon and prints both the room-level map and the full
//! tile-level layout.

use dungeon_gen::evolution::{generate, DungeonConfig};

fn main() {
    let seed = std::env::var("RNG_SEED")
        .map_or(Ok(42), |seed| seed.parse::<u64>())
        .expect("invalid seed (must be an int)");

    let config = DungeonConfig::new(18, 4, 4, 1.89, seed);
    println!("Generating dungeon (seed={})…", config.seed);
    let d = generate(&config);

    println!("\n=== Room map ===");
    println!("S=spawn X=exit B=boss E=enemy K=key L=lock W=switch D=sw-door $=sign .=normal");
    d.print_map();
    println!("Best found at generation {}", d.best_generation);

    println!(
        "\n=== Tile map ({} × {}) ===",
        d.tile_map.width(),
        d.tile_map.height()
    );
    println!("#=wall .=floor +=door L=locked-door W=sw-door O=pillar ~=water V=pit");
    println!("C=chest S=switch ?=sign X=stairs @=spawn E=enemy B=boss");
    println!("{}", d.tile_map.ascii());

    println!("\n=== Signs ===");
    for (x, y, sign) in d.signs() {
        println!("  ({:+},{:+})  [{:?}]  \"{}\"", x, y, sign.kind, sign.text);
    }

    println!("\n=== Enemies ===");
    for (x, y, eg) in d.enemies() {
        if eg.is_boss {
            println!("  ({:+},{:+})  BOSS", x, y);
        } else {
            println!("  ({:+},{:+})  {} regular", x, y, eg.count);
        }
    }

    let ep = d.grid.pos_of(d.exit_room_id).unwrap();
    println!(
        "\n=== Exit === room {} @ grid ({:+},{:+})",
        d.exit_room_id, ep.0, ep.1
    );
}
