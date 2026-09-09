#!/usr/bin/env python3
"""
Python offline level & state compiler for Revenant engine.
Doom-style offline tool (Rule 1, Rule 4, Rule 5):
- Generates/packs level geometry and sector connectivity.
- Precomputes spatial structures and entity spawn tables.
"""

import struct
import sys
import os

MAP_W = 96
MAP_H = 64
SECTOR_W = 8
SECTOR_H = 8

def hash_coord(x: int, y: int) -> int:
    n = (x * 374761393) ^ (y * 668265263) ^ 0x9e3779b9
    n = n & 0xFFFFFFFF
    n = (n ^ (n >> 13)) * 1274126177
    n = n & 0xFFFFFFFF
    return (n ^ (n >> 16)) & 0xFFFFFFFF

def generate_map():
    grid = bytearray(MAP_W * MAP_H)
    for y in range(MAP_H):
        for x in range(MAP_W):
            idx = y * MAP_W + x
            if x == 0 or y == 0 or x == MAP_W - 1 or y == MAP_H - 1:
                grid[idx] = 1
                continue
            if x == MAP_W - 4 and y == MAP_H - 4:
                grid[idx] = 9
                continue

            sx = x // SECTOR_W
            sy = y // SECTOR_H
            lx = x % SECTOR_W
            ly = y % SECTOR_H
            room_material = (hash_coord(sx, sy) % 5 + 1)

            if lx == 0 or lx == SECTOR_W - 1:
                if (ly == 3 or ly == 4) and 0 < sx < (MAP_W // SECTOR_W):
                    grid[idx] = 0
                else:
                    grid[idx] = room_material
                continue

            if ly == 0 or ly == SECTOR_H - 1:
                if (lx == 3 or lx == 4) and 0 < sy < (MAP_H // SECTOR_H):
                    grid[idx] = 0
                else:
                    grid[idx] = room_material
                continue

            variant = hash_coord(sx * 7 + lx, sy * 11 + ly) % 11
            pillar = (lx == 2 or lx == 5) and (ly == 2 or ly == 5) and variant < 3
            if pillar:
                grid[idx] = room_material
            else:
                grid[idx] = 0

    return grid

def generate_spawns():
    enemy_spawns = []
    health_pickups = []
    ammo_pickups = []

    for sy in range(MAP_H // SECTOR_H):
        for sx in range(MAP_W // SECTOR_W):
            if (sx, sy) == (0, 0) or (sx, sy) == (MAP_W // SECTOR_W - 1, MAP_H // SECTOR_H - 1):
                continue
            cx = sx * SECTOR_W + 4
            cy = sy * SECTOR_H + 4
            roll = hash_coord(sx, sy) % 100
            if sy >= 5 or (sx >= 8 and sy >= 2):
                tier = 2
            elif sy >= 2 or sx >= 5:
                tier = 1
            else:
                tier = 0

            if roll < 70:
                enemy_spawns.append((cx + 0.5, cy + 0.5, tier))
            if tier >= 1 and roll % 3 == 0:
                enemy_spawns.append(((cx - 1) + 0.5, cy + 0.5, tier))
            if roll % 13 == 0:
                health_pickups.append((cx + 0.5, (cy - 1) + 0.5))
            if roll % 9 == 0:
                ammo_pickups.append(((cx + 1) + 0.5, cy + 0.5))

    return (4.5, 4.5), enemy_spawns, health_pickups, ammo_pickups

def pack_level(output_path: str):
    grid = generate_map()
    player_start, enemy_spawns, health_pickups, ammo_pickups = generate_spawns()

    header = struct.pack("<4sIIIIff", b"RVLV", 1, MAP_W, MAP_H, len(grid), player_start[0], player_start[1])
    
    enemy_bytes = struct.pack("<I", len(enemy_spawns))
    for ex, ey, tier in enemy_spawns:
        enemy_bytes += struct.pack("<ffB3x", ex, ey, tier)

    health_bytes = struct.pack("<I", len(health_pickups))
    for hx, hy in health_pickups:
        health_bytes += struct.pack("<ff", hx, hy)

    ammo_bytes = struct.pack("<I", len(ammo_pickups))
    for ax, ay in ammo_pickups:
        ammo_bytes += struct.pack("<ff", ax, ay)

    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, "wb") as f:
        f.write(header)
        f.write(grid)
        f.write(enemy_bytes)
        f.write(health_bytes)
        f.write(ammo_bytes)

    print(f"Packed level to {output_path} ({os.path.getsize(output_path)} bytes)")

if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "content/levels/dungeon.lvl"
    pack_level(out)
