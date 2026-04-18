#ifndef TILES_H
#define TILES_H

#include <raylib.h>

typedef struct Tile Tile;
typedef struct TileMap TileMap;
typedef struct TileDrawBehavior TileDrawBehavior;
typedef struct TileRenderer TileRenderer;

struct Tile {
    char type;
    char meta[7];
};

struct TileDrawBehavior {
    Texture texture;
    void (*callback)(
        Texture tex,
        Rectangle target,
        unsigned char neighbor_bits);
};

struct TileMap {
    Tile* map;
    int width, height;
};

struct TileRenderer {
    TileDrawBehavior draw_rules[128];
    int tile_width, tile_height;
};


TileMap* make_tilemap(int width, int height, const char* layout);
TileRenderer* make_tile_renderer(int tile_width, int tile_height);
void delete_tilemap(TileMap* tilemap);
void delete_tile_renderer(TileRenderer* renderer);
void draw_tilemap(const TileMap* map, const TileRenderer* rdr, Vector2 offset);
void register_tile_type(
    TileRenderer* tr,
    char symbol, 
    Texture texture,
    void (*callback)(
        Texture tex,
        Rectangle target,
        unsigned char neighbor_bits
    )
);

Tile get_tile(const TileMap* map, int x, int y);
unsigned char get_neighbor_bits(const TileMap* map, char tiletype, int x, int y);

#endif
