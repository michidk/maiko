"""Exercise Mach-O assembly of the real SR ABI fixture, without a Mac SDK."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[1]

class MacAssembly(unittest.TestCase):
    def test_guest_callback_object_has_darwin_symbols_and_executable_text(self):
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)/'entry.o'
            result=subprocess.run([sys.executable,str(ROOT/'macos/assemble.py'),
                str(ROOT/'runtime/native_entry.asm'),str(output),
                '--include',str(ROOT/'.cache/sr/SR/x64-support'),'--define','ABI_TEST'],
                capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stdout+result.stderr)
            header=output.read_bytes()[:8]
            self.assertEqual(header,bytes.fromhex('cffaedfe07000001'))
            symbols=subprocess.check_output(['llvm-nm',str(output)],text=True)
            self.assertIn(' T _rg_call',symbols)
            self.assertIn(' U _rg_service',symbols)
            self.assertIn(' T _abi_host_roundtrip',symbols)
            self.assertNotIn(' U rg_service',symbols)
            sections=subprocess.check_output(['llvm-objdump','--macho','--section-headers',str(output)],text=True)
            self.assertIn('__text',sections)
            self.assertNotIn('.note.GNU-stack',sections)

if __name__=='__main__':unittest.main()
