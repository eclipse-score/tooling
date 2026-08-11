<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Sequence Diagram Support Guide

This guide describes the syntax and semantics currently supported by the PlantUML sequence diagram parser and resolver. The goal is to produce sequence interactions that can be consumed by the logical model, rather than only rendering a diagram.

**Recommended guidelines:**

- Declare participants explicitly.
- Use aliases consistently after declaring them.
- Use exactly one arrowhead for each message.
- Split complex lifecycle actions into individual actions.
- Explicitly close every group with `end`.

## Minimal valid example

The following example uses only structures that are fully modeled and can be used as a starting point for a new file:

```text
@startuml OrderFlow

actor Client
participant "Order Service" as OrderService <<service>>
database Orders

Client -> OrderService : submit(order)
activate OrderService

alt order is valid
    OrderService -> Orders : save(order)
else order is invalid
    OrderService --> Client : rejected
end

deactivate OrderService
@enduml
```

After resolution, this example contains a participant table, message interactions, activation/deactivation actions, and a conditional branch node with two branches.

## Supported and modeled content

### Participants

The following participant declaration forms are supported:

```text
participant Service
participant "Order Service"
participant "Order Service" as OrderService
participant OrderService as "Order Service"  // not recommended
participant "Order Service" as OrderService <<service>>
```

The alias form is recommended. Although `participant OrderService as "Order Service"` is supported by the current grammar, the consistent form should be `"Display Name" as Alias` to avoid confusing display names with reference names.

The display name, alias, participant type, and stereotype are written to the logical model.

```text
participant "Order Service" as OrderService
Client -> OrderService : correct()
```

After declaring an alias, subsequent messages, lifecycle commands, and `ref` blocks should use that alias consistently. Referring to the quoted display name instead may create a separate implicit participant, so using the display name as a message endpoint is not recommended.

Undeclared message endpoints are automatically created as regular `participant` instances. This is convenient for short diagrams, but explicit declarations are recommended for production diagrams to preserve participant type, stereotype, and stable source locations.

### Messages and arrow direction

Messages with exactly one arrowhead are supported.

```text
A -> B : A call B with a solid line
A --> B : A call B with a dashed line
A <- B : B call A with a solid line   // not recommended
A <-- B : B call A with a dashed line // not recommended
```

The resolver determines the sender and receiver from the arrow direction. Reverse arrows such as `<-` are not recommended. Line styles and arrow decorations do not change the sender/receiver relationship. The message text follows the colon and may be omitted:

```text
Service -> Worker : process(item)
Worker --> Service :result
```

The difference between `->` and `-->` is only the line style. The resolver parses both as the same kind of logical message (`Interaction`).

`-->` does not automatically acquire response semantics. If a response only needs to be shown in the PlantUML diagram, **use `return`**:

```text
Service -> Worker : process(item)
return result
```

The parser accepts `return result` and PlantUML can render it as a response. The resolver does not convert it into an interaction node in the logical model. If the response must enter the logical model, use an explicit reverse message such as `Worker --> Service : result`. This is resolved as `Worker -> Service` with the message text `"result"`.

Self-messages, messages with a missing endpoint, and lost/found markers are supported:

```text
Service -> Service : retry()
--> Service : incoming request
Service --> : outgoing response
[-> Service : incoming from outside
Service -->] : outgoing to outside
```

Messages with missing endpoints and lost/found markers (`[`, `]`, and `?`) are represented with a `null` endpoint in the logical model and do not create user-declared participants. Avoid these forms unless necessary; when modeling an external interaction, explicitly declaring a participant is recommended.

### Lifecycle

Standalone commands and message suffixes are supported. Each action with a concrete participant target produces a lifecycle node. Lost/found endpoints have no participant target and therefore do not produce a corresponding lifecycle node.

```text
create Worker
activate Worker
deactivate Worker

Worker ++
Worker --

Service -> Worker ++ : activate Worker
Worker -> Repository ** : create Repository
Repository --> Worker -- : response
Service -> Worker !! : destroy Worker
```

