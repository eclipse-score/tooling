<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Class Design Implementation Specification

## Purpose

This validator enforces consistency between the class model in the unit design
and the class model extracted from the implementation.

It shall make sure that every class and free function, as well as every class
member, method, enum literal, type alias, and relationship modeled in the unit
design, exists in the implementation and is compatible with it.

Implementation-only entities and members are allowed. The unit design is the
validated contract.

## What is Validated

The validator compares unit design class diagrams with implementation class
diagrams produced by the C++ parser. It receives two indexed class-diagram
inputs:

| Input | Source | Meaning |
|---|---|---|
| `design_classes` | `unit_design` class diagram FlatBuffers | Required class model from the unit design |
| `implementation_classes` | C++ parser class diagram FlatBuffers from `unit` implementation targets | Observed class model from implementation code |

### Entity Presence and Type Consistency

Every entity in the unit design class diagram must have a corresponding entity
in the implementation class diagram. Matching entities must have the same entity
type.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationEntityConsistency`)*

This item covers entity matching and entity type only. The consistency of data
contained by a matched entity is covered by the following type alias, variable,
method, enum literal, relationship, and template parameter items.

If the same class ID is declared more than once across the unit design's input
class diagrams (e.g. an overview and a detail `.puml` file), the declarations
merge when at most one of them has a body (members, type aliases or enum
literals); the relationships of all declarations are united. Two declarations
with a body are reported as a failure naming both source locations, independent
of entity type consistency. A different entity type or stereotype across the
declarations is a failure as well.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationDeclarationMerge`)*

```text
' PlantUML class diagram
package vehicle {
  class "Transport"
}
```

```cpp
// implementation C++ code
namespace vehicle {
  class Transport {}
}
```

### Template Parameter Consistency

