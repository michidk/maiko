"""A build must reject source drift, including newly added inputs."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('macos_backend', ROOT / 'check_macos_backend.py')
backend = importlib.util.module_from_spec(spec)
spec.loader.exec_module(backend)


class SourceManifest(unittest.TestCase):
    def test_changed_and_added_sources_are_rejected_but_bytecode_is_ignored(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ['check_macos_backend.py', 'macos/test.c', 'runtime/test.h',
                         *('patches/' + name for name in backend.PATCHES)]:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('original input\n')
            expected = backend.source_manifest(root)
            backend.verify_source_manifest(expected, root)
            source = root / 'macos/test.c'
            source.write_text('changed input\n')
            with self.assertRaisesRegex(RuntimeError, 'Source inputs changed'):
                backend.verify_source_manifest(expected, root)
            source.write_text('original input\n')
            added = root / 'runtime/new.h'
            added.write_text('new input\n')
            with self.assertRaisesRegex(RuntimeError, 'Source inputs changed'):
                backend.verify_source_manifest(expected, root)
            added.unlink()
            bytecode = root / 'macos/__pycache__/test.pyc'
            bytecode.parent.mkdir()
            bytecode.write_bytes(b'not a build input')
            backend.verify_source_manifest(expected, root)


if __name__ == '__main__':
    unittest.main()
