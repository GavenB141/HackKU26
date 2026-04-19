#include "player.h"
#include "dungeon.h"
#include "enemy.h"
#include "sfx.h"
#include <raylib.h>
#include <raymath.h>
#include <stdlib.h>
#include <time.h>
#include <limits.h>

#define TILE_SIZE 16

static const Vector2 CANVAS_SIZE = {224, 176};

static struct GameState {
    const Vector2 canvas_size;
    RenderTexture canvas;
    Camera2D camera;
    Player* player;
    Dungeon* dungeon;
    Texture nums_tex;
    Texture item_tex;
    int level_number;
    int dungeon_id;
    enum
    {
        GS_MAIN_MENU,
        GS_DUNGEON_CRAWL,
    } screen;
    float fade_alpha;
    bool fading_out;
} state = {
    .camera = {
        {CANVAS_SIZE.x / 2 - 24, CANVAS_SIZE.y / 2},
        {CANVAS_SIZE.x / 2, CANVAS_SIZE.y / 2},
        0,
        1},
    .screen = GS_MAIN_MENU,
};

static void draw_canvas_scaled_to_screen() {
    const int scale_x = GetScreenWidth() / CANVAS_SIZE.x; 
    const int scale_y = GetScreenHeight() / CANVAS_SIZE.y;
    const int scale   = scale_x < scale_y ? scale_x : scale_y;

    const Rectangle draw_source = {0, 0, CANVAS_SIZE.x, -CANVAS_SIZE.y};
    Rectangle draw_target;
    draw_target.width = CANVAS_SIZE.x * scale;
    draw_target.height = CANVAS_SIZE.y * scale;
    draw_target.x = (GetScreenWidth()  - draw_target.width)  / 2;
    draw_target.y = (GetScreenHeight() - draw_target.height) / 2;

    DrawTexturePro(
        state.canvas.texture, draw_source, draw_target, Vector2Zero(), 0, WHITE);
}

static void update_camera(float dt) {
    Rectangle bounds = dungeon_room_bounds(state.dungeon);
    Vector2 target = {bounds.x + bounds.width / 2, bounds.y + bounds.height / 2};
    state.camera.target = Vector2MoveTowards(state.camera.target, target, dt * 1000);
}

static void draw_number(Vector2 first_digit_loc, unsigned int number) {
    Rectangle src = {0, 0, 7, 8};

    do {
        int digit = number % 10;
        src.x = (digit % 5) * 7;
        src.y = (int)(digit / 5) * 8;
        DrawTextureRec(state.nums_tex, src, first_digit_loc, WHITE);
        first_digit_loc.x -= src.width + 1;
        number /= 10;
    } while (number);
}

static bool room_has_stairs(const DungeonRoom* r) {
    int n = r->map->width * r->map->height;
    for (int i = 0; i < n; i++) {
        if (r->map->map[i].type == 'X') return true;
    }
    return false;
}

static void draw_minimap() {
    if (!state.dungeon || state.dungeon->num_rooms == 0) return;

    const int CELL = 4;
    const float map_x = CANVAS_SIZE.x - 48 + 4;
    const float map_y = TILE_SIZE * 3 + 4;

    int min_gx = INT_MAX, min_gy = INT_MAX;
    for (int i = 0; i < state.dungeon->num_rooms; i++) {
        DungeonRoom* r = &state.dungeon->rooms[i];
        int gx = r->origin_x / 11;
        int gy = r->origin_y / 11;
        if (gx < min_gx) min_gx = gx;
        if (gy < min_gy) min_gy = gy;
    }

    for (int i = 0; i < state.dungeon->num_rooms; i++) {
        DungeonRoom* r = &state.dungeon->rooms[i];
        if (!r->explored) continue;
        int gx = r->origin_x / 11 - min_gx;
        int gy = r->origin_y / 11 - min_gy;
        Color col = (i == state.dungeon->active_room) ? WHITE
                  : room_has_stairs(r)               ? RED
                  : (Color){110, 110, 110, 255};
        DrawRectangle((int)(map_x + gx * CELL), (int)(map_y + gy * CELL), CELL, CELL, col);
    }
}

