#include "tiles.h"
#include <assert.h>
#include <math.h>
#include <raylib.h>
#include <raymath.h>
#include <stdlib.h>
#include "dungeon.h"

struct DungeonRoom {
    TileMap* map;
    int origin_x, origin_y;
};

struct Dungeon {
    TileRenderer* renderer;
    DungeonRoom* rooms;
    int num_rooms;
    int active_room;

    // for animating
    int previous_room;
    float transition_progress;
};

static DungeonCollisionResult compute_tile_range(
    const DungeonRoom* room,
    Rectangle rect,
    int tile_width,
    int tile_height
) {
    int tx_min = floor(rect.x / tile_width);
    int tx_max = (int)ceil((rect.x + rect.width) / tile_width) - 1;
    int ty_min = floor(rect.y / tile_height);
    int ty_max = (int)ceil((rect.y + rect.height) / tile_height) - 1;

    DungeonCollisionResult range = {0};

    for (int tx = tx_min; tx <= tx_max; tx++) {
        for (int ty = ty_min; ty <= ty_max; ty++) {
            assert(range.contact_count < MAX_TILE_CONTACTS - 1);
            Tile tile = get_tile(
                room->map, tx - room->origin_x, ty - room->origin_y);
            range.contacts[range.contact_count++] = (DungeonTileContact) {
                .tx = tx,
                .ty = ty,
                .tile = tile
            };
        }
    }

    return range;
}

static bool is_blocking(char tiletype, const char* blocking) {
    if (tiletype == -1) return false;
    for(; *blocking; blocking++)
        if (*blocking == tiletype) return true;
    return false;
}

DungeonCollisionResult dungeon_translate_rect(
    const Dungeon* dungeon,
    Rectangle initial,
    Vector2 translation,
    const char* blocking_tiles
) {
    if (dungeon->num_rooms == 0) return (DungeonCollisionResult){0};

    const TileRenderer* rdr = dungeon->renderer;
    const DungeonRoom* room = &dungeon->rooms[dungeon->active_room];
    Rectangle candidate = initial;
    candidate.x += translation.x;
    DungeonCollisionResult result = compute_tile_range(
        room, candidate, rdr->tile_width, rdr->tile_height);

    for (int i = 0; i < result.contact_count; i++) {
        const DungeonTileContact* contact = &result.contacts[i];
        if (!is_blocking(contact->tile.type, blocking_tiles)) continue;
        if (translation.x > 0) {
            float resolved_x = contact->tx * rdr->tile_width - initial.width;
            if (resolved_x < candidate.x) candidate.x = resolved_x;
        } else if (translation.x < 0) {
            float resolved_x = (contact->tx + 1) * rdr->tile_width;
            if (resolved_x > candidate.x) candidate.x = resolved_x;
        }
    }

    candidate.y += translation.y;
    result = compute_tile_range(
        room, candidate, rdr->tile_width, rdr->tile_height);
    for (int i = 0; i < result.contact_count; i++) {
        const DungeonTileContact* contact = &result.contacts[i];
        if (!is_blocking(contact->tile.type, blocking_tiles)) continue;
        if (translation.y > 0) {
            float resolved_y = contact->ty * rdr->tile_height - initial.height;
            if (resolved_y < candidate.y) candidate.y = resolved_y;
        } else if (translation.y < 0) {
            float resolved_y = (contact->ty + 1) * rdr->tile_height;
            if (resolved_y > candidate.y) candidate.y = resolved_y;
        }
    }

    result = compute_tile_range(
        room, candidate, rdr->tile_width, rdr->tile_height);

    result.resolved = candidate;
    return result;
}

Dungeon* make_dungeon(TileRenderer* renderer) {
    Dungeon* dungeon = calloc(1, sizeof(Dungeon));
    dungeon->renderer = renderer;
    return dungeon;
}

void delete_dungeon(Dungeon* dungeon) {
    for (int i = 0; i < dungeon->num_rooms; i++) {
        delete_tilemap(dungeon->rooms[i].map);
    }
    free(dungeon->rooms);
    free(dungeon);
}

void add_dungeon_room(
    Dungeon* dungeon,
    int origin_x, int origin_y,
    int width, int height,
    const char* layout
) {
    if (dungeon->rooms) {
        dungeon->rooms = realloc(
            dungeon->rooms, ++dungeon->num_rooms * sizeof(DungeonRoom));
    } else {
        dungeon->num_rooms = 1;
        dungeon->rooms = malloc(sizeof(DungeonRoom));
    }

    DungeonRoom* new_room = &dungeon->rooms[dungeon->num_rooms - 1];
    new_room->origin_x = origin_x;
    new_room->origin_y = origin_y;
    new_room->map = make_tilemap(width, height, layout);
}

static Rectangle get_room_bounds(const Dungeon* dungeon, int room_id) {
    const DungeonRoom* room = &dungeon->rooms[room_id];
    const TileRenderer* renderer = dungeon->renderer;

    Rectangle bounds = {
        room->origin_x * renderer->tile_width,
        room->origin_y * renderer->tile_height
    };

    bounds.width = renderer->tile_width * room->map->width;
    bounds.height = renderer->tile_height * room->map->height;

    return bounds;
}

Rectangle dungeon_room_bounds(const Dungeon* dungeon) {
    return get_room_bounds(dungeon, dungeon->active_room);
}

void draw_dungeon(Dungeon* dungeon, float dt) {
    if(dungeon->num_rooms == 0) return;

    const DungeonRoom* active = &dungeon->rooms[dungeon->active_room];

    draw_tilemap(
        active->map,
        dungeon->renderer,
        Vector2Multiply(
            (Vector2){active->origin_x, active->origin_y},
            (Vector2){dungeon->renderer->tile_width, dungeon->renderer->tile_height}
        )
    );

    if (dungeon->transition_progress < 1) {
        const DungeonRoom* previous = &dungeon->rooms[dungeon->previous_room];
        const Rectangle active_bounds = get_room_bounds(dungeon, dungeon->active_room);
        const Rectangle previous_bounds = get_room_bounds(dungeon, dungeon->previous_room);

        draw_tilemap(
            previous->map,
            dungeon->renderer,
            Vector2Multiply(
                (Vector2){previous->origin_x, previous->origin_y},
                (Vector2){dungeon->renderer->tile_width, dungeon->renderer->tile_height}
            )
        );
        DrawRectangleRec(active_bounds, ColorAlpha(DARKGRAY, 1 - dungeon->transition_progress));
        DrawRectangleRec(previous_bounds, ColorAlpha(DARKGRAY, dungeon->transition_progress));
        dungeon->transition_progress += dt * 6;
    }
}

void dungeon_focus(Dungeon* dungeon, Vector2 position) {
    for (int i = 0; i < dungeon->num_rooms; i++) {
        Rectangle room_bounds = get_room_bounds(dungeon, i);
        if (CheckCollisionPointRec(position, room_bounds)) {
            if (dungeon->active_room != i) {
                dungeon->transition_progress = 0.0;
                dungeon->previous_room = dungeon->active_room;
                dungeon->active_room = i;
            }
            return;
        }
    }
}