Message suffixes apply as follows:

| Suffix | Action | Target |
|---|---|---|
| `++` | activate | Message receiver |
| `--` | deactivate | Message sender |
| `**` | create | Message receiver, before the interaction |
| `!!` | destroy | Message receiver |

Standalone `create`, `activate`, `deactivate`, and `destroy` commands are recommended. Message suffixes are suitable when the action is strictly bound to a single message.

The `create` command may also specify a participant type, display name, alias, and stereotype, for example `create database "Event Store" as EventStore <<storage>>`. These participant properties are preserved in the participant model.

Combined suffixes are supported. The resolver creates lifecycle nodes in the source-code order of the suffixes. `**` creates the participant before the message interaction; other actions are created after the interaction. For example:

```text
Service -> Worker --++ : response // not recommended
```

is equivalent to the following logical nodes:

```text
Interaction(Service -> Worker)
Lifecycle(Service, Deactivate)
Lifecycle(Worker, Activate)
```

Here, `--` applies to the message sender, while `++` and `!!` apply to the message receiver. Combined suffixes should still be used only when their semantics and order are clear.

Within the same logical block or branch, the resolver prohibits using a participant after `destroy` through a message, reference, `activate`, or `deactivate`. A destruction state inside a branch does not propagate outside that branch. To explicitly make the participant usable again, first use `create`, or recreate it with a single `**` suffix:

```text
create Worker
destroy Worker
create Worker
Service -> Worker : usable again
```

### Control structures

Nested `alt`, `opt`, `loop`, `par`, `break`, and `group` structures are supported. `alt`, `opt`, `loop`, `par`, and `break` produce logical nodes; `group` is only a transparent container. Labels are retained as conditions, loop conditions, parallel-branch labels, or break reasons. `critical` is accepted by the parser but is not part of the current logical model.

```text
// alt-else
alt cache hit
    Client -> Service : return cached value
else cache miss
    Client -> Service : fetch value
end

// opt
opt tracing enabled
    Service -> Audit : record()
end

// loop
loop while pending
    Client -> Service : poll()
end

// par-else
par primary path
    Service -> Worker : dispatch()
else audit path
    Service -> Audit : record()
end

// break
break invalid request
    Service --> Client : rejected
end
```

Rules:

- The current version accepts `critical` syntax and checks its group closure, but does not create a logical node; its contents do not enter the logical model.
- `else` is allowed only in `alt` and `par`. Using it in `opt`, `loop`, `break`, or `group` produces an error.
- An `else` in `alt` creates an additional conditional branch; an `else` in `par` creates an additional parallel branch.
- Every group must be closed with `end`. The untyped form `end` and a matching form such as `end loop` are both allowed.
- A typed end statement must match the innermost group, so `loop ... end alt` is invalid.
- `group` can improve diagram readability, but it is transparent in the logical model: its interactions are promoted to the outer level. Use `alt`, `opt`, `loop`, `par`, or `break` when downstream tools need to understand business control flow.

### Reference blocks

Single-line and multi-line `ref over` blocks are supported. The participant list and reference text are retained:

```text
ref over Service, Worker : shared behavior

ref over Service, Worker
    See the separate retry sequence.
end ref
```

It is recommended that participants in a `ref` block are established by a declaration, message, or `create` first. A `ref` block does not create implicit participants. Referencing a destroyed participant produces an error.

## Content accepted but not included in the logical model

The following syntax is accepted or skipped during parsing for compatibility with existing PlantUML source files. The resolver does not create logical nodes for it or retain its visual effects. Do not rely on this syntax to express business semantics that downstream tools must understand.

In particular, the current logical model does not support `return` or `critical`. The parser accepts them, but the resolver does not create a return interaction node or a critical-section logical node. Use a supported explicit message or control structure when the business meaning must enter the logical model.

