from __future__ import annotations

from collections.abc import Callable
from typing import Any
from pathlib import Path
from typing import Literal
import os

import pytest

# Type aliases for better readability
TestFunction = Callable[..., Any]
Decorator = Callable[[TestFunction], TestFunction]


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
    test_type: Literal[
        "fault-injection", "interface-test", "requirements-based", "resource-usage"
    ],
    derivation_technique: Literal[
        "requirements-analysis",
        "design-analysis",
        "boundary-values",
        "equivalence-classes",
        "fuzz-testing",
        "error-guessing",
        "explorative-testing",
    ],
) -> Decorator:
    """
    Decorator to add user properties, file and lineNr to testcases in the XML output
    """
    # Early error handling
    if partially_verifies is None and fully_verifies is None:
        raise ValueError(
            "Either 'partially_verifies' or 'fully_verifies' must be provided."
        )

    #          ╭──────────────────────────────────────╮
    #          │  HINT. This is currently commented   │
    #          │ out to not restrict usage a lot but  │
    #          │   will be commented back in in the   │
    #          │                future                │
    #          ╰──────────────────────────────────────╯

    # if not test_type:
    #     raise ValueError("'test_type' is required and cannot be empty.")
    #
    # if not derivation_technique:
    #     raise ValueError("'derivation_technique' is required and cannot be empty.")
    #

    def decorator(func: TestFunction) -> TestFunction:
        # Clean properties (skip None)
        properties = {
            "PartiallyVerifies": ", ".join(partially_verifies)
            if partially_verifies
            else "",
            "FullyVerifies": ", ".join(fully_verifies) if fully_verifies else "",
            "TestType": test_type,
            "DerivationTechnique": derivation_technique,
        }
        # Ensure a 'description' is there inside the Docstring
        if not func.__doc__ or not func.__doc__.strip():
            raise ValueError(
                f"{func.__name__} does not have a description."
                + "Descriptions (in docstrings) are mandatory."
            )
        # NOTE: This might come back to bite us in some weird edgecase, though I have not thought of one so far
        # Remove keys with 'falsey' values
        cleaned_properties = {k: v for k, v in properties.items() if v}
        return pytest.mark.test_properties(cleaned_properties)(func)

    return decorator


def pytest_runtest_makereport(item: pytest.Item, call: pytest.CallInfo[None]) -> None:
    """Attach file and line info to the report for use in junitxml output."""
    if call.when != "call":
        return
    # Since our decorator 'add_test_properties' will create a 'test_properties' marker
    # This function then searches for the nearest dictionary attached to an item with 
    # that marker and parses this into properties.

    # In short: 
    #   => This function adds the properties specified via the decorator to the item so 
    #      they can be written to the XML output in the end
    # Note: This does NOT add 'line' and 'file' to the testcase.
    marker = item.get_closest_marker("test_properties")
    if marker and isinstance(marker.args[0], dict):
        for k, v in marker.args[0].items():
            item.user_properties.append((k, str(v)))


@pytest.fixture(autouse=True)
def add_file_and_line_attr(
    record_xml_attribute: Callable[[str, str], None], request: pytest.FixtureRequest
) -> None:
    node = request.node
    # print(node.__dir__())
    # node.fspath gives the path to the test file
    # node.file_path
    git_root = find_git_root(node.fspath)
    file_path = (
        Path(os.path.realpath(node.fspath)).relative_to(git_root)
    )
    # file_path = Path(node.fspath).relative_to(git_root)
    # node.location is a tuple (filename, lineno, testname)
    line_number = node.location[1]

    record_xml_attribute("file", file_path)
    record_xml_attribute("line", str(line_number))
