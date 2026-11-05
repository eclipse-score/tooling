<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Sequence Diagram Design Guide

## Purpose

This guide defines how sequence diagrams are drawn for an architectural design
and a unit design, and which rules the validators apply to them. It follows
UML sequence-diagram semantics. Deviations are listed in
[Deviations from UML](#deviations-from-uml).

The related validators are specified in:

- `component_sequence.md` (units and interface connections)
- `sequence_internal_api.md` (methods, roles, coverage, gate messages)
- `sequence_public_api.md` (actor calls and callbacks against the public API)
- `class_design_sequence.md` (class lifelines and operations)

## Lifelines

A lifeline is a `participant` or `actor`. The written name is the identifier
(see `plantuml/parser/docs/element-identifiers.md`); the alias is a local key
for messages.

| Declaration | Meaning | Checked against |
|---|---|---|
| `participant unit_1` | Unit of the architecture, or class of a unit design | Unit ids / class ids |
| `participant comp_a` | Component or SEooC drawn as one black box | Component and SEooC ids |
| `actor User` | Application or user of the SEooC | Public API |
| `participant Platform <<external>>` | Platform or another SEooC (stereotype letter case is ignored) | Not checked |

Rules:

1. Actors and `<<external>>` participants are the environment. They need no
   counterpart in the component or class diagram.
   *(Requirement: {requirement:downstream-ref}`Tools.SequenceEnvironmentParticipants`)*
2. A component or SEooC lifeline stands for all of its units as a black box.
   It is accepted by Identifier Consistency, but it does not exercise the units
   inside it: a unit is covered only by a lifeline of its own or by a message
   that reaches it. Its interfaces are the union of the interfaces of its
   units.
3. Declare every lifeline. An undeclared message endpoint becomes a plain
   `participant` and must then resolve to a unit or class.
4. An `actor` or `<<external>>` declaration in any sequence diagram applies to
   that uid in all sequence diagrams. Declaring one uid as `actor` in one place
   and `<<external>>` in another is an error.
5. A uid declared as `actor` or `<<external>>` shall not name a unit, component
   or SEooC of the component diagram. The environment is outside the described
   architecture; declare the lifeline as a plain `participant` or rename it.
6. A uid that matches both a unit and a component or SEooC is ambiguous and
   is reported instead of being resolved to one of them.

## Messages

The arrow selects the message kind, as in UML:

| Arrow | UML kind | Validated as operation call |
|---|---|---|
| `A -> B : op()` | Synchronous call | yes |
| `A ->> B : op()` | Asynchronous call | yes |
| `B --> A : result` | Reply | no |
| `B -->> A : result` | Reply | no |
| `A -> B ** : op()` | Create message | no |
| `A -> B !! : op()` | Delete message | no |

Only the `>>` and `<<` arrowheads make a call asynchronous. The half
arrowheads `\` and `/` are drawing style and do not change the kind.

Rules:

1. Only the `**` suffix makes a message a create message. A `create B`
   statement is a lifecycle statement for the lifeline `B`; the messages to `B`
   stay ordinary calls and are validated. The `destroy B` statement ends the
   lifeline and is not a message.
2. A call label is `name(params)`. The method name is the text before the first
   `(`; the parameters are not compared with the internal API.
3. Replies carry data back to the caller. Write them as dashed arrows; a
   `return` statement is ignored. They are never matched against an API,
   whatever the label looks like (`Ack`, `Result(x)`).
   *(Requirement: {requirement:downstream-ref}`Tools.SequenceMessageKinds`)*
4. A callback is a call, synchronous or asynchronous, from an element of the
   SEooC to an actor. Its label is the name of the public-API callback
   interface or one of its methods.

## Gates, Found and Lost Messages

A message with no sender (`[-> B : op()`, found) or no receiver
(`A ->] : op()`, lost) is a call from or to an unspecified environment, as
the UML gate.

- A found message to an element shall use a method declared on an interface
  the element provides, either in the internal API or in the public API. A
  method that is not declared on a provided interface is an error.
- A declared found message counts as a use of the method for Interface
  Coverage; a rejected one does not.
- A lost message is not checked and does not count for Interface Coverage.
- A gate message is not checked for an interface connection between units.
  *(Requirement: {requirement:downstream-ref}`Tools.SequenceGateMessages`)*

Use a gate when the partner is intentionally unspecified. Use `actor` or
`<<external>>` when the partner is known.

## Public API Boundary

When a public API is declared, messages between an actor and the SEooC are
checked against it, whether or not internal API diagrams are declared
(`sequence_public_api.md`).
*(Requirement: {requirement:downstream-ref}`Tools.SequencePublicApiBoundary`)*

- Actor to SEooC element (or component, SEooC): the called method shall be
  declared on a public API interface the element provides.
- SEooC to actor (callback): the label shall name a public API interface the
  element requires, or one of its methods.

The scope comes from the component diagram: an element provides a public
interface when it is bound with `)-` and requires it when bound with `-(`.
If the component diagram is missing, or the element binds no public interface,
the check falls back to the whole public API and compares names only.

Messages to `<<external>>` participants are not checked: the interface of a
platform is not part of the SEooC. Without a declared public API the boundary
check is skipped.

## Internal Calls

Calls between units, between a unit and itself, and between a unit and a
container are checked as in `sequence_internal_api.md`:

- the method is declared on a shared interface (Method-Name Consistency),
- the caller requires and the callee provides it (Role Consistency),
- every declared method is called at least once (Interface Coverage); calls
  from or to actors and `<<external>>` participants do not count.

A call with a container lifeline needs a shared interface with its partner,
using the interfaces of the units inside the container. The method shall be
declared on one of the shared interfaces, in the internal API or in the public
API. Roles are not checked, because a container has no interface roles of its
own. The call does not exercise the interface connection between two units
inside the container. Calls with an environment participant are skipped by
the unit-pair checks.

## Unit Design

In a unit design, lifelines are classes. Actors and `<<external>>`
participants are skipped by Participant-Class Consistency; messages to them
are skipped by Message-Operation Consistency.

## Modeling C++ Constructs

Each construct below shows the C++ code and how it is drawn. The lifelines are
classes in a unit design and units in an architectural design; the patterns are
the same. A label is `name(arguments)`; only the name, the text before the
first `(`, is compared.

### Function Call

A call of a member function on another object is a synchronous message to the
lifeline of the object's class. The method shall be declared on the callee
(class design) or on an interface shared by caller and callee (internal API).

```text
' PlantUML sequence diagram
participant Controller
participant Repository

Controller -> Repository : FindById(id)
```

```cpp
// implementation C++ code
void Controller::Load(Id id) {
  Entity entity = repository_.FindById(id);
}
```

### Self Call

A call of another member function of the same class is a message from the
lifeline to itself. Private methods are valid targets.

```text
' PlantUML sequence diagram
participant Controller

Controller -> Controller : Validate()
```

```cpp
// implementation C++ code
void Controller::Execute() {
  Validate();
}
```

### Inherited and Virtual Methods

In a unit design, a method declared in a base class or an implemented interface
is a valid target on the derived class, unless it is `private` in the base.

```text
' PlantUML sequence diagram
participant Controller
participant SqlRepository

Controller -> SqlRepository : FindById(id)
```

```cpp
// implementation C++ code
class Repository {
 public:
  virtual Entity FindById(Id id) = 0;
};

class SqlRepository : public Repository {};

void Controller::Load(Id id) {
  Entity entity = sql_repository_.FindById(id);
}
```

### Static Method Call

A static method is called on the lifeline of its class. No instance is needed.

```text
' PlantUML sequence diagram
participant Controller
participant Repository

Controller -> Repository : Open(path)
```

```cpp
// implementation C++ code
void Controller::Connect(const Path& path) {
  Repository repository = Repository::Open(path);
}
```

### Return Value

A returned value is a reply: a dashed arrow back to the caller. Replies are not
matched against an API. A `return` statement is ignored.

```text
' PlantUML sequence diagram
participant Controller
participant Repository

Controller -> Repository : FindById(id)
Repository --> Controller : entity
```

```cpp
// implementation C++ code
Entity Repository::FindById(Id id) {
  return entities_.at(id);
}
```

### Object Creation

A constructor call that allocates the object is a create message (`**`). It is
not matched against an API, so the label is free text.

```text
' PlantUML sequence diagram
participant Controller
participant Repository

Controller -> Repository ** : Repository(database)
Controller -> Repository : FindById(id)
```

```cpp
// implementation C++ code
void Controller::Open() {
  repository_ = std::make_unique<Repository>(database_);
  Entity entity = repository_->FindById(id_);
}
```

Two-phase construction is drawn with a `create` statement. The lifeline starts
at the statement, and the `Init` call that follows is an ordinary call that is
validated.

```text
' PlantUML sequence diagram
participant Controller
create Repository

Controller -> Repository : Init(database)
```

```cpp
// implementation C++ code
void Controller::Open() {
  repository_ = std::make_unique<Repository>();
  repository_->Init(database_);
}
```

### Object Destruction

Destroying the object is a delete message (`!!`) or a `destroy` statement. The
lifeline ends there; a later message to it is an error unless it is created
again.

```text
' PlantUML sequence diagram
participant Controller
participant Repository

Controller -> Repository !! : ~Repository()
```

```cpp
// implementation C++ code
void Controller::Close() {
  repository_.reset();
}
```

```text
' PlantUML sequence diagram
participant Controller
participant Repository

destroy Repository
```

### Asynchronous Call

A call that returns before the work is done is an asynchronous message (`->>`).
Posting to an executor, starting a thread or writing to a queue are examples.
The method is validated like a synchronous call.

```text
' PlantUML sequence diagram
participant Controller
participant Executor

Controller ->> Executor : Post(job)
```

```cpp
// implementation C++ code
void Controller::Submit(Job job) {
  executor_.Post(std::move(job));
}
```

### Call from the Application

The application of the SEooC is an `actor`. The called method shall be declared
on a public API interface provided by the called element
(`sequence_public_api.md`).

```text
' PlantUML sequence diagram
actor Application
participant Controller

Application -> Controller : Start(config)
```

```cpp
// application C++ code
void Application::Run() {
  controller.Start(config);
}
```

### Callback to the Application

A call from the SEooC to code of the application, such as a listener or a
`std::function`, is a message to the `actor`. The label is a method of a public
API callback interface that the calling element requires, or the interface
name.

```text
' PlantUML sequence diagram
actor Application
participant Controller

Controller -> Application : OnFinished()
```

```cpp
// implementation C++ code
class Listener {
 public:
  virtual void OnFinished() = 0;
};

void Controller::Finish() {
  listener_->OnFinished();
}
```

### Call to the Platform or Another SEooC

Operating system, standard library and other SEooCs are `<<external>>`
participants. Messages to and from them are not checked.

```text
' PlantUML sequence diagram
participant Controller
participant Platform <<external>>

Controller -> Platform : ReadClock()
Platform ->> Controller : OnSignal()
```

```cpp
// implementation C++ code
void Controller::Poll() {
  TimePoint now = platform::ReadClock();
}

void Controller::OnSignal() {
  // called from a signal handler of the platform
}
```

### Call to a Component

In an architectural design, a component or SEooC can be drawn as one black box.
The method shall be declared on an interface that the caller shares with the
units inside the component. The call does not cover the interface connection
of a single unit.

```text
' PlantUML sequence diagram
participant Controller
participant component_b

Controller -> component_b : GetData()
```

```cpp
// implementation C++ code
void Controller::Refresh() {
  Data data = data_source_.GetData();  // provided by a unit of component_b
}
```

### Call from or to an Unspecified Partner

A function that is called by an unspecified caller, such as a timer framework
or an interrupt handler, is a found message. A call to unspecified receivers,
such as unknown subscribers, is a lost message. The found message shall use a
method of an interface that the receiver provides.

```text
' PlantUML sequence diagram
participant Controller

[-> Controller : OnTimer()
Controller ->] : Publish(event)
```

```cpp
// implementation C++ code
void Controller::OnTimer() {  // called by a timer framework
  event_bus_.Publish(event_);  // subscribers are unknown
}
```

### Control Flow

`if`/`else` is drawn as `alt`, an `if` without `else` as `opt`, and `for` and
`while` as `loop`. All branches are validated like straight-line messages.
`par` and `break` are validated the same way. The messages inside `critical`
are not validated.

```text
' PlantUML sequence diagram
participant Controller
participant Repository
participant Logger

alt entity is new
    Controller -> Repository : Insert(entity)
else entity exists
    Controller -> Repository : Update(entity)
end

loop for each pending entity
    Controller -> Repository : Store(pending)
end

opt logging enabled
    Controller -> Logger : Write(text)
end
```

```cpp
// implementation C++ code
void Controller::Sync() {
  if (is_new_) {
    repository_.Insert(entity_);
  } else {
    repository_.Update(entity_);
  }

  for (const Entity& pending : pending_) {
    repository_.Store(pending);
  }

  if (logging_enabled_) {
    logger_.Write(text_);
  }
}
```

### Overloads, Templates and Operators

Overloads share one name and are not told apart. Template arguments are not
written in the label, because they would become part of the method name. The
call operator and other operators keep their name.

```text
' PlantUML sequence diagram
participant Controller
participant Repository
participant Handler

Controller -> Repository : Find(key)
Controller -> Handler : operator()(event)
```

```cpp
// implementation C++ code
void Controller::Run() {
  repository_.Find<Key>(key_);
  handler_(event_);
}
```

### Namespaced Classes

A class in a namespace is declared with its qualified name and an alias for the
messages.

```text
' PlantUML sequence diagram
participant Controller
participant "pkg::Repository" as repository

Controller -> repository : FindById(id)
```

```cpp
// implementation C++ code
namespace pkg {
class Repository {
 public:
  Entity FindById(Id id);
};
}  // namespace pkg
```

## Deviations from UML

| UML | Here | Reason |
|---|---|---|
| Any lifeline may be a role | Lifelines are units, classes, containers, actors or `<<external>>` | Traceability to the architecture |
| Return values on the call arrow | Reply messages are not checked | Operation signatures are in the class design |
| Combined fragments (`alt`, `loop`, `opt`) | Parsed; branches are validated like straight-line messages | Coverage needs all branches |
| Interaction use (`ref`) | Parsed, not expanded | The referenced diagram is not followed |
| Parameters on a call | Only the operation name is compared | Types are not part of the sequence |

## Example

```text
actor User
participant "pkg::Controller" as ctrl
participant "pkg::Repository" as repo
participant Platform <<external>>

User -> ctrl : Start(config)
ctrl -> repo : FindById(id)
repo --> ctrl : entity
ctrl ->> Platform : Log(msg)
ctrl ->> User : OnFinished()
```

- `Start` shall be declared on a public API interface.
- `OnFinished` shall be a public API callback interface or one of its methods.
- `FindById` shall be declared on an interface shared by `ctrl` and `repo`.
- The reply and the message to `Platform` are not checked.
