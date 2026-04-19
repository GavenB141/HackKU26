#ifndef ENEMY_H
#define ENEMY_H

#include <raylib.h>

typedef struct Enemy Enemy;

#include "player.h"

typedef enum EnemyState
{
    ENEMY_WANDER,
    ENEMY_CHARGE_PREPARE,
    ENEMY_CHARGING,
    ENEMY_POST_CHARGE,
} EnemyState;

struct Enemy
{
    Enemy *next_enemy;
    Vector2 position;
    Vector2 charge_dir;
    float state_time_left;
    EnemyState current_state;
};
void update_enemies(Enemy *enemy, const Dungeon *dungeon, Player *player, float dt);
void draw_enemies(Enemy *enemy);
#endif