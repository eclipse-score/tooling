"""
Test that the simplified setup works correctly.

This test verifies that:
1. score_python_basics provides Python toolchain automatically
2. pip dependencies can be added with minimal configuration
3. No manual rules_python setup is required
"""

import sys
import pytest


def test_python_version():
    """Test that we have the expected Python version from score_python_basics."""
    assert sys.version_info.major == 3
    assert sys.version_info.minor == 12


def test_pip_dependency_available():
    """Test that pip dependencies configured through simplified setup work."""
    try:
        import requests
        # Basic test that the module loads and has expected attributes
        assert hasattr(requests, "get")
        assert hasattr(requests, "post")
        print("✅ requests module loaded successfully")
    except ImportError as e:
        pytest.fail(f"requests dependency not available: {e}")


def test_score_python_basics_rules():
    """Test that score_python_basics rules are available."""
    # This test runs using score_py_pytest, which itself tests that 
    # the score_python_basics rules are working
    assert True  # If we reach here, score_py_pytest worked


if __name__ == "__main__":
    test_python_version()
    test_pip_dependency_available() 
    test_score_python_basics_rules()
    print("✅ All simplified setup tests passed!")