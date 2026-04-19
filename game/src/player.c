#include <math.h>
#include <raylib.h>
#include <raymath.h>
#include <stdlib.h>
#include "player.h"
#include "dungeon.h"
#include "sfx.h"

#define HAMMER_CHARGE_TIME 1.0
#define HAMMER_SWING_TIME  0.025
#define HAMMER_IMPACT_TIME 0.4
#define HAMMER_FLASH_PERIOD 0.05
#define HAMMER_SHOCKWAVE_RADIUS 18.0
#define HAMMER_SHOCKWAVE_DURATION 0.15
#define PLAYER_WALK_PERIOD 0.1

Player* make_player() {
    Player* player = calloc(1, sizeof(Player));
    player->body.size = (Vector2){10, 10};
    player->health = 3;
    player->spritesheet = LoadTexture("assets/hammer_rat.png");
    return player;
}

void delete_player(Player *player) {
    UnloadTexture(player->spritesheet);
    free(player);
}

Vector2 get_player_center(const Player* player) {
    return Vector2Add(player->body.position, Vector2Scale(player->body.size, 0.5));
}

void draw_player(Player* player, float dt) {
    Rectangle src = {0, 0, 48, 48};
    
    // Select src frame based on player state
    if (player->hammer_impact > 0) {
        src.x = 17;
        src.x += 6 * player->facing;
    } else if (player->hammer_swing > 0) {
        src.x = 16;
        src.x += 6 * player->facing;
    } else if (player->hammer_charge > 0) {
        if (player->hammer_charge < HAMMER_CHARGE_TIME / 3) {
            src.x = 12;
        } else if (player->hammer_charge < 2 * HAMMER_CHARGE_TIME / 3) {
            src.x = 13;
        } else if (player->hammer_charge < HAMMER_CHARGE_TIME) {
            src.x = 14;
        } else {
            float flash = fmodf(player->hammer_charge, HAMMER_FLASH_PERIOD);
            if (flash >= HAMMER_FLASH_PERIOD / 2) {
                src.x = 15;
            } else {
                src.x = 14;
            }
        }
        src.x += 6 * player->facing;
    } else if (!(Vector2Equals(player->last_translation, Vector2Zero()))) {
        player->walk_cycle_time = fmodf(
            player->walk_cycle_time + dt, PLAYER_WALK_PERIOD);

        src.x = player->walk_cycle_time > PLAYER_WALK_PERIOD / 2 ? 5 : 4;
        src.x += 2 * player->facing;
    } else {
        src.x = player->facing;
    }

    src.x *= 48;

    Rectangle target = player->body.aabb;
    target.width = 48;
    target.height = 48;

    if (player->shockwave_duration > 0) {
        player->shockwave_duration -= dt;
        float radius = HAMMER_SHOCKWAVE_RADIUS *
            ((HAMMER_SHOCKWAVE_DURATION - player->shockwave_duration) / HAMMER_SHOCKWAVE_DURATION);
        float alpha = player->shockwave_duration / HAMMER_SHOCKWAVE_DURATION;
        DrawCircleV(
            player->shockwave_epicenter,
            radius, ColorAlpha(DARKGRAY, alpha)
        );
    }

    DrawTexturePro(
        player->spritesheet,
        src,
        target,
        (Vector2){19, 21},
        0, WHITE
    );
}

static void resolve_player_direction(Player* player, Vector2 move_dir) {
    if (player->hammer_charge != 0) return;

    if (move_dir.x < 0 && player->facing == LEFT) return;
    if (move_dir.x > 0 && player->facing == RIGHT) return;
    if (move_dir.y < 0 && player->facing == BACKWARD) return;
    if (move_dir.y > 0 && player->facing == FORWARD) return;

    if (move_dir.y < 0) player->facing = BACKWARD;
    else if (move_dir.y > 0) player->facing = FORWARD;
    else if (move_dir.x < 0) player->facing = LEFT;
    else if (move_dir.x > 0) player->facing = RIGHT;
}

