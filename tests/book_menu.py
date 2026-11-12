"""Wait for the original book's input gate without modifying game memory."""
from pathlib import Path
import time


class BookMenu:
    def __init__(self, process, symbols):
        self.process = process
        addresses = {}
        for line in Path(symbols).read_text().splitlines():
            fields = line.split()
            if len(fields) == 3 and fields[2] in ('loc_1A824C', 'loc_1F9D84', 'loc_1A6577'):
                addresses[fields[2]] = int(fields[0], 16)
        self.mode = addresses['loc_1A824C']
        self.flags = addresses['loc_1F9D84']
        self.main_page = addresses['loc_1A6577']

    def main_entry(self, index=None):
        with open(f'/proc/{self.process.pid}/mem', 'rb', buffering=0) as memory:
            def read(address, size=4):
                memory.seek(address)
                value = memory.read(size)
                if len(value) != size:
                    raise OSError('Short read of original book entry')
                return int.from_bytes(value, 'little')
            entry = read(self.main_page + (0x100 if index is None else 0x104))
            if index is not None:
                for _ in range(index):
                    if not entry:
                        raise RuntimeError('Original main-page entry missing')
                    entry = read(entry + 0x31)
            if not entry:
                raise RuntimeError('Original main-page selection missing')
            action = read(entry + 4)
            return {'action': action if action < 0x80000000 else action - 0x100000000,
                    'grayed': bool(read(entry + 0xc, 1) & 8)}

    def continue_available(self):
        entry = self.main_entry(5)
        if entry['action'] != -1:
            raise RuntimeError('Unexpected original main-page Continue action')
        return not entry['grayed']

    def select_main_action(self, action, key):
        self.wait_ready()
        for _ in range(8):
            entry = self.main_entry()
            if entry['action'] == action and not entry['grayed']:
                return
            key('Up')
        raise RuntimeError(f'Original main action {action} unavailable')

    def wait_ready(self, timeout=20):
        # loc_AFF9C sets bit 0 throughout camera and page turns. loc_AC073
        # rejects navigation while it is set. Require a sustained clear gate,
        # since the update briefly clears the bit before recomputing it.
        deadline = time.monotonic() + timeout
        ready_since = None
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError('Game ended before original book became ready')
            try:
                # Wine may not have mapped the guest yet. Reopen after a failed
                # read so an early exec/mapping change cannot leave a stale fd.
                with open(f'/proc/{self.process.pid}/mem', 'rb', buffering=0) as memory:
                    memory.seek(self.mode)
                    mode_bytes = memory.read(4)
                    memory.seek(self.flags)
                    flag_bytes = memory.read(1)
                if len(mode_bytes) != 4 or len(flag_bytes) != 1:
                    raise OSError('Short read of original book state')
            except OSError:
                ready_since = None
                time.sleep(.02)
                continue
            mode = int.from_bytes(mode_bytes, 'little')
            flags = flag_bytes[0]
            if mode == 30 and not flags & 1:
                if ready_since is None:
                    ready_since = time.monotonic()
                elif time.monotonic() - ready_since >= .3:
                    return
            else:
                ready_since = None
            time.sleep(.02)
        raise TimeoutError('Original book input gate did not become ready')
