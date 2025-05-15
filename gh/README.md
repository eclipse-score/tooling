# GitHub GraphQL Client for Bazel (rules_ghql)

An **experimental** Bazel rule that generates a **typed, asynchronous Python client** for the GitHub GraphQL API using [`ariadne-codegen`](https://github.com/mirumee/ariadne-codegen).

> ⚠️ **Status**: This is currently a **Request for Comments (RFC)**. It's not a working implementation and doesn't even qualify as a proof of concept (POC). It simply does not work yet.

---

## ✨ What It Does

- Uses GitHub's public GraphQL schema
- Allows you to define your own `.graphql` queries
- Automatically generates:
  - An async Python client
  - Pydantic-based models for query results
- Exposes the generated code as a Bazel `py_library` for consumption

---

## 🛠️ Usage (Bazel)

See the [example](test/scenario_1) directory for a complete usage scenario.

---

## 🧪 Why This Exists

Using GraphQL in Python without tooling is painful.  
You end up managing transport logic, schemas, type casting, and boilerplate — all without any type safety.

This project aims to eliminate that by generating a **typed, async GraphQL client** and matching **Pydantic models**, fully integrated with Bazel for trivial usage.

---

## 🚧 Limitations

- Only supports GitHub’s schema (**intentionally**)
- Minimal configurability (**intentionally**)
- Only supports Python 3.12 (**intentionally**)

---

## 💬 Feedback Welcome

This library is under active design iteration.  
If you have suggestions, questions, or ideas — feel free to get in touch.
