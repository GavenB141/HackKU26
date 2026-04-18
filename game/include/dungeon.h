#ifndef DUNGEON_H
#define DUNGEON_H

#include "tiles.h"

typedef struct DungeonRoom DungeonRoom;
typedef struct Dungeon Dungeon;

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

DungeonCollisionResult dungeon_translate_rect(
    const Dungeon* dungeon,
    Rectangle init,
    Vector2 translation,
    const char* blocking_tiles
);

Dungeon* make_dungeon();
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

#endif
