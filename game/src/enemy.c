#include "enemy.h"
#include "sfx.h"
#include "player.h"

#include <stddef.h>
#include <raymath.h>

void update_enemies(Enemy *enemy, const Dungeon *dungeon, Player *player, float dt)
{
    if (!enemy) return;

    const float wander_speed = 20;
    const float wander_chase_weight = 5;
    const float swarm_avoidance_weight = 20;
    const float swarm_avoidance_radius = 16 * 3;

    const float charge_speed = 120;
    const float minimum_charge_radius = 16 * 2;
    const float change_chance_per_second = 0.10f;
    const float charge_prep_time = 0.4f;
    const float charge_time = 0.3f;
    const float charge_post_time = 0.5f;

    switch (enemy->current_state)
    {
    case ENEMY_WANDER:
        // normal "player seeking" behavior
        const Enemy *first_enemy = dungeon->rooms[dungeon->active_room].enemy;
        // chase player
        Vector2 move_direction =
            Vector2Scale(
                Vector2Subtract(player->body.position, enemy->position),
                wander_chase_weight);
        // don't "cluster" or pile up
        for (const Enemy *cursor = first_enemy; cursor != NULL; cursor = cursor->next_enemy)
        {
            if (enemy == cursor)
                continue;
            Vector2 distance_vector = Vector2Subtract(enemy->position, cursor->position);
            // clamp this; it shouldn't go negative within the linear falloff
            distance_vector = Vector2ClampValue(distance_vector, 0, swarm_avoidance_radius);
            float distance = Vector2Length(distance_vector);
            float scale_factor = 1.0f - (distance / swarm_avoidance_radius);
            Vector2 scaled_distance = Vector2Scale(distance_vector, scale_factor);

            move_direction = Vector2Add(
                move_direction,
                Vector2Scale(scaled_distance, swarm_avoidance_weight));
        }

        // apply the movement
        move_direction = Vector2Normalize(move_direction);
        enemy->position = Vector2Add(enemy->position, Vector2Scale(move_direction, dt * wander_speed));

        // potentially prepare a charge
        if (Vector2Distance(player->body.position, enemy->position) < minimum_charge_radius)
        {
            if (GetRandomValue(0, 100) < (100 * change_chance_per_second * dt))
            {
                enemy->current_state = ENEMY_CHARGE_PREPARE;
                enemy->state_time_left = charge_prep_time;
                enemy->charge_dir = Vector2Normalize(Vector2Subtract(player->body.position, enemy->position));
            }
        }
        break;
    case ENEMY_CHARGE_PREPARE:
        enemy->state_time_left -= dt;
        if (enemy->state_time_left <= 0)
        {
            enemy->current_state = ENEMY_CHARGING;
            enemy->state_time_left = charge_time;
            play_sfx(SFX_GHOST_CHARGE);
        }
        break;
    case ENEMY_CHARGING:
        enemy->state_time_left -= dt;
        if (enemy->state_time_left <= 0)
        {
            enemy->current_state = ENEMY_POST_CHARGE;
            enemy->state_time_left = charge_post_time;
        }

        enemy->position = Vector2Add(enemy->position, Vector2Scale(enemy->charge_dir, charge_speed * dt));
        break;
    case ENEMY_POST_CHARGE:
        enemy->state_time_left -= dt;
        if (enemy->state_time_left <= 0)
        {
            enemy->current_state = ENEMY_WANDER;
        }
        break;
    case ENEMY_STUNNED:
        // stunned, so drift back to the spot then stop
        enemy->state_time_left -= dt;
        enemy->position = Vector2MoveTowards(enemy->position, enemy->stunned_sent_to, 22 * dt);
        if (enemy->state_time_left <= 0)
            enemy->current_state = ENEMY_WANDER;
        break;
    case ENEMY_DEAD:
        break;
    }

    // try to hurt the player directly
    bool can_hurt_player = !(enemy->current_state == ENEMY_STUNNED || enemy->current_state == ENEMY_DEAD) && player->invincible_time <= 0;
    if (can_hurt_player && CheckCollisionCircles(get_player_center(player), player->body.size.x / 2, enemy->position, 5))
    {
        player->health -= 1;
        player->invincible_time = 3.0;
        play_sfx(SFX_RAT_INJURED);
    }

    // recurse to next enemy
    update_enemies(enemy->next_enemy, dungeon, player, dt);
}

void draw_enemies(Enemy *enemy) {
    if (!enemy) return;

    if (enemy->current_state == ENEMY_DEAD)
    {
    }
    else if (enemy->current_state == ENEMY_STUNNED)
        DrawCircleV(enemy->position, 7, PINK);
    else
        DrawCircleV(enemy->position, 7, RED);

    draw_enemies(enemy->next_enemy);
}

bool try_attack_enemy(Enemy *enemy, Vector2 from_point, Vector2 target_point, float radius)
{
    if (enemy->current_state == ENEMY_DEAD)
        return false;
    if (!CheckCollisionCircles(enemy->position, 7, target_point, radius))
        return false;

    enemy->health -= 1;
    if (enemy->health <= 0)
    {
        // enemy has died!
        enemy->current_state = ENEMY_DEAD;
        play_sfx(SFX_GHOST_DEFEATED);
        return true;
    }

    // enemy has been hit and stunned
    enemy->current_state = ENEMY_STUNNED;
    enemy->stunned_sent_to = Vector2Scale(Vector2Normalize(Vector2Subtract(from_point, enemy->position)), 32);
    enemy->state_time_left = 2.12f;
    play_sfx(SFX_GHOST_INJURED);
    return true;
}