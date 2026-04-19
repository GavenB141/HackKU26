#include <raylib.h>
#include <raymath.h>
#include "player.h"
#include "dungeon.h"

void draw_player(Player* player) {
    DrawRectangleRec(player->body.aabb, GREEN);
}

static void move_player(Player* player, Dungeon* dungeon, float dt) {
    const int speed = 80;
    const float dash_cooldown = 0.35;
    Vector2 vel = Vector2Zero();

    if (player->dash_time > 0) {
        vel = player->dash_velocity;
        player->dash_time -= dt;
        if (player->dash_time <= 0) {
            player->dash_time = -dash_cooldown;
        }
    } else {
        if (IsKeyDown(KEY_W) || IsKeyDown(KEY_UP)) vel.y -= 1;
        if (IsKeyDown(KEY_A) || IsKeyDown(KEY_RIGHT)) vel.x -= 1;
        if (IsKeyDown(KEY_S) || IsKeyDown(KEY_DOWN)) vel.y += 1;
        if (IsKeyDown(KEY_D) || IsKeyDown(KEY_LEFT)) vel.x += 1;

        vel = Vector2Scale(
            Vector2Normalize(vel),
            speed
        );

        if (player->dash_time < 0) {
            player->dash_time = Clamp(player->dash_time + dt, -dash_cooldown, 0);
        } else if (IsKeyDown(KEY_LEFT_SHIFT)) {
            player->dash_velocity = Vector2Scale(vel, 4);
            player->dash_time = 0.15;
        }
    }

    vel = Vector2Scale(vel, dt);

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
