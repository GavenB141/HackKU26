#ifndef PLAYER_H
#define PLAYER_H

#include <raylib.h>

typedef struct Player Player;

#include "dungeon.h"

struct Player {
    union {
        Rectangle aabb;
        struct {
            Vector2 position;
            Vector2 size;
        };
    } body;

    // Spritesheet
    Texture spritesheet;

    // Dash mechanics
    Vector2 dash_velocity;
    float dash_time;

    int health;
};

Player* make_player();
void delete_player();

void draw_player(Player* player);
void update_player(Player* player, Dungeon* dungeon, float dt);

#endif