static void move_player(Player* player, Dungeon* dungeon, float dt, float speed) {
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
        if (IsKeyDown(KEY_A) || IsKeyDown(KEY_LEFT)) vel.x -= 1;
        if (IsKeyDown(KEY_S) || IsKeyDown(KEY_DOWN)) vel.y += 1;
        if (IsKeyDown(KEY_D) || IsKeyDown(KEY_RIGHT)) vel.x += 1;

        vel = Vector2Scale(
            Vector2Normalize(vel),
            speed
        );

        if (player->dash_time < 0) {
            player->dash_time = Clamp(player->dash_time + dt, -dash_cooldown, 0);
        } else if (player->hammer_charge == 0 && IsKeyDown(KEY_LEFT_SHIFT)) {
            player->dash_velocity = Vector2Scale(vel, 3);
            player->dash_time = 0.15;
        }
    }

    vel = Vector2Scale(vel, dt);
    resolve_player_direction(player, vel);

    // Probe just ahead of the player for locked doors and use a key if available
    if (player->keys > 0 && !Vector2Equals(vel, Vector2Zero())) {
        float tw = dungeon->renderer->tile_width;
        float th = dungeon->renderer->tile_height;
        DungeonRoom* room = &dungeon->rooms[dungeon->active_room];
        Rectangle aabb = player->body.aabb;
        const float probe = 2.0f;
        Vector2 points[4];
        int npoints = 0;
        if (vel.x > 0) {
            points[npoints++] = (Vector2){aabb.x + aabb.width + probe, aabb.y + 1};
            points[npoints++] = (Vector2){aabb.x + aabb.width + probe, aabb.y + aabb.height - 2};
        } else if (vel.x < 0) {
            points[npoints++] = (Vector2){aabb.x - probe, aabb.y + 1};
            points[npoints++] = (Vector2){aabb.x - probe, aabb.y + aabb.height - 2};
        }
        if (vel.y > 0) {
            points[npoints++] = (Vector2){aabb.x + 1, aabb.y + aabb.height + probe};
            points[npoints++] = (Vector2){aabb.x + aabb.width - 2, aabb.y + aabb.height + probe};
        } else if (vel.y < 0) {
            points[npoints++] = (Vector2){aabb.x + 1, aabb.y - probe};
            points[npoints++] = (Vector2){aabb.x + aabb.width - 2, aabb.y - probe};
        }
        for (int i = 0; i < npoints && player->keys > 0; i++) {
            int gx = (int)floorf(points[i].x / tw);
            int gy = (int)floorf(points[i].y / th);
            if (dungeon_unlock_door(dungeon, gx, gy))
                player->keys--;
        }
    }

    DungeonCollisionResult result = dungeon_translate_rect(
        dungeon, player->body.aabb, vel, 0);

    // Pick up keys from opened chests (k tiles with meta[0] set)
    DungeonRoom* active_room = &dungeon->rooms[dungeon->active_room];
    for (int i = 0; i < result.contact_count; i++) {
        DungeonTileContact* c = &result.contacts[i];
        if (c->tile.type == 'k' && c->tile.meta[0] && !c->tile.meta[1]) {
            int lx = c->tx - active_room->origin_x;
            int ly = c->ty - active_room->origin_y;
            active_room->map->map[ly * active_room->map->width + lx].meta[1] = 1;
            player->keys++;
        }
    }

    player->last_translation = Vector2Subtract(*(Vector2*)&result.resolved, player->body.position);
    player->body.aabb = result.resolved;
    dungeon_focus(
        dungeon, 
        Vector2Add(player->body.position, Vector2Scale(player->body.size, 0.5))
    );
}

static void player_hammer(Player* player, Dungeon* dungeon, float dt) {
    if (player->hammer_swing == 0) {
        if (IsKeyDown(KEY_SPACE)) {
            if (player->hammer_charge < HAMMER_CHARGE_TIME && player->hammer_charge + dt >= HAMMER_CHARGE_TIME)
            {
                play_sfx(SFX_HAMMER_READY);
            }
            player->hammer_charge += dt;
        } else if (IsKeyReleased(KEY_SPACE)) {
            if (player->hammer_charge >= HAMMER_CHARGE_TIME) {
                player->hammer_swing = HAMMER_SWING_TIME;
            } else {
                player->hammer_charge = 0;
            }
        }
    } else if (player->hammer_impact == 0) {
        player->hammer_swing -= dt;
        if (player->hammer_swing <= 0) {
            player->hammer_impact = HAMMER_IMPACT_TIME + player->hammer_swing;
            Vector2 offset;
            switch (player->facing) {
                case FORWARD:  offset = (Vector2){0, 10}; break;
                case RIGHT:    offset = (Vector2){13.5, 6}; break;
                case BACKWARD: offset = (Vector2){0,-10}; break;
                case LEFT:     offset = (Vector2){-13.5,6}; break;
            }
            Vector2 center = get_player_center(player);
            player->shockwave_epicenter = Vector2Add(center, offset);
            player->shockwave_duration = HAMMER_SHOCKWAVE_DURATION;
            if (!cast_attack(dungeon, center, player->shockwave_epicenter, HAMMER_SHOCKWAVE_RADIUS))
                play_sfx(SFX_HAMMER_HIT);
        }
    } else {
        player->hammer_impact -= dt;
        if (player->hammer_impact <= 0) {
            player->hammer_charge = 0;
            player->hammer_swing = 0;
            player->hammer_impact = 0;
        }
    }
}

void update_player(Player* player, Dungeon* dungeon, float dt) {
    if (player->dash_time <= 0)
        player_hammer(player, dungeon, dt);

    const float speed =
        player->hammer_charge == 0 ? 80 :
        player->hammer_swing == 0 && player->hammer_impact == 0 ? 10 : 0;

    move_player(player, dungeon, dt, speed);
}
