"""Reject traces that would hide the reported endless-rise failure."""
import unittest
from jump_trace import assess_jump


def sample(y, queued=0, falling=0, held=0):
    return dict(actor='0x20000000', position=[0, y, 0],
                motion=[0, falling, queued], jump_key=held)


class JumpTrace(unittest.TestCase):
    def test_rise_fall_and_supported_landing(self):
        trace = [sample(0), sample(-1024, queued=1, held=1),
                 sample(-2048, falling=1), sample(-1024, falling=1),
                 sample(0), sample(0), sample(0)]
        result = assess_jump(trace)
        self.assertTrue(all(result[key] for key in
                            ('ascent', 'descent', 'landed', 'queued_observed', 'falling_observed')))

    def test_endless_rise_and_frozen_airborne_are_failures(self):
        for trace in [[sample(0), sample(-1024, queued=1), sample(-2048, queued=1)],
                      [sample(0), sample(-1024, falling=1), sample(-1024, falling=1)]]:
            result = assess_jump(trace)
            self.assertTrue(result['ascent'])
            self.assertFalse(result['descent'])
            self.assertFalse(result['landed'])

    def test_height_crossing_or_held_jump_does_not_prove_landing(self):
        for end in [[sample(0, falling=1)] * 3, [sample(0, held=1)] * 3,
                    [sample(0), sample(-1024, queued=1), sample(-2048, queued=1)]]:
            result = assess_jump([sample(0), sample(-2048, falling=1)] + end)
            self.assertFalse(result['landed'])

    def test_actor_change_or_airborne_start_is_rejected(self):
        with self.assertRaises(ValueError):
            assess_jump([sample(0, falling=1)])
        with self.assertRaises(ValueError):
            assess_jump([sample(0, held=1)])
        changed = sample(0)
        changed['actor'] = '0x30000000'
        with self.assertRaises(ValueError):
            assess_jump([sample(0), changed])

    def test_no_delivered_jump_cannot_be_a_landing(self):
        result = assess_jump([sample(0)] * 10)
        self.assertFalse(result['ascent'])
        self.assertFalse(result['descent'])
        self.assertFalse(result['landed'])


if __name__ == '__main__':
    unittest.main()
