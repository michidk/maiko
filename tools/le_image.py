"""Read the exact LE image used by this experiment without executing it."""
from dataclasses import dataclass
from pathlib import Path
import hashlib
import struct

RGFX_SHA256 = "844a316b5f71e3c7fadf4b012ab63c5c13aa3606a4f1504f852ac1607ee4dfdf"

@dataclass(frozen=True)
class Object:
    base: int
    size: int
    flags: int
    data: bytes

class Image:
    def __init__(self, path: Path):
        raw = path.read_bytes()
        self.sha256 = hashlib.sha256(raw).hexdigest()
        if self.sha256 != RGFX_SHA256:
            raise ValueError(f"Unsupported RGFX build: {self.sha256}; expected {RGFX_SHA256}")
        self.file_size = len(raw)
        h = struct.unpack_from('<I', raw, 0x3c)[0]
        if raw[h:h+4] != b'LE\0\0':
            raise ValueError('Expected little-endian LE executable')
        u32 = lambda offset: struct.unpack_from('<I', raw, h + offset)[0]
        self.entry_object = u32(24)
        self.entry_offset = u32(28)
        self.stack_object = u32(32)
        self.stack_offset = u32(36)
        page_size = u32(40)
        pages_offset = u32(128)
        self.objects = []
        for index in range(u32(68)):
            size, base, flags, first, count, _ = struct.unpack_from('<6I', raw, h + u32(64) + index * 24)
            data = bytearray(size)
            for j in range(count):
                page_index = first - 1 + j
                entry = h + u32(72) + page_index * 4
                physical = int.from_bytes(raw[entry:entry+3], 'big')
                if raw[entry+3] != 0 or physical == 0:
                    raise ValueError('Unsupported LE page map entry')
                offset = pages_offset + (physical - 1) * page_size
                length = min(page_size, size - j * page_size)
                if offset + length > len(raw):
                    raise ValueError('Truncated LE page')
                data[j * page_size:j * page_size + length] = raw[offset:offset+length]
            self.objects.append(Object(base, size, flags, bytes(data)))

    def at(self, address: int, length: int) -> bytes:
        for obj in self.objects:
            offset = address - obj.base
            if 0 <= offset and offset + length <= obj.size:
                return obj.data[offset:offset+length]
        raise ValueError(f'Address outside image: {address:#x}')

    def report(self):
        return {
            'sha256': self.sha256,
            'file_size': self.file_size,
            'entry': hex(self.objects[self.entry_object-1].base + self.entry_offset),
            'objects': [dict(base=hex(o.base), size=o.size, executable=bool(o.flags & 4), bits=32 if o.flags & 0x2000 else 16) for o in self.objects],
        }
