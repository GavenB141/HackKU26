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

    enum Direction {
        FORWARD, RIGHT, BACKWARD, LEFT
    } facing;

    // Spritesheet
    Texture spritesheet;

    // Dash mechanics
    Vector2 dash_velocity;
    float dash_time;

    // Hammer animation times
    float hammer_charge;
    float hammer_swing;
    float hammer_impact;

    // Shockwave effect
    Vector2 shockwave_epicenter;
    float shockwave_duration;

    // Walk animation state
    float walk_cycle_time;
    Vector2 last_translation;

    int health;
    int keys;

    float invincible_time;
};

Player* make_player();
void delete_player(Player* player);

void draw_player(Player* player, float dt);
void update_player(Player* player, Dungeon* dungeon, float dt);

Vector2 get_player_center(const Player *player);

#endif
