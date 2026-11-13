"""Assess observed original actor state, without changing game memory or physics."""


def assess_jump(samples, tolerance=512):
    if not samples:
        raise ValueError('A jump needs an initial grounded sample')
    initial = samples[0]
    baseline = initial['position'][1]
    if initial['motion'][1:] != [0, 0]:
        raise ValueError('Jump started before queued and falling motion cleared')
    if initial['jump_key'] != 0:
        raise ValueError('A separate jump must start with the jump key released')
    if any(sample['actor'] != initial['actor'] for sample in samples):
        raise ValueError('Selected actor changed during jump observation')
    apex = min(range(len(samples)), key=lambda i: samples[i]['position'][1])
    highest = samples[apex]['position'][1]
    ascent = highest < baseline - tolerance
    descent = any(sample['position'][1] > highest + tolerance
                  for sample in samples[apex + 1:])
    landing = next((i for i in range(apex + 1, len(samples))
                    if abs(samples[i]['position'][1] - baseline) <= tolerance
                    and samples[i]['motion'][1:] == [0, 0]
                    and samples[i]['jump_key'] == 0), None)
    # A passing trace must end supported and stay there after the landing.
    # A single intermediate height crossing does not prove landing.
    landed = ascent and descent and landing is not None and len(samples) - landing >= 3 and all(
        abs(sample['position'][1] - baseline) <= tolerance
        and sample['motion'][1:] == [0, 0] and sample['jump_key'] == 0
        for sample in samples[landing:])
    return dict(ascent=ascent, descent=descent, landed=landed,
                queued_observed=any(sample['motion'][2] for sample in samples),
                falling_observed=any(sample['motion'][1] for sample in samples),
                baseline_y=baseline, apex_y=highest, final_y=samples[-1]['position'][1],
                apex_index=apex, landing_index=landing)
