#!/usr/bin/env python3
"""
Test that Python toolchain is available and working.

This test verifies that score_python_basics automatically provides
a working Python 3.12 toolchain without requiring manual setup.
"""

import sys

def main():
    print(f"Python version: {sys.version}")
    print(f"Python executable: {sys.executable}")
    
    # Verify we're using Python 3.12 as configured by score_python_basics
    assert sys.version_info.major == 3
    assert sys.version_info.minor == 12
    
    print("✅ Python toolchain test passed!")
    return 0

if __name__ == "__main__":
    sys.exit(main())