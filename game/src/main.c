#include "player.h"
#include "dungeon.h"
#include "enemy.h"
#include "sfx.h"
#include <raylib.h>
#include <raymath.h>
#include <stdlib.h>
#include <time.h>

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
    int dungeon_id;
} state = {
    .camera =  {
        {CANVAS_SIZE.x / 2 - 24, CANVAS_SIZE.y / 2},
        {CANVAS_SIZE.x / 2, CANVAS_SIZE.y / 2},
        0, 1
    }
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

static void draw_hud() {
    // precalculate relevant positions
    const float canvas_left = CANVAS_SIZE.x - 48;
    const float health_offset = TILE_SIZE * 0;
    const float key_offset = TILE_SIZE * 1;

    // UI background
    DrawRectangle(canvas_left, 0, 48, CANVAS_SIZE.y, DARKGRAY);

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

    // Draw dungeon id
    draw_number((Vector2){CANVAS_SIZE.x - 12, CANVAS_SIZE.y - 10}, state.dungeon_id);
}

static void load_random_dungeon() {
    int rn = rand() % 166 + 1;
    state.dungeon_id = rn;
    
    if (!state.player) state.player = make_player();
    if (state.dungeon) delete_dungeon(state.dungeon);
    char* file = LoadFileText(TextFormat("assets/dungeons/%d", rn));
    state.dungeon = parse_dungeon(file);
    UnloadFileText(file);
    state.player->body.position = state.dungeon->spawn_point;
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

    load_random_dungeon();

    float elapsed = 0;
    while (!WindowShouldClose()) {
        float dt = GetFrameTime();
        elapsed += dt;

        update_player(state.player, state.dungeon, dt);
        update_camera(dt);
        update_enemies(state.dungeon->rooms[state.dungeon->active_room].enemy,
                       state.dungeon,
                       state.player,
                       dt);

        BeginTextureMode(state.canvas);
        ClearBackground(DARKGRAY);
        BeginMode2D(state.camera);
        draw_dungeon(state.dungeon, dt);
        draw_enemies(state.dungeon->rooms[state.dungeon->active_room].enemy, elapsed);
        draw_player(state.player, dt);
        EndMode2D();
        draw_hud();
        EndTextureMode();

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