Template parameters modeled on unit design entities and methods must match the
template parameters on the corresponding implementation entities and methods.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationTemplateParameterConsistency`)*

For classes, methods, and free functions:

- A nonempty design declaration parameter list may omit trailing implementation
  parameters; defaults are not required or inspected. Extra design entries and
  skipped intermediate positions are not allowed.
- Explicit parameters match by position, kind, and pack marker; non-type types
  follow **Type Normalization**. Template-template lists follow these rules
  recursively.
- Omitted design names are unconstrained; explicit names must match. An absent
  (`None`) list matches only `None`, and an explicit empty list only an explicit
  empty list; neither is a wildcard.
- Method/free-function template declarations are checked after selection under
  **Overload Group Matching**.

**Specialization exception:** implementation partial and explicit full
specializations are matched by fully qualified base name only. Their class
template parameters and specialization arguments are not validated against the
design. This exception does not apply to method template parameters. A debug
diagnostic explains when the class template-parameter check is skipped.

When matching partial or explicit full specializations exist, a matching primary
template is included in implementation consistency checks only if its template
parameters match the design. This allows rejection-only primary templates to be
excluded from a specialization's interface contract; the validator does not
determine whether a primary template serves that purpose. Without matching
specializations, the primary template is checked normally, including
template-parameter mismatches. Ordinary non-template classes are not subject to
this primary-template filtering rule.

**Notation:** type, non-type, and template-template parameters support optional
names and packs: `typename... Ts`, `int... Ns`, and
`template<typename> class... Cs`. `class` and `typename` are interchangeable for
type parameters and template-template parameter tails. Packs precede explicit
names (`typename Ts...` is unsupported). Legacy `T` and `Ts...` denote type
parameters; unnamed non-type parameters require built-in or unambiguous compound
types such as `int`, `std::size_t`, or `int*`.

**Scope:** this is a modeling subset, not C++ declaration validation. Defaults,
constraints, and complex declarators are unsupported; type lookup and semantic
validity remain the C++ compiler's responsibility. An unsupported template list
in a quoted name produces no template parameter metadata.

```text
' PlantUML class diagram
class Repository<T> {
  + FindById<Key>(id: Key) : T
}
```

```cpp
// implementation C++ code
template <typename T>
class Repository {
  template <typename Key>
  T FindById(Key id);
}
```

### Specialization Interface Matching

A quoted class name may describe a partial or explicit full specialization. Its trailing
template arguments are removed from the design entity ID; an alias does not
change that ID.

The declared interface is checked against **every implementation partial or explicit full
specialization with the same fully qualified base name**, without comparing its
class template parameters or specialization arguments. On the implementation side,
that base name is the qualified ID after removing only its final template-argument
list: `Outer<Policy>::Box<int>` matches `Outer<Policy>::Box`, not `Outer::Box`.
The raw design `<…>` text is not retained for argument comparison, so a typo such as
`<ReturnTyp(ArgTypes...), Capacity, Alignment>` or an unrelated list such as
`<int, X>` does not cause a specialization-argument mismatch. A debug diagnostic
identifies each skipped class template-parameter check and explains this
base-name-only matching limitation.

The design therefore defines one shared contract,
not separate contracts per specialization. Multiple patterns resolve to the
same ID and follow the declaration-merge rules above; conflicting declarations
fail before implementation comparison.

For example, a design entity `Box` is checked against both `Box<int>` and
`Box<double>` explicit full specializations. A design label `Box<int>` still
resolves to the base ID `Box`; it does not select only the `int` specialization.
Each specialization must satisfy the shared member and method contract, including
method template parameters. No template-argument substitution is performed when
comparing member or method types.

Explicit instantiation definitions (`template class Box<double>;`) and declarations
(`extern template class Box<float>;`) reuse an existing template implementation;
they do not define independent interfaces. The C++ extractor excludes these
declarations and their member traversal from the implementation class model.
Their concrete type names remain available when referenced by other declarations.
Implicit uses likewise do not introduce independent interface contracts.

For template-associated class cursors, the extractor distinguishes `template <>`
specialization headers from explicit instantiation headers using declaration tokens,
ignoring comments. An unrecognized header (for example an unsupported macro
expansion) produces a warning and is not collected; it is never silently classified
as a non-template class or an explicit specialization. Such warnings indicate an
incomplete extracted model and must be investigated before relying on validation.

```text
' One design entity; the interface is checked against each matching specialization.
namespace amp {
  class "move_only_function<ReturnType(ArgTypes...), Capacity, Alignment>" as move_only_function {
    + empty() noexcept : bool
  }
}
```

### Type Alias Consistency

Every type alias modeled in the unit design entity must exist in the matching
implementation entity. The alias name and normalized original type must match.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationTypeAliasConsistency`)*

```text
' PlantUML class diagram
class "Transport" {
  using Payload = std::uint8_t
}
```

```cpp
// implementation C++ code
class Transport {
public:
  using Payload = std::uint8_t;
};
```

### Variable Consistency

Every variable modeled in the unit design entity must exist in the matching
implementation entity. The variable name, normalized data type, visibility, and
static flag must match.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationVariableConsistency`)*

```text
' PlantUML class diagram
class "Transport" {
  - buffer: std::uint8_t*
  {static} - instance_count: uint32_t
}
```

```cpp
// implementation C++ code
class Transport {
private:
  std::uint8_t* buffer;
  static std::uint32_t instance_count;
};
```

### Method Consistency

Every method modeled in the unit design entity must exist in the matching
implementation entity. The method name, visibility, parameters, and method
modifiers must match, and the normalized return type must also match (this
applies to constructors and destructors too, which typically have no return
type on either side). Method modifiers include `const`, `static`, `virtual`,
`abstract`, `override`, constructor, destructor, and `noexcept`. A nonempty
design parameter list constrains a positional prefix: names, normalized types
(see **Type Normalization**), and variadic/pack-expansion (`...`) markers must
match. Only trailing implementation parameters may be omitted, without requiring
or inspecting defaults; extra design entries, skipped positions, and reordering
are not allowed. An empty list strictly requires zero implementation parameters.
Top-level `const` on a parameter is ignored; see
[Top-level `const` on parameters](#top-level-const-on-parameters).
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationMethodConsistency`)*

For example, `f(id: int)` may match `f(int id, bool enabled)`, but `f()` cannot.

