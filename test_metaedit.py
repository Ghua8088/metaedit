import metaedit
import os
import shutil

# 1. Create a dummy EXE for testing (on Windows, we can just copy any exe)
# For this test, we'll just try to use the module itself if it was a file, 
# but we need a real PE file to test the Rust part.
# We can copy the current python executable to a temp file.
import sys

dummy_exe = "test_app.exe"
shutil.copy(sys.executable, dummy_exe)

print(f"Testing metaedit on {dummy_exe}...")

try:
    # Use the clean dictionary-based API
    metaedit.update(
        dummy_exe, 
        version="2.0.0.0", 
        CompanyName="Antigravity AI",
        FileDescription="Testing the new Rust MetaEdit library",
        ProductName="MetaEdit Test Suite"
    )
    print("Success!")
finally:
    if os.path.exists(dummy_exe):
        os.remove(dummy_exe)
