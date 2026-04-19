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
    ENEMY_STUNNED,
    ENEMY_DEAD,
} EnemyState;

struct Enemy
{
    Enemy *next_enemy;
    Vector2 position;
    union
    {
        Vector2 charge_dir;
        Vector2 stunned_sent_to;
    };
    float state_time_left;
    EnemyState current_state;
    int health;
};
void update_enemies(Enemy *enemy, const Dungeon *dungeon, Player *player, float dt);
void draw_enemies(Enemy *enemy);
bool try_attack_enemy(Enemy *enemy, Vector2 from_point, Vector2 target_point, float radius);
#endif