from __future__ import annotations

import functools
from collections.abc import Callable
from typing import Any
from pathlib import Path
from typing import Literal
import os

import pytest

def find_git_root(start_path: str | Path = "") -> Path | None:
    """Find the git root directory starting from the given path or __file__."""
    if start_path == "":
        start_path = __file__

    git_root = Path(start_path).resolve()
    esbonio_search = False
    while not (git_root / ".git").exists():
        git_root = git_root.parent
        if git_root == Path("/"):
            # fallback to cwd when building with python -m sphinx docs _build -T
            if esbonio_search:
                return None
            git_root = Path.cwd().resolve()
            esbonio_search = True
    return git_root

def add_test_properties(
    *,
    partially_verifies: list[str] | None = None,
    fully_verifies: list[str] | None = None,
    test_type: Literal["fault-injection", "interface-test", "requirements-based", "resource-usage"],
    derivation_technique: Literal["requirements-analysis", "design-analysis", "boundary-values", "equivalence-classes", "fuzz-testing", "error-guessing", "explorative-testing"],
    **extra_properties: Any,
) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
    """
    Decorator to add user properties, file and lineNr to testcases in the XML output
    """
    # Early error handling
    if partially_verifies is None and fully_verifies is None:
        raise ValueError(
            "Either 'partially_verifies' or 'fully_verifies' must be provided."
        )    

    # if not test_type:
    #     raise ValueError("'test_type' is required and cannot be empty.")
    #
    # if not derivation_technique:
    #     raise ValueError("'derivation_technique' is required and cannot be empty.")
    #

    def decorator(func: Callable[..., Any]) -> Callable[..., Any]:
        # Clean properties (skip None)
        properties = {
            "PartiallyVerifies": ", ".join(partially_verifies) if partially_verifies else None,
            "FullyVerifies": ", ".join(fully_verifies) if fully_verifies else None,
            "TestType": test_type,
            "DerivationTechnique": derivation_technique,
            **{k: str(v) for k, v in extra_properties.items()},
        }
        # Ensure a 'description' is there inside the Docstring
        #print(f" THIS IS DECORATOR FOR FUNC: {func.__name__}")
        if not func.__doc__ or not func.__doc__.strip():
            raise ValueError(f"{func.__name__} does not have a description. Descriptions (in docstrings) are mandatory.")
        # Remove keys with None values
        cleaned_properties = {k: v for k, v in properties.items() if v is not None}
        return pytest.mark.test_properties(cleaned_properties)(func)

    return decorator



def pytest_runtest_makereport(item: pytest.Item, call: pytest.CallInfo[None]) -> None:
    """Attach file and line info to the report for use in junitxml output."""
    if call.when != "call":
        return

    # Also support test_properties marker
    marker = item.get_closest_marker("test_properties")
    if marker and isinstance(marker.args[0], dict):
        for k, v in marker.args[0].items():
            item.user_properties.append((k, str(v)))


@pytest.fixture(autouse=True)
def add_file_and_line_attr(record_xml_attribute, request):
    node = request.node
    #print(node.__dir__())
    # node.fspath gives the path to the test file
    #node.file_path
    git_root = find_git_root(node.fspath)
    file_path = str(os.path.realpath(node.fspath)).replace(str(git_root), "").removeprefix("/")
    # node.location is a tuple (filename, lineno, testname)
    line_number = node.location[1]

    record_xml_attribute("file", file_path)
    record_xml_attribute("line", str(line_number))
