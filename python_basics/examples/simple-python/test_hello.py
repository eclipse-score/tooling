"""Tests for hello.py"""

import hello

def test_hello():
    """Test that hello runs without error."""
    # If this test runs, it means:
    # 1. Python 3.12 toolchain from score_python_basics works
    # 2. score_py_pytest rule works  
    # 3. No manual Python setup was required!
    result = hello.main()
    assert result == 0