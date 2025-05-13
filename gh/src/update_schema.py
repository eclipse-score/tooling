#!/usr/bin/env python3
import os
import urllib.request

SCHEMA_URL = "https://docs.github.com/public/fpt/schema.docs.graphql"
DEST_PATH = os.path.join(os.path.dirname(__file__), "schema.graphql")

print(f"Downloading GitHub GraphQL schema from:\n  {SCHEMA_URL}")
with urllib.request.urlopen(SCHEMA_URL) as response:
    content = response.read()

with open(DEST_PATH, "wb") as f:
    f.write(content)
print(f"Saved schema to:\n  {DEST_PATH}")
