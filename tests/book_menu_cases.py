"""Exercise startup observation while Wine is still mapping the guest image."""
import io
import unittest
from unittest.mock import Mock, patch

from book_menu import BookMenu


class BookMappingCases(unittest.TestCase):
    def menu(self):
        menu = BookMenu.__new__(BookMenu)
        menu.process = Mock(pid=123)
        menu.process.poll.return_value = None
        menu.mode = 0
        menu.flags = 4
        return menu

    def test_transient_unmapped_image_recovers(self):
        reads = []

        def mapped(*args, **kwargs):
            reads.append(1)
            if len(reads) <= 2:
                raise OSError('Guest image not mapped yet')
            return io.BytesIO(b'\x1e\x00\x00\x00\x00')

        with patch('book_menu.open', side_effect=mapped):
            self.menu().wait_ready(timeout=1)
        self.assertGreater(len(reads), 2)

    def test_unmapped_image_times_out(self):
        with patch('book_menu.open', side_effect=OSError('Unmapped')):
            with self.assertRaises(TimeoutError):
                self.menu().wait_ready(timeout=.05)

    def test_exited_process_is_reported(self):
        menu = self.menu()
        menu.process.poll.return_value = 3
        with self.assertRaisesRegex(RuntimeError, 'Game ended'):
            menu.wait_ready()

    def test_short_read_cannot_look_ready(self):
        with patch('book_menu.open', side_effect=lambda *a, **k: io.BytesIO(b'\x1e')):
            with self.assertRaises(TimeoutError):
                self.menu().wait_ready(timeout=.05)


if __name__ == '__main__':
    unittest.main()
