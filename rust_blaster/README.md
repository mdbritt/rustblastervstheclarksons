# RUST BLASTER

A fast-paced 3D first-person shooter built entirely in Rust.

## How to Run

### 1. Install Rust

**Windows:** Download from https://www.rust-lang.org/tools/install

**Mac/Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Build and Run (No extra dependencies needed!)

```bash
cd rust_blaster
cargo run --release
```

The `--release` flag is important for smooth 60 FPS gameplay!

## Controls

| Key | Action |
|-----|--------|
| W/A/S/D | Move forward/left/back/right |
| Mouse | Look around |
| Left Click | Shoot |
| 1/2/3/4 | Switch weapons |
| Mouse Wheel | Cycle weapons |
| Shift | Sprint |
| ESC | Pause |

## Weapons

1. **PISTOL** - Reliable sidearm with infinite ammo
2. **SHOTGUN** - 8-pellet spread, devastating at close range
3. **MACHINE GUN** - Rapid fire, moderate accuracy
4. **ROCKET LAUNCHER** - Explosive splash damage

## Enemies

- **GRUNT** (Red) - Basic enemy, moderate speed
- **HEAVY** (Purple) - Slow but tanky, hits hard
- **DEMON** (Orange) - Fast and aggressive

## Objectives

1. Kill all enemies on the level
2. Find the green exit pad
3. Progress through all 3 levels
4. Get the highest score!

## Project Structure

```
rust_blaster/
├── Cargo.toml          # Project config & dependencies
├── README.md           # This file
└── src/
    └── main.rs         # Complete game (~1400 lines)
```

## Technical Details

- **Engine:** raylib (via raylib-rs bindings)
- **Rendering:** Real-time 3D with perspective projection
- **Physics:** Custom collision detection with wall sliding
- **AI:** Simple pathfinding toward player
- **Effects:** Particle system, screen shake, muzzle flash

## Troubleshooting

**Build errors about raylib:**
Make sure you have the development dependencies installed for your platform (see step 2).

**Low FPS:**
Make sure you're running with `--release`:
```bash
cargo run --release
```

**Mouse not working:**
The game captures your mouse. Press ESC to pause and regain cursor control.

## Credits

Built with:
- [Rust](https://www.rust-lang.org/)
- [macroquad](https://github.com/not-fl3/macroquad) - Pure Rust game library, no native deps!

Have fun blasting!
