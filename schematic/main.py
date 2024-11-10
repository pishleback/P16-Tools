import mcschematic
import json

def validate_memory(memory):
    assert(type(memory) is dict)
    assert(set(memory.keys()) == {"rom", "ram"})
    assert(type(memory["rom"]) is list)
    assert(len(memory["rom"]) == 16)
    for rom_page in memory["rom"]:
        assert(type(rom_page) == list)
        assert(len(rom_page) == 2 ** 8)
        for n in rom_page:
            assert(type(n) == int)
            assert(0 <= n < 16)
    assert(type(memory["ram"]) == list)
    assert(len(memory["ram"]) == 2 ** 12)
    for v in memory["ram"]:
        assert(type(v) == int)
        assert(0 <= v < 2 ** 16)

def place_barrel(schem, x, y, z, ss):
    assert(type(ss) is int)
    assert(0 <= ss < 16)
    n = [0, 123, 246, 370, 493, 617, 740, 863, 987, 1110, 1234, 1357, 1481, 1604, 1727, 1728][ss]
    items = []
    while n >= 64:
            items.append(64)
            n -= 64
    if n != 0:
        items.append(n)
        n = 0
    schem.setBlock((x, y, z), "minecraft:barrel[facing=up]")
    schem._structure._blockEntities[(x, y, z)] = """{Items:[""" + ", ".join("""{Slot:""" + str(idx) + """b, Count:""" + str(count) + """b, id:"minecraft:redstone"}""" for idx, count in enumerate(items)) + """]}"""

def place_barrel_or_glass(schem, x, y, z, ss):
    assert(type(ss) is int)
    assert(0 <= ss < 16)
    if ss == 0:
        schem.setBlock((x, y, z), "minecraft:glass")
    else:
        place_barrel(schem, x, y, z, ss)

def make_torch_rom_page(schem, ox, oy, oz, page):
    def set_nibble(x, y, z, n):
        for i in range(4):
            dx = -2 * i
            if n & 2 ** (3 - i) != 0:
                block = "minecraft:redstone_wall_torch[facing=north]"
            else:
                block = "minecraft:glass"
            schem.setBlock((x + dx, y, z), block)

    for (i, n) in enumerate(page):  
        q, r = divmod(i, 32)
        set_nibble(ox - 8 * q, oy, oz - 2 * r, n)

def make_barrel_rom_page(schem, ox, oy, oz, page):
    for a in range(8):
        for d in range(32):
            place_barrel_or_glass(schem, ox - 2 * d, oy - 2 * a, oz, page[d + 32 * a])


def make_rom_schem(schem, memory, rom_page):
    assert(type(rom_page) == int)
    assert(0 <= rom_page < 16)
    if rom_page in {1, 2, 3}:
        make_torch_rom_page(schem, -5, -10 - 5 * (rom_page - 1), -5, memory["rom"][rom_page])
    elif rom_page in {4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15}:
        make_barrel_rom_page(schem, -13, -11 - (16 if rom_page % 2 == 0 else 0), 13 + 4 * ((rom_page - 4) // 2), memory["rom"][rom_page])
    else:
        raise NotImplementedError()

with open("../memory.json", "r") as f:
    memory = json.load(f)
validate_memory(memory)

schem = mcschematic.MCSchematic()
make_rom_schem(schem, memory, 1)
make_rom_schem(schem, memory, 2)
make_rom_schem(schem, memory, 3)
make_rom_schem(schem, memory, 4)
make_rom_schem(schem, memory, 5)
make_rom_schem(schem, memory, 6)
make_rom_schem(schem, memory, 7)
make_rom_schem(schem, memory, 8)
make_rom_schem(schem, memory, 9)
make_rom_schem(schem, memory, 10)
make_rom_schem(schem, memory, 11)
make_rom_schem(schem, memory, 12)
make_rom_schem(schem, memory, 13)
make_rom_schem(schem, memory, 14)
make_rom_schem(schem, memory, 15)

schem.save("..", "memory", mcschematic.Version.JE_1_18_2)