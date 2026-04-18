#ifndef PLAYER_H
#define PLAYER_H

#include "dungeon.h"
#include <raylib.h>

typedef struct Player Player;

struct Player {
    union {
        Rectangle aabb;
        struct {
            Vector2 position;
            Vector2 size;
        };
    } body;
};

void draw_player(Player* player);
void update_player(Player* player, Dungeon* dungeon, float dt);

#endif
