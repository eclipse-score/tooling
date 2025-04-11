import subprocess
import json
import pytest
import time
import os


@pytest.fixture(scope="module")
def start_starpls_server():
    """Fixture to start the StarPLS server as a subprocess."""
    # Start the StarPLS server
    server_process = subprocess.Popen(
        ["starpls", "--server"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    # Give the server a few seconds to initialize
    time.sleep(2)

    yield server_process  # This will be passed to the test

    # Cleanup: terminate the server after the test
    server_process.terminate()
    server_process.wait()


def send_lsp_request(method, params):
    """Helper function to send an LSP request to the running server."""
    # The LSP request message format
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    }

    # Connect to the StarPLS server and send the request
    server_process = subprocess.Popen(
        ["starpls", "--server"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    # Send the request
    server_process.stdin.write(json.dumps(request) + "\n")
    server_process.stdin.flush()

    # Read the response
    response = server_process.stdout.readline()
    return json.loads(response)


def test_formatting(start_starpls_server):
    """Test if the StarPLS server formats a .bzl file correctly."""
    # Path to the .bzl file to be tested
    test_file_path = os.path.join(os.path.dirname(__file__), "testfile.bzl")

    # Prepare the parameters for the formatting request
    params = {
        "textDocument": {
            "uri": f"file://{test_file_path}"  # Use the file URI scheme
        }
    }

    # Send the formatting request to the StarPLS server
    response = send_lsp_request("textDocument/formatting", params)

    # Extract the formatted text from the response
    formatted_text = response.get("result", "")
    print(formatted_text)

    # Expected formatted version of the test.bzl file
    expected_formatted_text = """def my_rule():
    load(":my_rule.bzl", "my_rule")

    my_rule(
        name="my_target",
        srcs=["some_file.txt"],
        visibility=["//visibility:public"],
    )
"""

    # Assert that the formatted text matches the expected formatted text
    assert formatted_text == expected_formatted_text, "The file was not formatted correctly"