static void draw_hud() {
    // precalculate relevant positions
    const float canvas_left = CANVAS_SIZE.x - 48;
    const float level_offset = TILE_SIZE * 0;
    const float health_offset = TILE_SIZE * 1;
    const float key_offset = TILE_SIZE * 2;

    // UI background
    DrawRectangle(canvas_left, 0, 48, CANVAS_SIZE.y, DARKGRAY);

    // Draw level number
    DrawText("Lvl", canvas_left + 4, level_offset + 3, 10, WHITE);
    draw_number((Vector2){CANVAS_SIZE.x - 8, level_offset + 3}, state.level_number);

    // Draw the player health status
    for (int heart = 0; heart < state.player->health; heart++) {
        Rectangle src = {5 * TILE_SIZE, 0, 16, 16};
        Rectangle target = {canvas_left + (TILE_SIZE * heart), health_offset, 16, 16};
        DrawTexturePro(state.item_tex, src, target, Vector2Zero(), 0, WHITE);
    }

    // Draw player's key count
    Rectangle src = {3 * TILE_SIZE, TILE_SIZE, 16, 16};
    Rectangle target = {canvas_left + 4, key_offset, 16, 16};
    DrawTexturePro(state.item_tex, src, target, Vector2Zero(), 0, WHITE);
    draw_number((Vector2){CANVAS_SIZE.x - 12, key_offset + 4}, state.player->keys);

    // Draw minimap above dungeon id
    draw_minimap();

    // Draw dungeon id
    draw_number((Vector2){CANVAS_SIZE.x - 12, CANVAS_SIZE.y - 10}, state.dungeon_id);
}

static void load_random_dungeon() {
    int rn = rand() % 166 + 1;
    state.dungeon_id = rn;
    state.level_number++;
    
    if (!state.player) state.player = make_player();
    if (state.dungeon) delete_dungeon(state.dungeon);
    char* file = LoadFileText(TextFormat("assets/dungeons/%d", rn));
    state.dungeon = parse_dungeon(file);
    UnloadFileText(file);
    state.player->body.position = state.dungeon->spawn_point;
}

static void handle_transitions(float dt) {
    if (state.player->on_stairs && !state.fading_out && state.fade_alpha == 0)
        state.fading_out = true;

    if (state.fading_out) {
        state.fade_alpha += dt * 2.0f;
        if (state.fade_alpha >= 1.0f) {
            state.fade_alpha = 1.0f;
            state.fading_out = false;
            load_random_dungeon();
        }
    } else if (state.fade_alpha > 0) {
        state.fade_alpha -= dt * 2.0f;
        if (state.fade_alpha < 0) state.fade_alpha = 0;
    }
}

int main () {
    srand(time(0));

    InitWindow(CANVAS_SIZE.x * 3, CANVAS_SIZE.y * 3, "HackKU 2026");
    SetTargetFPS(144);
    SetWindowMinSize(CANVAS_SIZE.x, CANVAS_SIZE.y);
    SetWindowState(FLAG_WINDOW_RESIZABLE);

    init_sfx();

    state.canvas = LoadRenderTexture(CANVAS_SIZE.x, CANVAS_SIZE.y);
    state.nums_tex = LoadTexture("assets/numerals.png");
    state.item_tex = LoadTexture("assets/item_tiles.png");

    Texture2D menu_splash = LoadTexture("assets/title.png");

    load_random_dungeon();

    float elapsed = 0;
    while (!WindowShouldClose()) {
        float dt = GetFrameTime();
        elapsed += dt;

        switch (state.screen)
        {
        case GS_MAIN_MENU:
            // render
            BeginTextureMode(state.canvas);
            DrawTexture(menu_splash, 0, 0, WHITE);
            EndTextureMode();
            // input
            if (IsKeyReleased(KEY_SPACE))
            {
                state.screen = GS_DUNGEON_CRAWL;
            }

            break;
        case GS_DUNGEON_CRAWL:
            update_player(state.player, state.dungeon, dt);
            update_camera(dt);
            update_enemies(state.dungeon->rooms[state.dungeon->active_room].enemy,
                           state.dungeon,
                           state.player,
                           dt);

            if (state.player->health <= 0)
            {
                state.screen = GS_MAIN_MENU;
            }

            BeginTextureMode(state.canvas);
            ClearBackground(DARKGRAY);
            BeginMode2D(state.camera);
            draw_dungeon(state.dungeon, dt);
            draw_enemies(state.dungeon->rooms[state.dungeon->active_room].enemy, elapsed);
            draw_player(state.player, dt);
            EndMode2D();
            draw_hud();
            EndTextureMode();
            handle_transitions(dt);
            break;
        }

        BeginDrawing();
        ClearBackground(BLACK);
        draw_canvas_scaled_to_screen();
        EndDrawing();
    }

    delete_dungeon(state.dungeon);
    delete_player(state.player);

    UnloadTexture(state.nums_tex);
    UnloadTexture(state.item_tex);
    UnloadRenderTexture(state.canvas);
    CloseWindow();
}


