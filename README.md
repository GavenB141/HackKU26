# HammeRat

The first AAAA roguelike dungeon crawler where you play as a determined rat armed with an oversized hammer. Your mission? Descend through ever-changing floors of a haunted dungeon to claim the legendary cheese waiting at the bottom.

Ghosts, traps, and procedural chaos stand between you and that glorious wheel. Swing hard, dodge fast, and you may discover the legendary Cheddar.

## Gameplay

- **Procedural Dungeons**: Every descent is unique. Floors are randomly generated with winding corridors, chambers, and surprises.
- **Combat**: Bash ghosts in a manner that makes no logical sense with your trusty hammer. Timing and positioning matter, as you only get three strikes and your out.
- **The Goal**: Reach the deepest level and claim the cheese. Simple in concept, brutally replayable in practice.
- **Permadeath**: Classic roguelike style. Die, learn, descend again.

## Key Features

- **Rat**: You're rodent.
- **Smart Level Generation**: We drew inspiration from evolutionary genetic algorithm approaches to craft dungeons that change every time you descend. 
- **Ghost-Hammering Action**: Fluid movement and satisfying combat in a top-down view.
- **Replayability**: New seeds, new layouts, new ways to get absolutely wrecked by ghosts.

Built for HackKU26. The project has evolved quite a bit since the initial concept — this is the current version: pure rat-powered roguelike goodness.

## How We Built It

- **Language & Engine**: Written entirely in **C** using [raylib](https://www.raylib.com/) for rendering, input, and audio.
- **Deployment**: Compiled to **WebAssembly** and hosted on GitHub Pages — so anyone can jump in and play directly in their browser with zero installation.
- **Dungeon Generation**: Powered by a **genetic algorithm** inspired by evolutionary PCG techniques (specifically drawing from research on generating coherent dungeon maps with locked-door style progression and connectivity). We used AI tools to help implement and tune the evolutionary system for better layout quality, pathing, and replayability.

The whole thing stays lightweight, runs smoothly in the browser, and still feels like a proper roguelike descent.

## Controls 

- Move: Arrow keys or WASD
- Hammer swing: Space 
- Dash: Left Shift

## Play the Game
https://gavenb141.github.io/HackKU26/
