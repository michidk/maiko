"""Assess complete original fixed-clock countdowns from native STATE logs."""
import re


def fixed_clock_countdowns(text):
    active = {}
    completed = []
    pattern = r'STATE ms=(\d+) frame=(\d+) ui=(\d+) actor=([0-9a-f]+) startup=(\d+)'
    for match in re.finditer(pattern, text):
        ms, frame, ui, actor, remaining = match.groups()
        ms, frame, ui, remaining = map(int, (ms, frame, ui, remaining))
        if ui != 31 or actor == '00000000':
            continue
        sequence = active.get(actor, [])
        if remaining == 11:
            sequence = [(ms, frame, remaining)]
        elif sequence and remaining == sequence[-1][2] - 1:
            sequence.append((ms, frame, remaining))
        elif sequence and remaining != sequence[-1][2]:
            sequence = []
        if sequence and remaining == 0:
            elapsed = (ms - sequence[0][0]) & 0xffffffff
            intervals = len(sequence) - 1
            # Twelve-Hz steps span 916.7 ms over eleven intervals. Allow
            # presentation jitter, since STATE is logged after rendering.
            completed.append(dict(
                actor=actor, intervals=intervals, elapsed_ms=elapsed,
                rendered_frames=frame-sequence[0][1],
                paced=intervals == 11 and elapsed >= 850,
            ))
            sequence = []
        active[actor] = sequence
    return completed
