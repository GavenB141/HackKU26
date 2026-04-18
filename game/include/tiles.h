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

void draw_tilemap(const TileMap* map, const TileRenderer* rdr, Vector2 offset);
TileMap* make_tilemap(int width, int height, const char* layout);
TileRenderer* make_tile_renderer(int tile_width, int tile_height);
void delete_tilemap(TileMap* tilemap);
void delete_tile_renderer(TileRenderer* renderer);
void register_tile_type(TileRenderer* tr, char symbol, TileDrawBehavior behavior);

#endif