| Category | Accepted example | Current behavior |
|---|---|---|
| Participant order and colors | `participant Service order 1 #LightBlue` | Parsed, but order and color are ignored |
| Lifecycle colors | `activate Service #LightBlue`, `Service -> Worker ++ #gold` | Parsed, but color is ignored |
| Parallel message marker | `&Service -> Worker : parallel call` | Parsed, but the parallel marker is ignored; use a `par` block for modeled parallel control flow |
| Visual grouping | `group Deployment ... end group` | Contents are retained, but no `group` container node is created |
| Critical section | `critical exclusive ... end` | Syntax and group balance are checked, but its contents do not enter the logical model |
| Return command | `return result` | Parsed, but no interaction or return node is created |
| Title, legend, sprite, and transformation | `legend ... end legend`, `sprite ...` | Ignored |
| Preprocessing and display settings | `!pragma`, `!function`, `skinparam`, `autonumber`, `autoactivate`, `footbox` | Ignored |
| Layout and visibility | `box ... end box`, `minwidth`, `rotate`, `hide`, `show`, `== section ==`, `...`, `delay` | Ignored |

## Resolver errors

### Messages with no unique direction

A message must have exactly one arrowhead. Bidirectional or directionless arrows produce an error during resolution:

```text
A <--> B : invalid
A -- B : invalid
```

In addition to invalid message directions, the resolver rejects sequence diagrams in the following cases:

| Case | Example or description | Recommended fix |
|---|---|---|
| Unclosed group | `alt ...` without `end` | Add an end statement for every `alt`, `opt`, `loop`, `par`, `break`, `group`, or `critical` |
| Mismatched group end | `loop ... end alt` | Make the end statement match the innermost group type |
| `else` in an invalid group | `opt ... else ... end` | Use `else` only in `alt` or `par` |
| Use of a destroyed participant | Sending a message or executing `ref`, `activate`, or `deactivate` after `destroy Worker` | Use `create Worker` first or reorder the interaction |

These errors usually include the source file and line number. The repository also provides corresponding error cases, including:
- [invalid message direction](../integration_test/sequence_diagram/invalid_message_direction/invalid_message_direction.puml)
- [an unclosed group](../integration_test/sequence_diagram/invalid_unterminated_group/invalid_unterminated_group.puml)
- [an invalid `else`](../integration_test/sequence_diagram/invalid_else_in_opt/invalid_else_in_opt.puml)
- [a mismatched group end](../integration_test/sequence_diagram/invalid_mismatched_group_end/invalid_mismatched_group_end.puml)
- [use of a participant after destruction](../integration_test/sequence_diagram/invalid_destroyed_participant_use/invalid_destroyed_participant_use.puml)

## Authoring checklist

Use the following checklist when submitting a sequence diagram:

1. The file starts with `@startuml` and ends with `@enduml`.
2. Important participants are declared explicitly, with stable aliases for names containing spaces or complex display names.
3. Once an alias is declared, references use that alias consistently to avoid creating a separate implicit participant.
4. Every message has exactly one arrowhead. Use `return` only for responses shown in the diagram; use an explicit reverse message when the response must enter the logical model.
5. Combined message suffixes have clear semantics and order. `--` applies to the sender, `++` and `!!` apply to the receiver, and `**` creates the receiver before the interaction.
6. Every control structure is closed, and `else` appears only in `alt` or `par`.
7. A participant is not used after `destroy` unless it is created again first.
8. Interactions that downstream tools must understand use only structures described as supported and modeled; visual-only directives do not carry business semantics.

## Related test cases

The repository provides runnable end-to-end examples:

- [Complete sequence diagram](../integration_test/sequence_diagram/comprehensive_sequence_test.puml)
- [Lifecycle](../integration_test/sequence_diagram/sequence_lifecycle_nodes/sequence_lifecycle_nodes.puml)
- [Conditional branch](../integration_test/sequence_diagram/sequence_branch_node/sequence_branch_node.puml)
- [Parallel branch](../integration_test/sequence_diagram/sequence_parallel_node/sequence_parallel_node.puml)
- [Loop](../integration_test/sequence_diagram/sequence_loop_node/sequence_loop_node.puml)
- [Reference](../integration_test/sequence_diagram/sequence_reference_node/sequence_reference_node.puml)
