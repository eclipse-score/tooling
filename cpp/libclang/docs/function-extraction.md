<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# libclang function extraction and control flow

This document records the current contract for extracting C++ callable
definitions and their control flow. It describes the assumptions made by
`FunctionVisitor`, rather than the complete libclang API. For the overall
per-source-file analysis flow, see [libclang C++ source analysis](ast-traversal.md).

## Function extraction

A callable becomes a `FunctionDef` only when all of the following hold:

1. it is located in the current translation unit's main file;
2. a `FunctionId` can be derived;
3. its cursor kind maps to a supported `FunctionKind`;
4. it owns a direct `CompoundStmt` body.

A declaration-only function therefore does not produce a `FunctionDef`. This
is important because an empty function body and the absence of any function
body have different meanings.

The function visitor first derives the function identity. Subsequent diagnostic
messages can then include a qualified name rather than only an unqualified
cursor spelling.

### Function body processing

A function cursor's direct children include parameter declarations and, for a
normal definition, a `CompoundStmt`. The visitor locates that `CompoundStmt`
and passes it to `process_scope`.

`process_scope` processes direct statements in source order:

- `IfStmt` is represented as one `BodyItem::Branch` with ordered cases. A case
  has an optional `GuardExpression`, a body, and a source location; a final
  `else` case has no guard. When libclang exposes supported unary and binary
  cursor layouts, a guard preserves `&&`, `||`, and `!` as a tree so condition
  calls retain their C++ short-circuit execution prerequisites. Other
  expressions remain source-backed opaque leaves. An `else if` chain is
  flattened into additional cases. Unsupported `IfStmt` child layouts use a
  conservative fallback that preserves reachable calls without inventing an
  unreliable branch shape;
- `ForStmt`, `WhileStmt`, and `DoStmt` are represented as `BodyItem::Loop`
  with a typed `LoopKind`;
- `CallExpr` is represented as `BodyItem::Call` when its target has a different
  owner;
- other non-control-flow statements are searched recursively for calls.

Calls nested within another call are emitted before their enclosing call. This
is structural nesting order only: the visitor does not claim an evaluation
order between sibling C++ call arguments.

## Callable scope and identity

A `FunctionId` contains a callable name and a structured `Scope`.

```text
FunctionId = Scope + function name
```

`Scope` distinguishes:

- `Global` for global functions;
- `Namespace(path)` for namespace functions;
- `Type { namespace, type_path }` for member functions.

Keeping the scope structured prevents a namespace and a type with the same
spelling from being treated as the same owner. It also preserves nested types.
For example:

```cpp
namespace app {
class Outer {
public:
    class Inner {
    public:
        void run();
    };
};
}
```

is represented conceptually as:

```text
Scope::Type {
    namespace: ["app"],
    type_path: ["Outer", "Inner"],
}
FunctionId: app::Outer::Inner::run
```

The scope adapter extracts this information from semantic parents. It keeps
namespace paths distinct from type paths because they have different C++
semantics.

### Overloads

The current `FunctionId` does not include parameter types. Consequently,
overloads in the same scope currently share an identity for call-resolution
purposes. Do not use `FunctionId` as a single-value de-duplication key until a
signature is added to the model.

## Supported cursor kinds

The top-level visitor currently dispatches these cursor kinds to
`FunctionVisitor`:

| libclang cursor kind | Function kind |
| --- | --- |
| `FunctionDecl` | `Free` |
| `Method` | `Method` or `StaticMethod` |

C++ member operator overloads such as `operator+` and `operator[]` are normally
reported as `Method`; the current model does not use a distinct operator-method
kind.

`FunctionVisitor` has internal kind mappings for `Constructor`, `Destructor`,
and `ConversionFunction`, but the top-level visitor currently logs and ignores
those cursor kinds. Therefore they do not currently produce `FunctionDef`
entries. A conversion operator such as `operator bool()` is a
`ConversionFunction` and is distinct from a normal operator overload.

Function templates are not currently part of function extraction. Class visitor
handling of method templates is independent from extraction of function bodies
and call relationships.
