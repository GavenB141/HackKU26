#include "enemy.h"

void update_enemies(Enemy *enemy, Player *player) {
    if (!enemy) return;
    
    update_enemies(enemy->next_enemy, player);
}

void draw_enemies(Enemy *enemy) {
    if (!enemy) return;

    DrawCircleV(enemy->position, 7, RED);
    
    draw_enemies(enemy->next_enemy);
}