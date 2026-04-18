#include <raylib.h>
#include <raymath.h>
#include "player.h"
#include "dungeon.h"

void draw_player(Player* player) {
    DrawRectangleRec(player->body.aabb, GREEN);
}

static void move_player(Player* player, Dungeon* dungeon, float dt) {
    const int speed = 50;
    Vector2 vel = Vector2Zero();

    if (IsKeyDown(KEY_W)) vel.y -= 1;
    if (IsKeyDown(KEY_A)) vel.x -= 1;
    if (IsKeyDown(KEY_S)) vel.y += 1;
    if (IsKeyDown(KEY_D)) vel.x += 1;

    vel = Vector2Scale(
        Vector2Normalize(vel),
        speed * dt
    );

    DungeonCollisionResult result = dungeon_translate_rect(
        dungeon, player->body.aabb, vel, "#");

    player->body.aabb = result.resolved;
    dungeon_focus(
        dungeon, 
        Vector2Add(player->body.position, Vector2Scale(player->body.size, 0.5))
    );
}

void update_player(Player* player, Dungeon* dungeon, float dt) {
    move_player(player, dungeon, dt);
}
