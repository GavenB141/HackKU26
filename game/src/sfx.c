#include <raylib.h>
#include <stddef.h>
#include "sfx.h"

static Sound sfx_data[_SFX_MAX_ID] = {0};

void init_sfx() {
    InitAudioDevice();

    sfx_data[SFX_CHEST_BREAK] = LoadSound("assets/sfx/chest_break.wav");
    sfx_data[SFX_DOOR_UNLOCK] = LoadSound("assets/sfx/door_unlock.wav");
    sfx_data[SFX_GET_KEY] = LoadSound("assets/sfx/get_key.wav");
    sfx_data[SFX_GHOST_CHARGE] = LoadSound("assets/sfx/ghost_charge.wav");
    sfx_data[SFX_GHOST_DEFEATED] = LoadSound("assets/sfx/ghost_defeated.wav");
    sfx_data[SFX_GHOST_INJURED] = LoadSound("assets/sfx/ghost_injured.wav");
    sfx_data[SFX_HAMMER_HIT] = LoadSound("assets/sfx/hammer_hit.wav");
    sfx_data[SFX_HAMMER_READY] = LoadSound("assets/sfx/hammer_ready.wav");
    sfx_data[SFX_RAT_INJURED] = LoadSound("assets/sfx/rat_injured.wav");
    sfx_data[SFX_SWITCH_DEPRESSED] = LoadSound("assets/sfx/switch_depressed.wav");
    sfx_data[SFX_SWITCH_PRESSED] = LoadSound("assets/sfx/switch_pressed.wav");
}

void free_sfx() {
    for (size_t idx = 0; idx < _SFX_MAX_ID; idx++) {
        UnloadSound(sfx_data[idx]);
    }
    CloseAudioDevice();
}

void play_sfx(SfxId id) {
    PlaySound(sfx_data[id]);
}