#ifndef DUNGEON_H
#define DUNGEON_H

typedef struct DungeonRoom DungeonRoom;
typedef struct Dungeon Dungeon;

#include "tiles.h"
#include "enemy.h"

struct DungeonRoom {
    TileMap* map;
    int origin_x, origin_y;
    Enemy* enemy;
};

struct Dungeon {
    TileRenderer* renderer;
    DungeonRoom* rooms;
    int num_rooms;
    int active_room;
    Vector2 spawn_point;

    // for animating
    int previous_room;
    float transition_progress;
};

// Collisions
// ----------

#define MAX_TILE_CONTACTS 9
typedef struct DungeonTileContact DungeonTileContact;
typedef struct DungeonCollisionResult DungeonCollisionResult;

struct DungeonTileContact {
    int tx, ty;
    Tile tile;
};

struct DungeonCollisionResult {
    Rectangle resolved;
    DungeonTileContact contacts[MAX_TILE_CONTACTS];
    int contact_count;
};

bool default_blocking_fn(Tile tile);

DungeonCollisionResult dungeon_translate_rect(
    const Dungeon* dungeon,
    Rectangle init,
    Vector2 translation,
    bool (*is_blocking_fn)(Tile tile)
);

Dungeon* parse_dungeon(const char* text);
void delete_dungeon(Dungeon* dungeon);
void add_dungeon_room(
    Dungeon* dungeon,
    int origin_x, int origin_y,
    int width, int height,
    const char* layout
);
void draw_dungeon(Dungeon* dungeon, float dt);
void dungeon_focus(Dungeon* dungeon, Vector2 position);
Rectangle dungeon_room_bounds(const Dungeon* dungeon);
void cast_attack(Dungeon* dungeon, Vector2 origin, Vector2 target, float radius);

#endif
