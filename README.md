# Revenant

![Revenant Demo](assets/firstvisualoutput.gif)


## Run

```
cargo run --release
```

## Controls

- `W` / `S` — move forward / backward
- `A` / `D` — strafe left / right
- `Left` / `Right` — turn
- `Space` — fire
- `Esc` — quit

Walk into the marked exit tile in the south-west room to clear the level.
Green pickups restore health, yellow pickups restore ammo.

## Fullscreen / Display

1. **Run the game:**
   ```bash
   cargo run --release
   ```
2. **Maximize / Fullscreen the window:**
   - **Tiling / Hyprland / Sway**: Press your window manager's fullscreen key (e.g. `Super + F` or `Super + M`).
   - **GNOME / KDE / XFCE**: Press `Super + Up` or `Alt + F11` (or click the maximize button).

The viewport and HUD will scale up cleanly to fill your entire screen while preserving the correct retro aspect ratio.
