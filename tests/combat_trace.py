"""Read original actor/combat state without modifying guest memory."""
import struct


class CombatTrace:
    def __init__(self, process, symbols):
        self.process = process
        self.symbols = symbols

    def snapshot(self):
        with open(f"/proc/{self.process.pid}/mem", "rb", buffering=0) as memory:
            def read(address, size):
                memory.seek(address)
                data = memory.read(size)
                if len(data) != size:
                    raise OSError(f"Short guest read at {address:08x}")
                return data

            def word(address):
                return struct.unpack("<I", read(address, 4))[0]

            selected = word(self.symbols["loc_1A3865"])
            actor = word(self.symbols["loc_1A37AE"])
            visited = set()
            actors = []
            while actor:
                if actor in visited:
                    raise RuntimeError("Original actor list changed or is cyclic")
                if len(visited) >= 8192:
                    raise RuntimeError("Original actor list exceeds 8192-observation bound")
                visited.add(actor)
                data = read(actor, 0x276)
                combat = struct.unpack_from("<I", data, 0x23b)[0]
                attributes = struct.unpack_from("<I", data, 0x272)[0]
                animation = struct.unpack_from("<I", data, 0x247)[0]
                if combat:
                    attrs = read(attributes, 121)
                    record = read(combat, 0x18e)
                    actors.append({
                        "actor": hex(actor), "selected": actor == selected,
                        "position": list(struct.unpack_from("<iii", data, 0xcb)),
                        "heading": struct.unpack_from("<I", data, 0x124)[0],
                        "health": attrs[3], "protected": attrs[97],
                        "invulnerable": attrs[120], "lifecycle": data[0x1de],
                        "animation": struct.unpack("<H", read(animation, 2))[0] if animation else None,
                        "action": struct.unpack_from("<i", record, 0x38)[0],
                        "combat_flags": record[0x18d],
                    })
                actor = struct.unpack_from("<I", data, 4)[0]
            return {"actors": actors, "selected": hex(selected), "actor_count": len(visited),
                    "ui_mode": word(self.symbols["loc_1A824C"]),
                    "startup_remaining": word(self.symbols["loc_1A4624"])}
