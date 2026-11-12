import unittest

from startup_timing import fixed_clock_countdowns


def countdown(step_ms, actor='12345678', prefix=''):
    return prefix + '\n'.join(
        f'STATE ms={i*step_ms} frame={i+1} ui=31 actor={actor} startup={11-i}'
        for i in range(12)
    )


class StartupTimingCases(unittest.TestCase):
    def test_fast_fixed_steps_fail(self):
        result = fixed_clock_countdowns(countdown(33))
        self.assertEqual(result[0]['elapsed_ms'], 363)
        self.assertFalse(result[0]['paced'])

    def test_wall_time_fixed_steps_pass(self):
        result = fixed_clock_countdowns(countdown(84))
        self.assertEqual(result[0]['intervals'], 11)
        self.assertTrue(result[0]['paced'])

    def test_reset_and_incomplete_countdown_do_not_pass(self):
        prefix = '\n'.join(
            f'STATE ms={i*33} frame={i} ui=31 actor=12345678 startup={n}'
            for i, n in enumerate([11, 10, 9, 12, 12, 12])
        ) + '\n'
        self.assertEqual(fixed_clock_countdowns(prefix), [])
        result = fixed_clock_countdowns(countdown(84, prefix=prefix))
        self.assertEqual(len(result), 1)
        self.assertTrue(result[0]['paced'])

    def test_menu_and_unselected_actor_do_not_count(self):
        self.assertEqual(fixed_clock_countdowns(countdown(84).replace('ui=31', 'ui=30')), [])
        self.assertEqual(fixed_clock_countdowns(countdown(84, actor='00000000')), [])


if __name__ == '__main__':
    unittest.main()
