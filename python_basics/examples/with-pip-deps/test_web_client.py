"""Tests for web_client.py"""

import pytest
from unittest.mock import patch, Mock
import web_client

def test_imports():
    """Test that pip dependencies are available."""
    import requests
    import click
    
    # If these imports work, pip dependencies are properly configured
    assert hasattr(requests, "get")
    assert hasattr(click, "command")

@patch('requests.get')
def test_web_client_success(mock_get):
    """Test web client with mocked successful response."""
    # Mock successful response
    mock_response = Mock()
    mock_response.status_code = 200
    mock_response.text = "test content"
    mock_get.return_value = mock_response
    
    # Test the click command (this tests both click and requests integration)
    from click.testing import CliRunner
    runner = CliRunner()
    result = runner.invoke(web_client.main, ["--url", "http://example.com"])
    
    assert result.exit_code == 0
    assert "Status: 200" in result.output
    assert "Content length: 12" in result.output