`static`, `abstract`, `virtual`, and `override` may each be written either as a
brace qualifier (e.g. `{virtual}`) or as a stereotype (e.g. `<<virtual>>`); both
forms are equivalent. `override` may additionally be written as a bare trailing
keyword after the method signature, mirroring C++ syntax, e.g. `+ Run() override`.
`noexcept` is only written as a bare trailing keyword, e.g. `+ Run() noexcept`.
An `abstract` or `override` method is implicitly `virtual` (matching C++
semantics), so the effective modifier set includes `virtual` even if it was not
written explicitly. The visibility classifier (`+`/`-`/`#`/`~`) is always
written first, before any brace/stereotype modifier, e.g. `+ {virtual} Run()`.

Method `const` qualification may be written as a bare trailing qualifier, e.g.
`+ Empty() const : bool`, or as a stereotype before or after the return type,
e.g. `+ Empty() <<const>> : bool` or `+ Empty() : bool <<const>>`.
These forms are equivalent and can be combined with `noexcept`, e.g.
`+ Empty() const noexcept : bool`. Method qualification is independent of
`const` in a parameter or return type: `+ Value() : const int&` alone does not
declare a const-qualified method.

The trailing-parameter omission rule applies equally to const-qualified and
non-const methods; see the examples under **Overload Group Matching**.

```text
' PlantUML class diagram
class "Base" {
  + {virtual} Run(): void
  + {virtual} Stop(): void
}
class "Transport" {
  + <<override>> Run(): void
  + Stop() override : void
  + Tick() noexcept : void
}
Base <|-- Transport
```

```cpp
// implementation C++ code
class Base {
public:
  virtual void Run();
  virtual void Stop();
};

class Transport : public Base {
public:
  void Run() override;
  void Stop() override;
  void Tick() noexcept;
};
```

C-style variadic parameters (e.g. `void Log(const char* fmt, ...)`) are not
currently supported by the PlantUML parser and are not validated: a design
parameter is always treated as non-variadic.

Method lookup follows **Overload Group Matching**, including method `const`
qualification when forming candidates. Thus `Check()` and `Check() const` are
distinct overloads, even when their return types are identical. A `const` mismatch is
rejected in both directions: the design is const-qualified but the implementation
is not, or the implementation is const-qualified but the design is not. For a
unique same-named candidate, this produces a method-modifier mismatch showing
`[Const]` versus `[]`. With no compatible candidate, one same-name implementation
yields field-level diagnostics (including parameter count errors); otherwise, a
missing-signature diagnostic is reported.

```text
' PlantUML class diagram
class "Transport" {
  + Dispatch(mode: std::uint8_t, payload: vehicle::Payload): bool
}
```

```cpp
// implementation C++ code
bool Transport::Dispatch(std::uint8_t mode, vehicle::Payload payload)
```

### Enum Literal Consistency

Every enum literal modeled in the unit design enum must exist in the matching
implementation enum. The full literal data must match.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationEnumLiteralConsistency`)*

```text
' PlantUML class diagram
enum "Mode" {
  Startup
  Shutdown
}
```

```cpp
// implementation C++ code
enum Mode {
  Startup,
  Shutdown,
};
```

### Relationship Consistency

Every relationship modeled on a unit design entity must exist on the matching
implementation entity. Source, target, and relationship type must match.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationRelationshipConsistency`)*

```text
' PlantUML class diagram
struct Payload {}
class Vehicle {
  + Load(payload : Payload)
}
Vehicle ..> Payload
```

```cpp
// implementation C++ code
struct Payload {};
class Vehicle {
public:
  void Load(Payload payload);
};
```

### Free Function Consistency

