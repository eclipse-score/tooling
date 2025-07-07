# CLI Helper

`cli_help.py` is a Bazel tool designed to discover and display Bazel targets that have been tagged with `cli_help` tags. This tool helps developers quickly find available command-line tools and their descriptions within a Bazel workspace.

## Features

- Discovers Bazel targets across the entire workspace and external repositories
- Filters targets by `cli_help` tags to identify command-line tools
- Provides a clean, formatted output with target names and descriptions
- Color-coded output for better readability

## Requirements

- Python 3.6+
- Bazel build system
- `os`, `re`, `subprocess`, and `xml.etree.ElementTree` (standard library modules)

## Installation

To use `cli_help.py`, simply clone this repository and ensure it's properly integrated into your Bazel workspace.

## Usage

The script can be run from the command line using Bazel:

```bash
bazel run //tooling/cli_helper/tool:help
```

### How it works

1. **Workspace Discovery**: The tool automatically detects the current Bazel workspace directory
2. **Target Discovery**: Queries Bazel for all targets across the workspace and external repositories
3. **Tag Filtering**: Filters targets that have `cli_help` tags
4. **Output Formatting**: Displays target names and their associated help descriptions

### Output Format

The tool outputs a formatted list of targets with their descriptions:

```
BAZEL TARGETS:

//tooling/cli_helper/tool:help                    CLI helper tool for discovering Bazel targets
//some/other/tool:binary                          Another tool description
```

## Tagging Your Targets

To make your Bazel targets discoverable by this tool, add a `cli_help` tag to your target definitions:

```python
py_binary(
    name = "my_tool",
    srcs = ["my_tool.py"],
    tags = ["cli_help=My tool description"],
    visibility = ["//visibility:public"],
)
```

### Tag Format

The `cli_help` tag should follow this format:
- `cli_help=<description>` where `<description>` is a human-readable description of what the tool does

## Bazel Integration

### Using CLI Helper in Your Project

To integrate the CLI helper into your Bazel-based project, you can use Bazel modules. 

Add the following to your `MODULE.bazel`:

```python
bazel_dep(name = "score_cli_helper", version = "0.1.0")
```
Add the following to you `BUILD`:

```python
load("@score_cli_helper//:cli_helper.bzl", "cli_helper")

cli_helper(
    name = "cli-help",
    visibility = ["//visibility:public"],
)
```

### Running the Tool

Once integrated, you can run the tool using:

```bash
bazel run //:help
```

## Examples

### Basic Usage

```bash
# Run from the workspace root
bazel run //:help
```

### Expected Output

```
BAZEL TARGETS:

//tooling/cli_helper/tool:help                    CLI helper tool for discovering Bazel targets
//tooling/cr_checker/tool:cr_checker              Copyright header checker and fixer
//tooling/format_checker/tool:format_checker      Code formatting verification tool
```

## Benefits

- **Discoverability**: Easily find available command-line tools in your workspace
- **Documentation**: Provides inline descriptions for tools
- **Workspace Navigation**: Helps developers understand what tools are available
- **Integration**: Seamlessly integrates with Bazel's build system
