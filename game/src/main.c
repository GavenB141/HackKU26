#include <raylib.h>
#include <raymath.h>

static const Vector2 canvas_size = {240, 160};
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

int main () {
    InitWindow(canvas_size.x, canvas_size.y, "HackKU 2026");
    SetTargetFPS(144);
    SetWindowMinSize(canvas_size.x, canvas_size.y);
    SetWindowState(FLAG_WINDOW_RESIZABLE);

    canvas = LoadRenderTexture(canvas_size.x, canvas_size.y);

    while (!WindowShouldClose()) {
        BeginTextureMode(canvas);
        ClearBackground(DARKGRAY);
        // Main drawing logic goes here
        EndTextureMode();

        BeginDrawing();
        ClearBackground(BLACK);
        draw_canvas_scaled_to_screen();
        EndDrawing();
    }

    UnloadRenderTexture(canvas);
    CloseWindow();
}