Every free function (at global or namespace scope) modeled in the unit design
must exist in the implementation. Functions are grouped by fully
namespace-qualified name and follow **Method Consistency** parameter and
no-candidate diagnostic rules and **Overload Group Matching**. Return types and
template declarations must also match; class-only visibility/modifiers do not apply.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationFreeFunctionConsistency`)*

```text
' PlantUML class diagram
namespace vehicle {
  entity VehicleFuncs <<function>> {
    + Dispatch(mode: std::uint8_t): void
  }
}
```

```cpp
// implementation C++ code
namespace vehicle {
void Dispatch(std::uint8_t mode);
}
```

### Overload Group Matching

For each same-name method group within a matched class, or fully qualified free
function group:

1. Form candidates using positional normalized parameter types, counts, and
  variadic/pack flags under **Method Consistency**. For methods, `const`
  qualification must also be equal.
2. Require a **unique injective assignment** covering every design declaration:
   each selects a different implementation; implementation-only overloads may
   remain unused.
3. Check parameter names, return types, template declarations, and applicable
  visibility/remaining modifiers after selection, never to disambiguate candidates.

There is no exact-first preference, implicit conversion, template deduction, or
substitution.

| Design declarations | Implementation overloads | Result |
|---|---|---|
| `f(int)` | `f(int, int)` | Unique trailing omission, no defaults required |
| `f(int)` | `f(int)`, `f(int, int)` | Ambiguous, even with an exact signature |
| `f(int)`, `f(int, int)` | `f(int)`, `f(int, int)` | Unique group-wide assignment |
| `f(int)`, `f(int, int)` | `f(int, int)` | Collision: no injective assignment |
| `f(int)`, `f(int, int)` | `f(int)`, `f(int, int)`, `f(int, int, float)` | Still ambiguous |
| `f()` | `f()`, `f(int)` | Unique zero-parameter candidate |
| `f()` | `f(int)` | Parameter-count mismatch, not omission |
| `f(int)` | `f(double, int)` | Type mismatch, no conversion |
| `f(int id) const` | `f(int id, bool enabled) const` | Unique trailing omission also applies to const-qualified methods |
| `f() const` | `f(int id, bool enabled) const` | Parameter-count mismatch: an empty design list requires zero implementation parameters |
| `f(bool enabled) const` | `f(int id, bool enabled) const` | Type mismatch: the design must specify a positional prefix, not skip the first parameter |
| `f(int id)` | `f(int id, bool enabled) const` | Method const-qualification mismatch: non-const and const methods do not match |
| `template<typename T> f(int)` | `template<typename T> f(int)` | Unique assignment; template declaration matches |
| `template<typename T> f(int)` | `template<typename T, typename U> f(int)` | Unique assignment; trailing template parameter omission is allowed, no defaults required |
| `template<typename T> f(int)` | `template<int N> f(int)` | Unique assignment, then template parameter kind mismatch |
| `template<typename T> f(int)` | `f(int)` | Unique assignment, then template declaration mismatch |
| `template<typename T> f(int)` | `f(int)`, `template<typename T> f(int)` | Ambiguous; the matching template declaration does not disambiguate candidates |
| `template<typename T> f(T)` | `template<typename U> f(U)` | Parameter type mismatch; no template parameter renaming or substitution |
| `template<int N> C` | `template<int N, typename T> C` | Unique trailing omission also applies to non-type parameters |
| `template<int N> C` | `template<typename T, int N> C` | Kind mismatch: the design must specify a positional prefix, not skip the first parameter |
| `template<int N> C` | `template<long N> C` | Non-type parameter type mismatch; no conversion |
| `template<int N> C` | `template<int M> C` | Explicit non-type parameter names must match |
| `template<std::size_t N> C` | `template<size_t N> C` | Non-type parameter types match after type normalization |
| `template<template<typename> class C> Owner` | `template<template<typename> class C, typename T> Owner` | Unique trailing omission also applies to template-template parameters |
| `template<template<typename> class C> Owner` | `template<template<typename U> class C> Owner` | Inner template-template lists also allow trailing omission |
| `template<template<typename T> class C> Owner` | `template<template<int N> class C> Owner` | Nested template parameter kind mismatch |
| `template<template<typename> class C> Owner` | `template<typename C> Owner` | Template-template and type parameters do not match |
| `Map<K, V>` | `Map<K, V, Compare>` | Compatible type prefix only with known template metadata on both sides; otherwise type mismatch |

The `template<...> f(...)` notation above describes declaration metadata, not
template call arguments; return types and parameter names are omitted for brevity.
A unique assignment is only the selection result: subsequent template or other
field checks may still fail validation.

Ambiguity and collision diagnostics include the design and implementation
declarations, recorded parameter spellings/flags, and source locations.
No-candidate diagnostics follow **Method Consistency**.

Method/free-function grouping ignores source location and deduplicates exact
recorded non-location fields; distinct conflicting records remain. This is not
full C++ redeclaration resolution. Member `const` qualification distinguishes
candidates; ref qualification is not represented or checked.

#### Effect of Partial and Explicit Full Specializations

Class specialization matching and overload matching operate at different levels.
Under **Specialization Interface Matching**, every implementation partial or
explicit full class specialization with the same fully qualified base name is
checked against the shared design contract. Overload assignment is performed
**separately within each matched implementation class**, not across a combined
pool of methods from all specializations. Two specializations each providing
`f(int)` therefore do not, by themselves, create overload ambiguity.

| Design contract | Implementation specializations | Result |
|---|---|---|
| `Box<T>` with `f(int)` | Partial specialization `Box<T*>` and full specialization `Box<int>`, each with only `f(int)` in that method group | Unique method assignment in each specialization; both are checked |
| `Box<T>` with `f(int)` | `Box<int>` with `f(int)`; `Box<double>` with `f(int)` and `f(int, double)` | Unique in `Box<int>`, ambiguous in `Box<double>`; validation fails |
| Design label `Box<int>` with `f(int)` | `Box<int>` with `f(int)`; `Box<double>` without `f` | Both specializations are checked by base name; missing method in `Box<double>` fails validation |

In these examples:

- `Box<T>` declares a class template, but `f(int)` requires an `int` parameter,
  not `T`. Even a design label `Box<int>` matches specializations by base name
  `Box`, not by the argument `int`.
- Class specializations skip class template-parameter and specialization-argument
  checks only; their members and method template declarations are still checked.
  A matching primary template is also checked if its template parameters match
  the design (see **Template Parameter Consistency**).
- Methods and free functions follow the overload rules above, with no preference
  for explicit full function specializations. C++ function templates cannot be
  partially specialized; additional function templates are overloads.

### Type Normalization

Before comparing types, the validator applies limited normalization.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationTypeNormalization`)*

