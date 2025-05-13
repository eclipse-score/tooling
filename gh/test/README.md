# GitHub GraphQL Client Generator - Integration Tests

This directory contains integration test scenarios for the GitHub GraphQL client generator.

## Scenario 1: Basic User Query

Tests a simple GraphQL query to fetch a GitHub user profile using the generated client.

### Files:
- `get_user.graphql`: The GraphQL query definition
- `main.py`: A simple script that uses the generated client

### How to run:

```bash
bazel run //test/scenario_1:main
```

> Note: You'll need to replace "your_token" in main.py with a valid GitHub token for the test to succeed.