#include "player.h"
#include "dungeon.h"
#include "enemy.h"
#include "sfx.h"
#include <raylib.h>
#include <raymath.h>
#include <stdlib.h>
#include <time.h>

static const Vector2 CANVAS_SIZE = {224, 176};

static struct GameState {
    const Vector2 canvas_size;
    RenderTexture canvas;
    Camera2D camera;
    Player* player;
    Dungeon* dungeon;
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

static void update_camera(const Dungeon* dungeon, float dt) {
    Rectangle bounds = dungeon_room_bounds(dungeon);
    Vector2 target = {bounds.x + bounds.width / 2, bounds.y + bounds.height / 2};
    state.camera.target = Vector2MoveTowards(state.camera.target, target, dt * 1000);
}

static void draw_hud(const Player *player, Texture texture) {
    // precalculate relevant positions
    const float canvas_left = CANVAS_SIZE.x - 48;
    const float canvas_tile_size = 16;
    const float health_offset = canvas_tile_size * 2;
    const float key_offset = canvas_tile_size * 4;
    // UI background
    DrawRectangle(canvas_left, 0, 48, CANVAS_SIZE.y, DARKGRAY);

    // Draw the player health status
    DrawText("HEALTH", canvas_left, canvas_tile_size + 4, canvas_tile_size - 4, WHITE);
    for (int heart = 0; heart < player->health; heart++)
    {
        // draw
        Rectangle src = {5*canvas_tile_size, 0, 16, 16};
        Rectangle target = {canvas_left + (canvas_tile_size * heart), health_offset, 16, 16};
        DrawTexturePro(texture, src, target, Vector2Zero(), 0, WHITE);
    }

    // Draw the player health status
    DrawText("KEYS", canvas_left, 3*canvas_tile_size + 4, canvas_tile_size - 4, WHITE);
    for (int key = 0; key < player->keys; key++)
    {
        // draw
        Rectangle src = {3*canvas_tile_size, 1*canvas_tile_size, 16, 16};
        Rectangle target = {canvas_left + (canvas_tile_size * key), key_offset, 16, 16};
        DrawTexturePro(texture, src, target, Vector2Zero(), 0, WHITE);
    }
}

static void load_random_dungeon() {
    int rn = rand() % 10000 + 1;
    
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

    load_random_dungeon();

    Texture item_texture = LoadTexture("assets/item_tiles.png");
    
    while (!WindowShouldClose()) {
        float dt = GetFrameTime();

        update_player(state.player, state.dungeon, dt);
        update_camera(state.dungeon, dt);
        update_enemies(state.dungeon->rooms[state.dungeon->active_room].enemy,
                       state.dungeon,
                       state.player,
                       dt);

        BeginTextureMode(state.canvas);
        ClearBackground(DARKGRAY);
        BeginMode2D(state.camera);
        draw_dungeon(state.dungeon, dt);
        draw_enemies(state.dungeon->rooms[state.dungeon->active_room].enemy);
        draw_player(state.player, dt);
        EndMode2D();
        draw_hud(state.player, item_texture);
        EndTextureMode();

        BeginDrawing();
        ClearBackground(BLACK);
        draw_canvas_scaled_to_screen();
        EndDrawing();
    }

    delete_dungeon(state.dungeon);
    delete_player(state.player);

    UnloadRenderTexture(state.canvas);
    CloseWindow();
}


