#include "tiles.h"
#include <assert.h>
#include <math.h>
#include <raylib.h>
#include <raymath.h>
#include <stdlib.h>
#include <string.h>
#include "dungeon.h"
#include "sfx.h"

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

bool default_blocking_fn(Tile tile) {
    switch (tile.type) {
        case '#': case 'd': case 'l': case 's':
        case 'p':
        return true;
        case 'k': return !tile.meta[0];
        default:
        return false;
    }
}

DungeonCollisionResult dungeon_translate_rect(
    const Dungeon* dungeon,
    Rectangle initial,
    Vector2 translation,
    bool (*is_blocking_fn)(Tile tile)
) {
    if (dungeon->num_rooms == 0) return (DungeonCollisionResult){0};
    if (is_blocking_fn == NULL) is_blocking_fn = default_blocking_fn;

    const TileRenderer* rdr = dungeon->renderer;
    const DungeonRoom* room = &dungeon->rooms[dungeon->active_room];
    Rectangle candidate = initial;
    candidate.x += translation.x;
    DungeonCollisionResult result = compute_tile_range(
        room, candidate, rdr->tile_width, rdr->tile_height);

    for (int i = 0; i < result.contact_count; i++) {
        const DungeonTileContact* contact = &result.contacts[i];
        if (!is_blocking_fn(contact->tile)) continue;
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
        if (!is_blocking_fn(contact->tile)) continue;
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
        } else if (*layout == 'e') {
            Enemy *new_enemy = calloc(1, sizeof(Enemy));
            // link the list
            new_enemy->next_enemy = new_room->enemy;
            new_room->enemy = new_enemy;
            // place enemy in room by pixels
            new_enemy->position = (Vector2){
                (origin_x + i % width) * dungeon->renderer->tile_width + dungeon->renderer->tile_width / 2.0,
                (origin_y + (int)(i / width)) * dungeon->renderer->tile_height + dungeon->renderer->tile_height / 2.0};
            new_enemy->health = 2;
        } else if (*layout == '?') {
            new_room->map->map[i].type = '.';
        } else if (*layout == 'p') {
            new_room->map->map[i].type = '#';
        } else if (*layout == 'd') {
            new_room->map->map[i].type = 'D';
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

/*
 * Neighbor bit layout (from get_neighbor_bits):
 *   bit 0 = NW, bit 1 = N, bit 2 = NE
 *   bit 3 = W,             bit 4 = E
 *   bit 5 = SW, bit 6 = S, bit 7 = SE
 *
 * A diagonal neighbor only affects tile selection when BOTH of its adjacent
 * cardinals are also walls (i.e. it fills an inner corner). In all other cases
 * the diagonal bit is irrelevant and must be masked out before lookup, which
 * is what the old code failed to do for the N+E and W+S cardinal pairs.
 *
 * We re-encode as 8 "effective" bits with the same cardinal positions but
 * diagonal bits suppressed unless their two cardinals are both set:
 *   bit 0 = N
 *   bit 1 = W
 *   bit 2 = E
 *   bit 3 = S
 *   bit 4 = NW_fill  (set only when N && W && NW)
 *   bit 5 = NE_fill  (set only when N && E && NE)
 *   bit 6 = SW_fill  (set only when S && W && SW)
 *   bit 7 = SE_fill  (set only when S && E && SE)
 *
 * This maps all 256 raw inputs onto exactly 47 distinct keys, each of which
 * corresponds to one tile in the 7x7 tile sheet (coordinates in tile units).
 */
/* Tile sheet coordinates (column, row) indexed by effective neighbor bits. */
static const Vector2 WALL_TILE_TABLE[256] = {
    [0] = {3, 3},   /* isolated                          */
    [1] = {3, 0},   /* N only                            */
    [2] = {0, 3},   /* W only                            */
    [3] = {4, 4},   /* N+W, no corner fill               */
    [4] = {2, 3},   /* E only                            */
    [5] = {6, 4},   /* N+E, no corner fill               */
    [6] = {1, 3},   /* W+E (horizontal corridor)         */
    [7] = {5, 4},   /* N+W+E, no NW/NE fill              */
    [8] = {3, 2},   /* S only                            */
    [9] = {3, 1},   /* N+S (vertical corridor)           */
    [10] = {4, 6},  /* W+S, no corner fill               */
    [11] = {4, 5},  /* N+W+S, no NW/SW fill              */
    [12] = {6, 6},  /* E+S, no corner fill               */
    [13] = {6, 5},  /* N+E+S, no NE/SE fill              */
    [14] = {5, 6},  /* W+E+S, no SW/SE fill              */
    [15] = {5, 5},  /* N+W+E+S, no fills (all cardinals) */
    [19] = {0, 0},  /* N+W, NW filled                    */
    [23] = {3, 4},  /* N+W+E, NW filled                  */
    [27] = {0, 5},  /* N+W+S, NW filled                  */
    [31] = {4, 2},  /* N+W+E+S, NW filled                */
    [37] = {2, 0},  /* N+E, NE filled                    */
    [39] = {2, 4},  /* N+W+E, NE filled                  */
    [45] = {1, 5},  /* N+E+S, NE filled                  */
    [47] = {5, 2},  /* N+W+E+S, NE filled                */
    [55] = {1, 0},  /* N+W+E, NW+NE filled               */
    [63] = {6, 2},  /* N+W+E+S, NW+NE filled             */
    [74] = {0, 2},  /* W+S, SW filled                    */
    [75] = {0, 4},  /* N+W+S, SW filled                  */
    [78] = {3, 5},  /* W+E+S, SW filled                  */
    [79] = {4, 3},  /* N+W+E+S, SW filled                */
    [91] = {0, 1},  /* N+W+S, NW+SW filled               */
    [95] = {6, 3},  /* N+W+E+S, NW+SW filled             */
    [111] = {3, 6}, /* N+W+E+S, NE+SW filled             */
    [127] = {5, 1}, /* N+W+E+S, NW+NE+SW filled          */
    [140] = {2, 2}, /* E+S, SE filled                    */
    [141] = {1, 4}, /* N+E+S, SE filled                  */
    [142] = {2, 5}, /* W+E+S, SE filled                  */
    [143] = {5, 3}, /* N+W+E+S, SE filled                */
    [159] = {2, 6}, /* N+W+E+S, NE+SE filled             */
    [173] = {2, 1}, /* N+E+S, NE+SE filled               */
    [175] = {6, 0}, /* N+W+E+S, NW+NE+SE filled          */
    [191] = {4, 1}, /* N+W+E+S, NW+NE+SW+SE filled       */
    [206] = {1, 2}, /* W+E+S, SW+SE filled               */
    [207] = {6, 1}, /* N+W+E+S, SW+SE filled             */
    [223] = {5, 0}, /* N+W+E+S, NW+SW+SE filled          */
    [239] = {4, 0}, /* N+W+E+S, NE+SW+SE filled          */
    [255] = {1, 1}, /* all neighbors (fully surrounded)  */
};

/*
 * Compute the lookup key from raw neighbor bits.
 *
 * Diagonal bits are suppressed unless both of their adjacent cardinals
 * are set, which is the only situation where they influence tile choice.
 */
static int wall_tile_key(unsigned char nb)
{
    int N = (nb >> 1) & 1;
    int NE = (nb >> 2) & 1;
    int W = (nb >> 3) & 1;
    int E = (nb >> 4) & 1;
    int SW = (nb >> 5) & 1;
    int S = (nb >> 6) & 1;
    int SE = (nb >> 7) & 1;
    int NW = (nb >> 0) & 1;

    int nw_fill = NW & N & W;
    int ne_fill = NE & N & E;
    int sw_fill = SW & S & W;
    int se_fill = SE & S & E;

    return N | (W << 1) | (E << 2) | (S << 3) | (nw_fill << 4) | (ne_fill << 5) | (sw_fill << 6) | (se_fill << 7);
}

static void draw_wall_tile(
    Texture texture,
    Rectangle target,
    const TileMap *map,
    int x, int y)
{
    unsigned char nb = get_neighbor_bits(map, '#', x, y) | get_neighbor_bits(map, 'l', x, y) | get_neighbor_bits(map, 'd', x, y) | get_neighbor_bits(map, 'D', x, y);

    Vector2 tile = WALL_TILE_TABLE[wall_tile_key(nb)];
    DrawTexturePro(
        texture,
        (Rectangle){tile.x * 16, tile.y * 16, 16, 16},
        target,
        Vector2Zero(),
        0,
        WHITE);
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
    Tile tile = get_tile(map, x, y);
    if (tile.meta[1]) {
        src.y += 16;
    }else if (tile.meta[0]) {
        src.x -= 16;
    }
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

static void draw_door_tile(Texture texture, Rectangle target, const TileMap* map, int x, int y) {
    Rectangle src = {32, 0, 16, 16};

    unsigned char wall_bits = get_neighbor_bits(map, '.', x, y) ^ get_neighbor_bits(map, 'l', x, y);
    
    if ((wall_bits & 0b01000000) == 0b01000000) {
        // floor above, bottom door
        src.x = 0;
        src.y = 48;
    }else if ((wall_bits & 0b00000010) == 0b00000010)
    {
        // floor below, top door
        src.x = 0;
        src.y = 32;
    }else if ((wall_bits & 0b00001000) == 0b00001000)
    {
        // floor right, left door
        src.x = 16;
        src.y = 32;
    }else if ((wall_bits & 0b00010000) == 0b00010000)
    {
        // floor left, right door
        src.x = 16;
        src.y = 48;
    }
    DrawTexturePro(texture, src, target, Vector2Zero(), 0, WHITE);
}

static void draw_switch_tile(Texture texture, Rectangle target, const TileMap* map, int x, int y) {
    Rectangle src = {64, 48, 16, 16};
    Tile tile = get_tile(map, x, y);
    if (tile.meta[0]) src.y -= 16;
    DrawTexturePro(texture, src, target, Vector2Zero(), 0, WHITE);
}

static Dungeon* make_empty_dungeon() {
    Dungeon* dungeon = calloc(1, sizeof(Dungeon));
    TileRenderer* renderer = make_tile_renderer(16, 16);

    Texture wall_texture = LoadTexture("assets/map_tiles.png");
    register_tile_type(renderer, '#', wall_texture, draw_wall_tile);
    register_tile_type(renderer, '.', wall_texture, draw_floor_tile);

    Texture item_texture = LoadTexture("assets/item_tiles.png");
    register_tile_type(renderer, 'k', item_texture, draw_chest_tile);
    register_tile_type(renderer, 'X', item_texture, draw_stairs_tile);
    register_tile_type(renderer, 'D', item_texture, draw_door_tile);
    register_tile_type(renderer, 'l', item_texture, draw_locked_tile);
    register_tile_type(renderer, 'd', item_texture, draw_switch_door_tile);
    register_tile_type(renderer, 's', item_texture, draw_switch_tile);

    dungeon->renderer = renderer;
    return dungeon;
}

Dungeon* parse_dungeon(const char* text) {
    Dungeon* dungeon = make_empty_dungeon();
    DungeonRoom* room = 0;
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
            room = &dungeon->rooms[dungeon->num_rooms - 1];
            x = 0;
            y = 0;
            text += 11 * 11; // saves time
            stage = PARSE_META;
        } else if (stage == PARSE_META) {
            if (text[-1] == '\n' && text[0] == '\n') {
                stage = PARSE_Y;
            }
        }
    }
    return dungeon;
}

static Tile* get_tile_global(Dungeon* dungeon, int global_tx, int global_ty) {
    for (int i = 0; i < dungeon->num_rooms; i++) {
        DungeonRoom* room = &dungeon->rooms[i];
        int lx = global_tx - room->origin_x;
        int ly = global_ty - room->origin_y;
        if (lx >= 0 && ly >= 0 && lx < room->map->width && ly < room->map->height)
            return &room->map->map[ly * room->map->width + lx];
    }
    return NULL;
}

bool dungeon_unlock_door(Dungeon* dungeon, int global_tx, int global_ty) {
    Tile* tile = get_tile_global(dungeon, global_tx, global_ty);
    if (!tile || tile->type != 'l') return false;
    tile->type = 'D';
    int dx[] = {0, 0, -1, 1};
    int dy[] = {-1, 1, 0, 0};
    for (int i = 0; i < 4; i++) {
        Tile* adj = get_tile_global(dungeon, global_tx + dx[i], global_ty + dy[i]);
        if (adj && adj->type == 'l') adj->type = 'D';
    }
    play_sfx(SFX_DOOR_UNLOCK);
    return true;
}

static bool attack_tile(Dungeon *dungeon, int x, int y) {
    DungeonRoom* room = &dungeon->rooms[dungeon->active_room];
    int index = y * room->map->width + x;
    if (index >= room->map->width * room->map->height)
        return false;

    Tile* tile = &room->map->map[index];

    if (tile->type == 's') {
        tile->meta[0] = !tile->meta[0];

        if (tile->meta[0])
            play_sfx(SFX_SWITCH_PRESSED);
        else
            play_sfx(SFX_SWITCH_DEPRESSED);
        return true;
    }
    if (tile->type == 'k')
    {
        tile->meta[0] = 1;
        play_sfx(SFX_CHEST_BREAK);
        return true;
    }
    return false;
}

bool cast_attack(Dungeon *dungeon, Vector2 origin, Vector2 target, float radius)
{
    DungeonRoom* room = &dungeon->rooms[dungeon->active_room];

    bool hit_anything = false;
    // check the tiles
    for (int y = 0; y < room->map->height; y++) {
        for (int x = 0; x < room->map->width; x++) {
            Rectangle tile_frame = {
                (room->origin_x + x) * dungeon->renderer->tile_width,
                (room->origin_y + y) * dungeon->renderer->tile_height,
                dungeon->renderer->tile_width, dungeon->renderer->tile_height
            };

            if (CheckCollisionCircleRec(target, radius, tile_frame)) {
                hit_anything |= attack_tile(dungeon, x, y);
            }
        }
    }
    // check the enemies
    for (Enemy *enemy = room->enemy; enemy != NULL; enemy = enemy->next_enemy)
    {
        hit_anything |= try_attack_enemy(enemy, origin, target, radius);
    }
    return hit_anything;
}
