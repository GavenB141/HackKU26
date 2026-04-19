#ifndef ENEMY_H
#define ENEMY_H

#include <raylib.h>

typedef struct Enemy Enemy;

#include "player.h"

struct Enemy {
    Vector2 position;
    Enemy *next_enemy;
};
void update_enemies(Enemy *enemy, Player *player);
void draw_enemies(Enemy *enemy);
#endif