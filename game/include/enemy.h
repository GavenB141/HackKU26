#include <raylib.h>
#include "player.h"

typedef struct Enemy Enemy;
struct Enemy {
    Vector2 position;
    Enemy *next_enemy;
};
void update_enemies(Enemy *enemy, Player *player);
void draw_enemies(Enemy *enemy);
