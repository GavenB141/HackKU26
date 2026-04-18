# HackKU Game: [Working Title TBD]

Every run begins with a roll of the dice — not on a dungeon layout or enemy stats, but on reality itself.
Before you ever move a character, the game pulls from a fixed library of mad-libs-style world templates — sentence structures with slots for places, factions, villains, McGuffins, tones, and stakes. Word lists fill those slots, and in seconds you have a world: "A crumbling bureaucratic empire built on stolen music is threatened by a guild of amnesiac librarians." That's your setting. The game locks it in and hands it to an LLM, which uses it as a creative brief to design everything that follows — the story arc, the cast of characters, their personalities and relationships, and the layout logic for each level. No two runs share a world.

## Gameplay
At its core, this is a top-down roguelike puzzle game in the spirit of classic Zelda — small self-contained rooms, enemy encounters that reward positioning and timing, and environmental puzzles that gate your progress. Runs are structured as a series of stages, each culminating in a challenge before the story moves forward.
Movement is a first-class mechanic. Beyond walking, players have access to a dash and similar kinetic tools that make navigation feel snappy and expressive — useful for dodging attacks, solving spatial puzzles, and generally rewarding players who develop fluency with the controls.

## Characters That Look the Part
Every character — hero, villain, ally, or enemy — is assembled from a library of composable sprite components: body types, clothing layers, accessories, facial features, and more. Each component is stored as a grayscale or neutral-toned asset and dynamically colorized at runtime using a palette derived from the character's generated personality and faction. A cold, bureaucratic antagonist might be rendered in muted silvers and dusty purples; a chaotic rogue ally gets clashing warm tones that feel vaguely untrustworthy. The same underlying parts produce wildly different-looking characters depending on what the LLM designed. No hand-authored sprite sheet could cover the combinatorial range these worlds demand — this system does it automatically.

## The Living Story
Between stages, the LLM writes in-character dialog — banter, taunts, plot revelations, and reactions — all grounded in the specific world and characters generated for your run. Beat a tough room and your villain might gloat that you got lucky. Fail and crawl back to a checkpoint, and a companion character might offer something between sympathy and mockery. This dialog isn't generic; it references your world, your characters, and roughly where you are in the story arc.
Every line is automatically dubbed using ElevenLabs voice synthesis, giving each character a distinct voice that matches their personality. The result is something closer to a lo-fi audio drama than a silent text box.

## The Social Layer
A cloud backend handles the heavy lifting: proxying all AI and voice requests so players never touch your API keys, storing the seeds and generated content for each run — including the full character assembly data and color palettes — and powering a social sharing interface where players can post their runs for others to browse. Someone else's ridiculous generated world — the amnesiac librarians, the stolen music empire — becomes a thing people can discover, react to, and compare against their own. Every run is a unique artifact, right down to how its characters look.

---

The core loop is tight and familiar. The world wrapped around it is never the same twice.