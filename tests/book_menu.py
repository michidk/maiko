"""Wait for the original book's input gate without modifying game memory."""
from pathlib import Path
import time


class BookMenu:
    def __init__(self, process, symbols):
        self.process = process
        addresses = {}
        for line in Path(symbols).read_text().splitlines():
            fields = line.split()
            if len(fields) == 3 and fields[2] in ('loc_1A824C', 'loc_1F9D84'):
                addresses[fields[2]] = int(fields[0], 16)
        self.mode = addresses['loc_1A824C']
        self.flags = addresses['loc_1F9D84']

    def wait_ready(self, timeout=20):
        # loc_AFF9C sets bit 0 throughout camera and page turns. loc_AC073
        # rejects navigation while it is set. Require a sustained clear gate,
        # since the update briefly clears the bit before recomputing it.
        deadline = time.monotonic() + timeout
        ready_since = None
        with open(f'/proc/{self.process.pid}/mem', 'rb', buffering=0) as memory:
            while time.monotonic() < deadline:
                if self.process.poll() is not None:
                    raise RuntimeError('Game ended before original book became ready')
                memory.seek(self.mode)
                mode = int.from_bytes(memory.read(4), 'little')
                memory.seek(self.flags)
                flags = int.from_bytes(memory.read(1), 'little')
                if mode == 30 and not flags & 1:
                    if ready_since is None:
                        ready_since = time.monotonic()
                    elif time.monotonic() - ready_since >= .3:
                        return
                else:
                    ready_since = None
                time.sleep(.02)
        raise TimeoutError('Original book input gate did not become ready')
