"""Wait for the original book's input gate without modifying game memory."""
from pathlib import Path
import time


class BookMenu:
    def __init__(self, process, symbols):
        self.process = process
        addresses = {}
        for line in Path(symbols).read_text().splitlines():
            fields = line.split()
            if len(fields) == 3:
                addresses[fields[2]] = int(fields[0], 16)
        self.guest_base = 0
        if 'pointer_offset' in addresses:
            # LLasm executables use PIE and store 32-bit offsets in game data.
            # Resolve ELF symbols independently of those guest pointers.
            executable = Path(f'/proc/{process.pid}/exe').resolve()
            with executable.open('rb') as image:
                header = image.read(18)
            if header[:4] != b'\x7fELF':
                raise RuntimeError('Offset backend probe requires a native ELF process')
            bias = 0
            if int.from_bytes(header[16:18], 'little') == 3:
                for line in Path(f'/proc/{process.pid}/maps').read_text().splitlines():
                    fields = line.split(maxsplit=5)
                    if len(fields) == 6 and fields[2] == '00000000' and fields[5] == str(executable):
                        bias = int(fields[0].split('-')[0], 16)
                        break
                if not bias:
                    raise RuntimeError('Could not locate the translated ELF mapping')
            addresses = {name: value + bias for name, value in addresses.items()}
            with open(f'/proc/{process.pid}/mem', 'rb', buffering=0) as memory:
                memory.seek(addresses['pointer_offset'])
                self.guest_base = int.from_bytes(memory.read(8), 'little')
            if not self.guest_base:
                raise RuntimeError('Guest offset arena has not initialized')
        self.addresses = addresses
        self.mode = addresses['loc_1A824C']
        self.flags = addresses['loc_1F9D84']
        self.main_page = addresses['loc_1A6577']

    def pointer(self, address):
        return self.guest_base + address if address else 0

    def main_entry(self, index=None):
        with open(f'/proc/{self.process.pid}/mem', 'rb', buffering=0) as memory:
            def read(address, size=4):
                memory.seek(address)
                value = memory.read(size)
                if len(value) != size:
                    raise OSError('Short read of original book entry')
                return int.from_bytes(value, 'little')
            entry = self.pointer(read(self.main_page + (0x100 if index is None else 0x104)))
            if index is not None:
                for _ in range(index):
                    if not entry:
                        raise RuntimeError('Original main-page entry missing')
                    entry = self.pointer(read(entry + 0x31))
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
