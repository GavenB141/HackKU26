#ifndef SFX_H
#define SFX_H

typedef enum SfxId {
    SFX_DOOR_UNLOCK,
    SFX_GET_KEY,
    SFX_GHOST_DEFEATED,
    SFX_GHOST_INJURED,
    SFX_HAMMER_HIT,
    SFX_HAMMER_READY,
    SFX_RAT_INJURED,
    SFX_SWITCH_DEPRESSED,
    SFX_SWITCH_PRESSED,
    _SFX_MAX_ID,
} SfxId;

void init_sfx();
void free_sfx();
void play_sfx(SfxId id);
#endif