#include "player.h"
#include "tiles.h"
#include "dungeon.h"
#include <raylib.h>
#include <raymath.h>

static const Vector2 canvas_size = {224, 176};
static RenderTexture canvas;
static void draw_canvas_scaled_to_screen() {
    const int scale_x = GetScreenWidth() / canvas_size.x; 
    const int scale_y = GetScreenHeight() / canvas_size.y;
    const int scale   = scale_x < scale_y ? scale_x : scale_y;

    const Rectangle draw_source = {0, 0, canvas_size.x, -canvas_size.y};
    Rectangle draw_target;
    draw_target.width = canvas_size.x * scale;
    draw_target.height = canvas_size.y * scale;
    draw_target.x = (GetScreenWidth()  - draw_target.width)  / 2;
    draw_target.y = (GetScreenHeight() - draw_target.height) / 2;

    DrawTexturePro(
        canvas.texture, draw_source, draw_target, Vector2Zero(), 0, WHITE);
}

void generic_gray_draw(Texture texture, Rectangle target, unsigned char neighbor_bits) {
    DrawRectangleRec(target, GRAY);
}

void generic_white_draw(Texture texture, Rectangle target, unsigned char neighbor_bits) {
    DrawRectangleRec(target, RAYWHITE);
}

static Camera2D camera = {
    {canvas_size.x / 2 - 24, canvas_size.y / 2},
    {canvas_size.x / 2, canvas_size.y / 2},
    0, 1
};
static void update_camera(const Dungeon* dungeon, float dt) {
    Rectangle bounds = dungeon_room_bounds(dungeon);
    Vector2 target = {bounds.x + bounds.width / 2, bounds.y + bounds.height / 2};
    camera.target = Vector2MoveTowards(camera.target, target, dt * 1000);
}

int main () {
    InitWindow(canvas_size.x * 3, canvas_size.y * 3, "HackKU 2026");
    SetTargetFPS(144);
    SetWindowMinSize(canvas_size.x, canvas_size.y);
    SetWindowState(FLAG_WINDOW_RESIZABLE);

    canvas = LoadRenderTexture(canvas_size.x, canvas_size.y);

    Dungeon* dungeon = make_dungeon();

    add_dungeon_room(
        dungeon, 0, 0, 11, 11,
        "###########"
        "#....#....#"
        "#....#....#"
        "#...####..#"
        "#...#......"
        "#......#..."
        "#...####..#"
        "#.....#...#"
        "#.....#...#"
        "###########"
        "###########"
    );

    add_dungeon_room(
        dungeon, 11, 0, 11, 11,
        "#####..####"
        "#####..####"
        "#####..####"
        "####....###"
        "........###"
        "........###"
        "####....###"
        "###########"
        "###########"
        "###########"
        "###########"
    );

    add_dungeon_room(
        dungeon, 11, -11, 11, 11,
        "###########"
        "####....###"
        "###......##"
        "###......##"
        "###......##"
        "###......##"
        "###......##"
        "####....###"
        "#####..####"
        "#####..####"
        "#####..####"
    );

    Player player = {24,24,12,12};
    
    while (!WindowShouldClose()) {
        float dt = GetFrameTime();

        update_player(&player, dungeon, dt);
        update_camera(dungeon, dt);
        
        BeginTextureMode(canvas);
        ClearBackground(DARKGRAY);
        BeginMode2D(camera);
        draw_dungeon(dungeon, dt);
        draw_player(&player);
        EndMode2D();
        DrawRectangle(192, 0, 48, 160, DARKGRAY);
        EndTextureMode();

        BeginDrawing();
        ClearBackground(BLACK);
        draw_canvas_scaled_to_screen();
        EndDrawing();
    }

    delete_dungeon(dungeon);

    UnloadRenderTexture(canvas);
    CloseWindow();
}