| Input form | Normalized form |
|---|---|
| `std::uint8_t` | `uint8_t` |
| `uint8_t *` | `uint8_t*` |
| `Payload &` | `Payload&` |
| `uint8_t* const` | `uint8_t*const` |
| `vehicle.Engine` | `vehicle::Engine` |

The last row accounts for a difference between the class diagram and
implementation parser UIDs: dots used as namespace separators are converted to
`::` before comparison. This applies to type names, class/entity IDs, and
relationship source/target identifiers alike. For full-string comparisons,
cv-qualifiers such as `const`
and `volatile`, and the pointer/reference decorators `*`/`&`, are preserved
(only the spacing around them is normalized). The `uint8_t* const` row
accounts for the implementation parser printing a const pointer as
`uint8_t *const`, which is compared against the design form `uint8_t* const`.
Known template metadata preserves the same outer cv/pointer/reference structure.

#### Top-level `const` on parameters

Top-level `const` on a by-value parameter is not part of the C++ signature, so
it is ignored when comparing method and free function parameter types. Low-level
`const` is part of the type and must match.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationTopLevelConstParameters`)*

| Design parameter type | Implementation parameter type | Result |
|---|---|---|
| `const int` | `int` | match |
| `int* const` | `int*` | match |
| `const Box<int*>` | `Box<int*>` | match |
| `const int*` | `int*` | mismatch |
| `const int&` | `int&` | mismatch |

In full-string comparisons, top-level `const` is not ignored for return types,
variable data types, or type alias original types; those are compared as written.
`volatile` is not supported by this parameter-normalization rule. When both types
have known template metadata, the same parameter-normalization rule applies.

The class diagram reader already rewrites `::` to `.` in class/entity IDs,
enclosing namespace IDs, and relationship endpoints of both inputs, so
sequence validators and this validator compare IDs in the same form. Type
names are read as written and normalized only here.

When **both inputs provide known template-instantiation metadata**, the outermost template
actual-argument list may be a **nonempty positional prefix** of the implementation
list: `Map<K, V>` matches `Map<K, V, C>`. Defaults are neither required nor inferred.
No proof that omitted arguments originated from defaults is needed. Metadata stores
the template name, complete argument strings, and optional flat outer decorators.
The template name must match strictly after the normalization above. Outer
`const`, `volatile`, pointer levels, lvalue-reference and rvalue-reference
decorators **must match**, including cv attachment at each layer (cv order within
a layer is canonicalized). `Map<K>&` matches `Map<K, V>&`, but bare `Map<K>`
does not match `const Map<K, V>&`; neither `&` versus `*` nor `&` versus `&&`
matches. Only parameter by-value top-level `const` and top-level pointer `const`
are ignored, as above. Return types, fields and aliases retain all outer cv.

> **WARNING: NESTED TEMPLATE ACTUAL OMISSION IS NOT SUPPORTED.**
> Explicit nested actuals are full strings, never recursively prefix-matched.

Every explicitly listed argument is compared as a **complete normalized string**,
without splitting or parsing nested lists. Inner qualifiers and pointer/reference
modifiers remain strict: `Map<const K>` does not match `Map<K, V>`.
There is no nested prefix relaxation: `Outer<Map<K>>` matches
`Outer<Map<K>, Policy>`, but not `Outer<Map<K, V>, Policy>`. Bare `Map` and `Map<>`
are not wildcards, and the design cannot contain more actuals than the
implementation. Empty lists match only empty lists.

This policy applies uniformly to alias original types, member-variable types,
method/free-function return types, and parameter types, including overload
candidate selection and final field checks. It is directional (design to
implementation), not an equality relation. Callable deduplication therefore uses
exact field and metadata equality (ignoring source location), never prefix matching;
declarations with different or absent metadata remain distinct. An exact type
signature has no priority over another compatible prefix candidate, and return
types do not disambiguate overloads.

If either metadata value or its outer-decorator metadata is absent (including
legacy serialized inputs and unsupported type syntax), types require
**complete normalized string equality**; the validator does not parse display
strings to recover a prefix. Parameter comparisons still ignore top-level `const`
as described above. Otherwise, this retains outer modifiers and full equality for
non-template types: `const K&` does not match `K&`. Literal-bearing strings always
require exact string equality, even if metadata is present. Missing type strings match only missing
type strings. No alias expansion, default-argument insertion, or template
substitution is performed. Declaration template-parameter rules are unchanged;
**Specialization Interface Matching** does not otherwise relax member type checks.

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Duplicate class ID across unit design class diagrams | Entity Presence and Type Consistency |
| Missing implementation entity | Entity Presence and Type Consistency |
| Entity type mismatch | Entity Presence and Type Consistency |
| Entity template parameter mismatch | Template Parameter Consistency |
| Missing type alias | Type Alias Consistency |
| Type alias original type mismatch | Type Alias Consistency |
| Missing variable | Variable Consistency |
| Variable type, visibility, or static flag mismatch | Variable Consistency |
| Missing method | Method Consistency |
| Method return type, visibility, parameter, or modifier mismatch | Method Consistency |
| Method `const` qualification mismatch in either direction | Method Consistency |
| Method template parameter mismatch | Template Parameter Consistency |
| Ambiguous member/free-function overload assignment | Overload Group Matching |
| Multiple designs contend for too few compatible implementations | Overload Group Matching |
| Incompatible template metadata, strict argument-string mismatch, or unequal complete types when metadata is absent | Type Normalization |
| Missing enum literal | Enum Literal Consistency |
| Enum literal data mismatch | Enum Literal Consistency |
| Missing relationship | Relationship Consistency |
| Relationship source, target, or type mismatch | Relationship Consistency |

## Debug Output

The validator emits debug output containing:

- the design entity ID being compared
- the implementation entity ID being compared
- detailed design entity snapshots at trace level
- detailed implementation entity snapshots at trace level
- type aliases, variables, methods, template parameters, enum literals, and
  relationships for each matched entity

Failure messages contain source file and source line information when available.
Missing or empty source metadata is rendered as `<unknown>`.
