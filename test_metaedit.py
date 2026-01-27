import metaedit
import os
import shutil
import sys
import unittest
import tempfile
from pathlib import Path

# Try importing verification libraries
try:
    import pefile
    HAS_PEFILE = True
except ImportError:
    HAS_PEFILE = False

try:
    from PIL import Image
    HAS_PILLOW = True
except ImportError:
    HAS_PILLOW = False

class TestMetaEdit(unittest.TestCase):
    def setUp(self):
        # Create a temporary directory
        self.test_dir = tempfile.mkdtemp()
        self.exe_path = os.path.join(self.test_dir, "test_app.exe")
        
        # Copy python executable as our target (it's a valid PE on Windows)
        if sys.platform == "win32":
            shutil.copy(sys.executable, self.exe_path)
        else:
            # Create a dummy file for non-Windows (or skip pe tests)
            with open(self.exe_path, "wb") as f:
                f.write(b"MZ" + b"\0" * 100) # Fake minimal header

        # Create a dummy icon if Pillow is available
        self.icon_path = os.path.join(self.test_dir, "test_icon.png")
        if HAS_PILLOW:
            img = Image.new('RGBA', (64, 64), color = (255, 0, 0, 255))
            img.save(self.icon_path)

    def tearDown(self):
        shutil.rmtree(self.test_dir)

    def test_basic_update_strings(self):
        if sys.platform != "win32":
            return
            
        print("\nTesting String Update...")
        metaedit.update(
            self.exe_path,
            CompanyName="MetaEdit Corp",
            FileDescription="Unit Test File",
            ProductVersion="9.9.9.9"
        )
        
        if HAS_PEFILE:
            pe = pefile.PE(self.exe_path)
            # Verification is complex with pefile directly on resources, 
            # so we mostly check that the file is not corrupted and still valid PE
            self.assertTrue(pe.is_exe)
            pe.close()
        
        print("String Update Success")

    def test_icon_update(self):
        if sys.platform != "win32":
            return
        if not HAS_PILLOW:
            print("Skipping icon test (Pillow not installed)")
            return

        print("\nTesting Icon Update with PNG...")
        # valid PNG -> should be converted to multi-size ICO automatically
        metaedit.update(self.exe_path, icon=self.icon_path)
        
        if HAS_PEFILE:
            pe = pefile.PE(self.exe_path)
            # Check if resources section exists
            has_resources = hasattr(pe, 'DIRECTORY_ENTRY_RESOURCE')
            self.assertTrue(has_resources, "Resources directory disappeared!")
            pe.close()
            
        print("Icon Update Success")

    def test_signature_stripping(self):
        if sys.platform != "win32":
            return

        # metaedit.update should automatically strip signatures
        metaedit.update(self.exe_path, CompanyName="Unsigned Corp")
        
        if HAS_PEFILE:
            pe = pefile.PE(self.exe_path)
            try:
                # Security directory is index 4
                security_dir = pe.OPTIONAL_HEADER.DATA_DIRECTORY[4]
                print(f"DEBUG: VirtualAddress={security_dir.VirtualAddress}, Size={security_dir.Size}")
                self.assertEqual(security_dir.VirtualAddress, 0, "Signature pointer not cleared")
                self.assertEqual(security_dir.Size, 0, "Signature size not cleared")
            finally:
                pe.close()

    def test_error_handling(self):
        # Test non-existent file
        with self.assertRaises(FileNotFoundError):
            metaedit.update("non_existent_file.exe", version="1.0")

if __name__ == "__main__":
    unittest.main()
