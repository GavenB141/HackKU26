#include "tiles.h"
#include <assert.h>
#include <math.h>
#include <raylib.h>
#include <raymath.h>
#include <stdlib.h>
#include "dungeon.h"

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

void delete_dungeon(Dungeon* dungeon) {
    for (int i = 0; i < dungeon->num_rooms; i++) {
        delete_tilemap(dungeon->rooms[i].map);

        Enemy *enemy = dungeon->rooms[i].enemy;
        while (enemy)
        {
            Enemy *next_enemy = enemy->next_enemy;
            free(enemy);
            enemy = next_enemy;
        }
    }
    delete_tile_renderer(dungeon->renderer);
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
    new_room->enemy = NULL;

    for (int i = 0; *layout && i < width * height; i++, layout++)
    {
        if (*layout == '\n') {
            i--;
            continue;
        }

        if (*layout == '@') {
            dungeon->spawn_point = (Vector2){
                (origin_x + i % width) * dungeon->renderer->tile_width,
                (origin_y + (int)(i / width)) * dungeon->renderer->tile_height
            };
        }
        else if (*layout == 'e')
        {
            Enemy *new_enemy = calloc(1, sizeof(Enemy));
            // link the list
            new_enemy->next_enemy = new_room->enemy;
            new_room->enemy = new_enemy;
            // place enemy in room by pixels
            new_enemy->position = (Vector2){
                (origin_x + i % width) * dungeon->renderer->tile_width + dungeon->renderer->tile_width / 2,
                (origin_y + (int)(i / width)) * dungeon->renderer->tile_height + dungeon->renderer->tile_height / 2};
        }
    }
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

static void draw_wall_tile(
    Texture texture,
    Rectangle target,
    const TileMap* map,
    int x, int y
) {
    Vector2 selection = {-1, -1};
    unsigned char neighbor_bits = get_neighbor_bits(map, '#', x, y) | get_neighbor_bits(map, 'l', x, y) | get_neighbor_bits(map, 's', x, y);

    if (neighbor_bits == 255) {
        selection = (Vector2){1,1};
    }

    if (selection.x == -1) switch (neighbor_bits & 0b01011010) {
        case 0: selection = (Vector2){3,3}; break;
        case 2: selection = (Vector2){3,0}; break;
        case 8: selection = (Vector2){0,3}; break;
        case 16: selection = (Vector2){2,3}; break;
        case 64: selection = (Vector2){3,2}; break;
        case 66: selection = (Vector2){3,1}; break;
        case 24: selection = (Vector2){1,3}; break;
    }

    if (selection.x == -1) switch (neighbor_bits) {
        case 0b11111110: selection = (Vector2){4,0}; break;
        case 0b11111011: selection = (Vector2){5,0}; break;
        case 0b11011111: selection = (Vector2){4,1}; break;
        case 0b01111111: selection = (Vector2){5,1}; break;
        case 0b11011110: selection = (Vector2){6,0}; break;
        case 0b11111010: selection = (Vector2){6,1}; break;
        case 0b01011111: selection = (Vector2){6,2}; break;
        case 0b01111011: selection = (Vector2){6,3}; break;
        case 0b11011011: selection = (Vector2){2,6}; break;
        case 0b01111110: selection = (Vector2){3,6}; break;

        case 0b01011011: selection = (Vector2){4,2}; break;
        case 0b01011110: selection = (Vector2){5,2}; break;
        case 0b01111010: selection = (Vector2){4,3}; break;
        case 0b11011010: selection = (Vector2){5,3}; break;
        case 0b01011010: selection = (Vector2){5,5}; break;
    }

    if (selection.x == -1) switch (neighbor_bits & 0b11010110) {
        case 0b11010110: selection = (Vector2){2,1}; break;
        case 0b11010010: selection = (Vector2){1,4}; break;
        case 0b01010110: selection = (Vector2){1,5}; break;
        case 0b01010010: selection = (Vector2){6,5}; break;
    }

    if (selection.x == -1) switch (neighbor_bits & 0b00011111) {
        case 0b00011111: selection = (Vector2){1,0}; break;
        case 0b00011110: selection = (Vector2){2,4}; break;
        case 0b00011011: selection = (Vector2){3,4}; break;
        case 0b00011010: selection = (Vector2){5,4}; break;
        case 0b00010010: selection = (Vector2){6,4}; break;
    }

    if (selection.x == -1) switch (neighbor_bits & 0b01101011) {
        case 0b01101011: selection = (Vector2){0,1}; break;
        case 0b01101010: selection = (Vector2){0,4}; break;
        case 0b01001011: selection = (Vector2){0,5}; break;
        case 0b01001010: selection = (Vector2){4,5}; break;
    }

    if (selection.x == -1) switch (neighbor_bits & 0b11111000) {
        case 0b11111000: selection = (Vector2){1,2}; break;
        case 0b11011000: selection = (Vector2){2,5}; break;
        case 0b01111000: selection = (Vector2){3,5}; break;
        case 0b01011000: selection = (Vector2){5,6}; break;
        case 0b01001000: selection = (Vector2){4,6}; break;
    }

    if (selection.x == -1) switch (neighbor_bits & 0b11010000) {
        case 0b11010000: selection = (Vector2){2,2}; break;
        case 0b01010000: selection = (Vector2){6,6}; break;
    }

    if (selection.x == -1) switch (neighbor_bits & 0b00001011) {
        case 0b00001011: selection = (Vector2){0,0}; break;
        case 0b00001010: selection = (Vector2){4,4}; break;
    }

    if (selection.x == -1 && (neighbor_bits & 0b00010110) == 0b00010110)
        selection = (Vector2){2,0};
    if (selection.x == -1 && (neighbor_bits & 0b01101000) == 0b01101000)
        selection = (Vector2){0,2};

    selection = Vector2Scale(selection, 16);
    DrawTexturePro(
        texture,
        (Rectangle){selection.x, selection.y, 16, 16},
        target,
        Vector2Zero(),
        0,
        WHITE
    );
}

static void draw_floor_tile(Texture texture, Rectangle target, const TileMap* map, int x, int y) {
    Rectangle src = {16, 96, 16, 16};

    DrawTexturePro(texture, src, target, Vector2Zero(), 0, WHITE);
}

static void draw_locked_tile(Texture texture, Rectangle target, const TileMap* map, int x, int y) {
    Rectangle src = {32, 0, 16, 16};

    unsigned char wall_bits = get_neighbor_bits(map, '.', x, y) ^ get_neighbor_bits(map, 'l', x, y);
    
    if ((wall_bits & 0b01000000) == 0b01000000) {
        // floor above, bottom door
        src.x = 16;
        src.y = 16;
    }else if ((wall_bits & 0b00000010) == 0b00000010)
    {
        // floor below, top door
        src.x = 16;
        src.y = 0;
    }else if ((wall_bits & 0b00001000) == 0b00001000)
    {
        // floor right, left door
        src.x = 32;
        src.y = 0;
    }else if ((wall_bits & 0b00010000) == 0b00010000)
    {
        // floor left, right door
        src.x = 32;
        src.y = 16;
    }
    

    DrawTexturePro(texture, src, target, Vector2Zero(), 0, WHITE);
}

static void draw_chest_tile(Texture texture, Rectangle target, const TileMap* map, int x, int y) {
    Rectangle src = {64, 0, 16, 16};
    DrawTexturePro(texture, src, target, Vector2Zero(), 0, WHITE);
}

static void draw_stairs_tile(Texture texture, Rectangle target, const TileMap* map, int x, int y) {
    Rectangle src = {96, 0, 16, 16};
    DrawTexturePro(texture, src, target, Vector2Zero(), 0, WHITE);
}

static void draw_switch_door_tile(Texture texture, Rectangle target, const TileMap* map, int x, int y) {
    Rectangle src = {32, 0, 16, 16};

    unsigned char wall_bits = get_neighbor_bits(map, '.', x, y) ^ get_neighbor_bits(map, 'l', x, y);
    
    if ((wall_bits & 0b01000000) == 0b01000000) {
        // floor above, bottom door
        src.x = 32;
        src.y = 48;
    }else if ((wall_bits & 0b00000010) == 0b00000010)
    {
        // floor below, top door
        src.x = 32;
        src.y = 32;
    }else if ((wall_bits & 0b00001000) == 0b00001000)
    {
        // floor right, left door
        src.x = 48;
        src.y = 32;
    }else if ((wall_bits & 0b00010000) == 0b00010000)
    {
        // floor left, right door
        src.x = 48;
        src.y = 48;
    }
    DrawTexturePro(texture, src, target, Vector2Zero(), 0, WHITE);
}

static void draw_switch_tile(Texture texture, Rectangle target, const TileMap* map, int x, int y) {
    Rectangle src = {64, 32, 16, 16};
    DrawTexturePro(texture, src, target, Vector2Zero(), 0, WHITE);
}

Dungeon* make_empty_dungeon() {
    Dungeon* dungeon = calloc(1, sizeof(Dungeon));
    TileRenderer* renderer = make_tile_renderer(16, 16);

    Texture wall_texture = LoadTexture("assets/map_tiles.png");
    register_tile_type(renderer, '#', wall_texture, draw_wall_tile);
    register_tile_type(renderer, '.', wall_texture, draw_floor_tile);

    Texture item_texture = LoadTexture("assets/item_tiles.png");
    register_tile_type(renderer, 'l', item_texture, draw_locked_tile);
    register_tile_type(renderer, 'k', item_texture, draw_chest_tile);
    register_tile_type(renderer, 'X', item_texture, draw_stairs_tile);
    register_tile_type(renderer, 'd', item_texture, draw_switch_door_tile);
    register_tile_type(renderer, 's', item_texture, draw_switch_tile);

    dungeon->renderer = renderer;
    return dungeon;
}

Dungeon* parse_dungeon(const char* text) {
    Dungeon* dungeon = make_empty_dungeon();
    int x = 0, y = 0;
    
    enum ParseStage {
        PARSE_X, PARSE_Y, PARSE_ROOM, PARSE_META
    } stage = PARSE_Y;

    for (; *text; text++) {
        if (stage == PARSE_X) {
            if (*text == '\n') {
                stage = PARSE_ROOM;
            } else if (*text >= '0' && *text <= '9') {
                x = x * 10 + (*text - '0');
            }
        } else if (stage == PARSE_Y) {
            if (*text == '\n') {
                stage = PARSE_X;
            } else if (*text >= '0' && *text <= '9') {
                y = y * 10 + (*text - '0');
            }
        } else if (stage == PARSE_ROOM) {
            add_dungeon_room(dungeon, x * 11, y * 11, 11, 11, text);
            x = 0;
            y = 0;
            text += 11 * 11; // saves time
            stage = PARSE_META;
        } else if (stage == PARSE_META) {
            if (text[-1] == '\n' && *text == '\n') {
                stage = PARSE_Y;
            }
        }
    }
    return dungeon;
}
