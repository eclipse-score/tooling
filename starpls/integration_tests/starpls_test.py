# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************
import os
import subprocess
import unittest
import time
import json

from runfiles import Runfiles


def format_lsp_request(msg):
    """Formats a JSON dictionary into an LSP message with headers."""

    json_msg = json.dumps(msg)
    content_length = len(json_msg.encode('utf-8')) 
    return f"Content-Length: {content_length}\r\n\r\n{json_msg}"

class StarplsIntegrationTest(unittest.TestCase):
    def setUp(self):
        """Runs before each test method."""
        self.runfiles = Runfiles.Create()

        binary_runfile_path = "_main/starpls/integration_tests/_starpls_binary_for_test_bin"
        self.starpls_binary_path = self.runfiles.Rlocation(binary_runfile_path)
        
        if not self.starpls_binary_path:
            self.fail(f"setUp failed: Could not find starpls binary via runfiles at {binary_runfile_path}")
        print(f"Found starpls binary for test: {self.starpls_binary_path}")

    def test_starpls_binary_downloaded_and_executable(self):
        """
        Tests that the setup_starpls macro successfully downloads the binary
        and makes it executable and that version command returns the expected output.
        """

        binary_real_path = self.starpls_binary_path 

        try:
            print(f"Attempting to run: {binary_real_path} version")
            result = subprocess.run(
                [binary_real_path, "version"],
                capture_output=True,
                text=True,
                check=True, 
                timeout=15
            )
            print("stdout:\n", result.stdout)
            print("stderr:\n", result.stderr)

            self.assertEqual(result.returncode, 0, "Running starpls version failed")
            self.assertIn("starpls", result.stdout, "Expected 'starpls' in version output")
        except FileNotFoundError:
            self.fail(f"Failed to execute binary: File not found at {binary_real_path}")
        except Exception as e:
            self.fail(f"Error: {e}")

    def test_starpls_server_initialize_simple(self):
        """
        Tests starting the server, sending initialize, and checking for any response.
        """

        binary_real_path = self.starpls_binary_path
        server_process = None

        try:
            print(f"Attempting to start server: {binary_real_path} server")
            server_process = subprocess.Popen(
                [binary_real_path, "server"],
                stdin=subprocess.PIPE,   
                stdout=subprocess.PIPE,  
                stderr=subprocess.PIPE,
                text=False
            )
            
            time.sleep(5) 

            if server_process.poll() is not None:
                 raise RuntimeError(f"Server process terminated prematurely with code {server_process.returncode}")

            print("Server process running. Sending init request.")

            # Send Init Request
            initialize_request = {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"processId": os.getpid(), "rootUri": None, "capabilities": {}}
            }

            lsp_request_bytes = format_lsp_request(initialize_request).encode('utf-8')
            print(f"Sending Request Bytes: {lsp_request_bytes}")
            server_process.stdin.write(lsp_request_bytes)
            server_process.stdin.flush()
            # IMPORTANT: Wait for server to process the request
            time.sleep(1)
            server_process.stdin.close()

            # Read Any Response
            print("Try and read any response.")
            stdout_output = server_process.stdout.read()

            print(f"Received stdout Bytes: {stdout_output}")
            self.assertGreater(len(stdout_output), 0, "Server did not produce any output on stdout after initialize request")
            print("Server produced output. Basic check passed.")

        except FileNotFoundError:
            self.fail(f"Failed to start server: Binary not found at {binary_real_path}")
        except Exception as e:
            self.fail(f"Error: {e}")
        finally:
            print("Test cleanup: Terminate server process.")
            if server_process:
                if server_process.poll() is None:
                    if not server_process.stdout.closed: 
                        server_process.stdout.close()
                    if not server_process.stderr.closed: 
                        server_process.stderr.close()
                    
                    #try and terminate before killing
                    server_process.terminate()
                    try:
                        server_process.wait(timeout=5)
                        print("Server process terminated.")
                    except subprocess.TimeoutExpired:
                        print("Server process termination timed out, killing...")
                        server_process.kill()
                        server_process.wait()
                        print("Server process killed.")
                else:
                    print(f"Server process already terminated with code: {server_process.returncode}")


if __name__ == "__main__":
    unittest.main()