# Examples of score_python_basics simplified setup

This directory contains examples showing different ways to use the simplified setup pattern.

## Examples:

### 1. `simple-python/` - Basic Python project (no external dependencies)
Shows the minimal setup: just one line in MODULE.bazel

### 2. `with-pip-deps/` - Python project with pip dependencies  
Shows how to add pip dependencies with minimal configuration

### 3. `multi-environment/` - Advanced setup with multiple pip configurations
Shows how to handle main/test dependency separation

Each example includes:
- `MODULE.bazel` - Simplified setup configuration
- `BUILD` - Example targets using score_python_basics rules
- `*.py` - Sample Python code
- `requirements.txt` - pip dependencies (where applicable)

## Before vs After Comparison:

| Aspect | Old Pattern (❌) | New Pattern (✅) |
|--------|------------------|------------------|
| Lines in MODULE.bazel | ~20 lines | 1-6 lines |
| Manual rules_python setup | Required | Automatic |
| Python toolchain config | Manual | Automatic |
| Complexity | High | Low |
| Error-prone | Yes | No |