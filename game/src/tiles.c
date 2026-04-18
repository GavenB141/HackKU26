#include <assert.h>
#include <raylib.h>
#include <stdlib.h>
#include "tiles.h"

Tile get_tile(const TileMap* map, int x, int y) {
    if (x >= 0 && y >= 0 && x < map->width && y < map->height)
        return map->map[y * map->width + x];
    Tile zero = {0};
    zero.type = -1;
    return zero;
}

unsigned char get_neighbor_bits(const TileMap* map, char tiletype, int x, int y) {
    unsigned char bits = 0;

    for (int dy = -1; dy <= 1; dy++) {
        int ny = dy + y;
        for (int dx = -1; dx <= 1; dx++) {
            if (dy == 0 && dx == 0) continue;
            int nx = dx + x;
            
            bits <<= 1;
            bits |= get_tile(map, nx, ny).type == tiletype;
        }
    }

    return bits;
}

void draw_tilemap(const TileMap* map, const TileRenderer* rdr, Vector2 offset) {
    Rectangle target = {0, 0, rdr->tile_width, rdr->tile_height};

    for (int y = 0; y < map->height; y++) {
        target.y = offset.y + y * rdr->tile_height;
        for (int x = 0; x < map->width; x++) {
            target.x = offset.x + x * rdr->tile_width;

            const Tile tile = get_tile(map, x, y);
            const TileDrawBehavior* behavior = &rdr->draw_rules[tile.type];
            if (behavior->callback) {
                behavior->callback(
                    behavior->texture, target, get_neighbor_bits(map, tile.type, x, y));
            }
        }
    }
}

TileMap* make_tilemap(int width, int height, const char* layout) {
    TileMap* tm = malloc(sizeof(TileMap));
    tm->map = calloc(width * height, sizeof(Tile));
    tm->width = width;
    tm->height = height;

    for (int i = 0; i < width * height && layout[i]; i++) {
        tm->map[i].type = layout[i];
    }

    return tm;
}

TileRenderer* make_tile_renderer(int tile_width, int tile_height) {
    TileRenderer* tr = calloc(1, sizeof(TileRenderer));
    tr->tile_height = tile_height;
    tr->tile_width = tile_width;
    return tr;
}

void delete_tilemap(TileMap* tilemap) {
    free(tilemap->map);
    free(tilemap);
}

void delete_tile_renderer(TileRenderer* renderer) {
    free(renderer);
}

void register_tile_type(TileRenderer* tr, char symbol, TileDrawBehavior behavior) {
    assert(symbol > 0);
    tr->draw_rules[symbol] = behavior;
}
