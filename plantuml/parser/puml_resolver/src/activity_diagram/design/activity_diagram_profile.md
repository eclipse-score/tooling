# Controlled Activity Diagram Profile

## Purpose

This document defines an initial controlled profile for PlantUML activity diagrams.

The goal is to reduce free-form natural language in activity diagrams and make them easier to compare with sequence diagrams.

Support for comparisons against class or model diagrams, and against implementation-level code evidence, is intentionally left as future work in this document.

This profile is intentionally not a full NLP solution.
Instead, it defines a small set of drawing rules so that activity diagrams can be compared via stable structure and references rather than raw text.

## Scope

This profile is intended for activity diagrams that describe:

- algorithms
- control flow
- callbacks
- interactions with named runtime entities

Typical examples are event access flows such as `GetNewSamples` and `ReferenceNextEvent`. As well as allocation, notification, and subscription-related activities.

## Design Principles

### 1. Human-readable text remains allowed

Each activity node may still have a display text intended for human readers.
This text is not the primary source for consistency checking.

### 2. Machine-comparable semantics must be explicit

Each relevant node shall expose a constrained machine-readable semantic description.

### 3. Comparison is done on logic, not prose

Cross-diagram consistency shall be checked on:

- control structure
- referenced entities
- referenced operations
- read and write sets
- callback and creation semantics

It shall not rely on free-text sentence equality.

In the current scope of this document, this rule is applied to activity-to-sequence comparison.

In practice, this means that two diagrams do not need to use identical sentences.
They only need to expose the same logical flow, the same referenced objects or operations, and the same relevant state updates or callback semantics.

### 4. One activity node may map to multiple sequence interactions

An activity action can represent a single high-level step that is expanded into several sequence messages.
Therefore, consistency checking must support one-to-many mappings.

## Drawing Rules

The rules in this section follow a consistent outline:

- Rule statement: what the diagram author should do
- Rationale: why the rule matters for comparison and review
- Examples: how to follow the rule and what to avoid

### 1. Reuse Stable Names

Rule statement:

When possible, reuse the same names that appear in related sequence diagrams or design documents.
This applies in particular to:

- entities
- variables
- operations
- callback targets
- created objects

Rationale:

Stable names reduce ambiguity and make it easier to align activity steps with sequence interactions, design documentation, and later comparison logic.

Examples:

Preferred examples:

```text
@startuml
start
' Compliant: reuses the stable operation name from related design artifacts.
:ReferenceNextEvent\n@ref: EventDataControl.ReferenceNextEvent;
' Compliant: reuses stable variable names that can be matched across diagrams.
:Adapt maxSampleCount: min(maxSampleCount, freeSamples)\n@reads: [maxSampleCount, freeSamples]\n@writes: [maxSampleCount];
' Compliant: reuses stable callback and payload names.
:Callback App with SamplePtr\n@ref: App.callback\n@target: App\n@arg: SamplePtr;
stop
@enduml
```

Avoid examples:

```text
@startuml
start
' Non-compliant: "Process data" is too vague and does not reuse a stable operation or entity name.
:Process data;
' Non-compliant: "Handle event" does not identify which event-related operation is meant.
:Handle event;
' Non-compliant: "Do callback logic" hides the callback target and callback name.
:Do callback logic;
' Non-compliant: "Update internal state" does not expose which state variable is updated.
:Update internal state;
stop
@enduml
```

### 2. One Action Should Describe One Logical Step

Rule statement:

Each Action node should describe one step that can be matched to a single logical step in the sequence diagram.

Rationale:

An Action is easier to compare when it expresses one effect or one decision point. Combining unrelated effects in one node makes one-to-one or one-to-many matching less clear.

Examples:

Preferred examples:

```text
@startuml
start
' Compliant: one action expresses one sub-activity reference.
:ReferenceNextEvent;
' Compliant: one action expresses one state update.
:Update UpperLimit with timestamp of referenced event;
' Compliant: one action expresses one callback step.
:Callback App with SamplePtr;
stop
@enduml
```

Avoid examples:

```text
@startuml
start
' Non-compliant: this single action combines sub-activity, state update, and callback behavior.
:ReferenceNextEvent, update UpperLimit, and callback App with SamplePtr;
' Non-compliant: this single action combines a state read with object creation.
:Read freeSamples and create SamplePtr;
stop
@enduml
```

### 3. Make the Intent of Each Action Explicit

Rule statement:

For key actions, make the intent clear from the text.
Typical intents are:

- read state
- update state
- call an operation or sub-activity
- check a condition
- callback user logic
- create or destroy an object

Rationale:

Explicit intent helps a reviewer or comparison tool understand whether an Action is reading state, changing state, invoking another operation, or interacting with a user-visible target.

Examples:

Preferred wording:

```text
@startuml
start
' Compliant: the verb "Read" makes the action intent explicit.
:Read freeSamples;
' Compliant: the verb "Call" makes the action intent explicit.
:Call ReferenceNextEvent;
' Compliant: the verb "Callback" makes the user-visible effect explicit.
:Callback App with SamplePtr;
stop
@enduml
```

Avoid wording:

```text
@startuml
start
' Non-compliant: "Process data" does not say whether this reads, updates, creates, or calls.
:Process data;
' Non-compliant: "Handle event" hides the concrete action intent.
:Handle event;
' Non-compliant: "Do callback logic" does not say whether this is a callback, a read, or an update.
:Do callback logic;
stop
@enduml
```

### 4. Make Read and Write Targets Visible

Rule statement:

If an action reads state, mention what is being read.
If an action updates state, mention what is being written and, where useful, from what source.

Rationale:

Visible read and write targets make data dependencies explicit. This is important when comparing activity logic with sequence interactions or implementation evidence.

Examples:

Preferred examples:

```text
@startuml
start
' Compliant: the read and write targets are visible in both the display text and metadata.
:Adapt maxSampleCount: min(maxSampleCount, freeSamples)\n@reads: [maxSampleCount, freeSamples]\n@writes: [maxSampleCount];
' Compliant: the written state and its source are both visible.
:Update UpperLimit with timestamp of referenced event\n@reads: [referenced_event.timestamp]\n@writes: [upper_limit];
stop
@enduml
```

Avoid examples:

```text
@startuml
start
' Non-compliant: this does not say which state is updated.
:Update internal state;
' Non-compliant: this does not say which value changes or where the value comes from.
:Adjust value;
stop
@enduml
```

### 5. Use Explicit Conditions in If Nodes

Rule statement:

If nodes should use conditions that make the success or failure criterion visible.

Rationale:

An explicit condition shows what is actually being checked. This makes control flow easier to review and easier to align with predicates in related diagrams.

Examples:

Preferred examples:

```text
@startuml
start
' Compliant: the condition makes the success criterion explicit.
if (reference success?\n@condition_ref: optional_slot.has_value\n@branches: [yes, no]) then (yes)
  :Callback App with SamplePtr;
else (no)
  :Stop processing;
endif
stop
@enduml
```

Avoid examples:

```text
@startuml
start
' Non-compliant: "continue?" does not say what is being checked.
if (continue?) then (yes)
  :Callback App with SamplePtr;
else (no)
  :Stop processing;
endif
' Non-compliant: "valid?" is too vague to identify the predicate.
if (valid?) then (yes)
  :ReferenceNextEvent;
else (no)
  :Stop processing;
endif
stop
@enduml
```

### 6. Use Explicit Loop Conditions

Rule statement:

While and RepeatWhile should make the loop condition visible.
The loop condition should correspond to a loop or repeated interaction in the related sequence diagram.

Rationale:

An explicit loop condition makes repeated behavior observable and testable. It also makes it easier to compare the activity loop with repeated sequence fragments.

Examples:

Preferred examples:

```text
@startuml
start
repeat
  :ReferenceNextEvent;
' Compliant: the loop condition states exactly why another iteration occurs.
repeat while (maxSampleCount not reached?\n@condition_ref: returned_count < maxSampleCount) is (yes)
stop
@enduml
```

Avoid examples:

```text
@startuml
start
repeat
  :ReferenceNextEvent;
' Non-compliant: "continue?" does not expose the loop predicate.
repeat while (continue?) is (yes)
stop
@enduml
```

### 7. Reuse Named Sub-Activities When Available

Rule statement:

If a step is already modeled as a separate activity or is known as a named operation, use that name directly.

Rationale:

Reusing the known name preserves traceability across diagrams and makes it clearer that the Action refers to an existing sub-activity or operation rather than an informal paraphrase.

Examples:

Preferred example:

```text
@startuml
start
' Compliant: the action reuses the existing sub-activity name directly.
:ReferenceNextEvent\n@ref: EventDataControl.ReferenceNextEvent;
stop
@enduml
```

This makes it easier to align one activity step with one or more sequence interactions.

Avoid example:

```text
@startuml
start
' Non-compliant: this paraphrases an existing operation instead of reusing its known name.
:Find next event;
stop
@enduml
```

### 8. Make Callback and Creation Semantics Visible

Rule statement:

If the flow delivers data to the user, mention the callback target and the delivered object.
If the flow creates an output or helper object, mention that object explicitly.

Rationale:

Callback and creation steps are often important externally visible effects. Naming the target and object makes those effects comparable across diagrams.

Examples:

Preferred examples:

```text
@startuml
start
' Compliant: the created object is named explicitly.
:Create SamplePtr;
' Compliant: the helper object is named explicitly.
:Create SampleDeleter;
' Compliant: the callback target and delivered object are both visible.
:Callback App with SamplePtr\n@ref: App.callback\n@target: App\n@arg: SamplePtr;
stop
@enduml
```

Avoid examples:

```text
@startuml
start
' Non-compliant: this hides both the callback target and the delivered object.
:Return result;
' Non-compliant: this does not identify which helper object is created.
:Allocate helper;
stop
@enduml
```

### 9. Recommended Format per Activity Element

Rule statement:

Write each supported activity element in a regular textual format so that a comparison view can recover stable fields such as kind, ref, reads, writes, in, out, target, and arg.

Rationale:

The rules above describe what should be visible in the diagram. This rule explains how to write that information directly in the PlantUML text so that a parser can extract it consistently.

Examples:

The recommended approach is to write these fields directly in the PlantUML text so that an activity diagram parser can extract them directly.

Recommended pattern:

- human-readable text first
- then one field per line using `@key: value`

General examples:

```
ReferenceNextEvent
@kind: subactivity_ref
@ref: EventDataControl.ReferenceNextEvent
@in: [last_reference_time, upper_limit]
@out: [optional_slot]
```

```
reference success?
@condition_ref: optional_slot.has_value
@branches: [yes, no]
```

```
maxSampleCount not reached?
@repeat_while: returned_count < maxSampleCount
@condition_ref: returned_count < maxSampleCount
```

#### Action

Rule statement:

Write an Action as one logical step using a stable verb plus a concrete object, target, or state.

Rationale:

A clear Action label and a regular field layout make the Action easier to compare with sequence calls, state updates, and callbacks.

Examples:

Preferred example:

```text
@startuml
start
' Compliant: the action uses a stable verb and makes the read intent explicit.
:Read freeSamples\n@kind: read\n@reads: [freeSamples];
' Compliant: the action uses a stable operation name and marks it as a sub-activity reference.
:ReferenceNextEvent\n@kind: subactivity_ref\n@ref: EventDataControl.ReferenceNextEvent\n@in: [last_reference_time, upper_limit]\n@out: [optional_slot];
' Compliant: the action uses a stable callback verb and names the target and payload.
:Callback App with SamplePtr\n@kind: callback\n@ref: App.callback\n@target: App\n@arg: SamplePtr;
stop
@enduml
```

Avoid example:

```text
@startuml
start
' Non-compliant: this action uses a vague label and does not expose the action kind.
:Process data;
' Non-compliant: this action combines multiple logical steps into one node.
:Read freeSamples and callback App with SamplePtr;
' Non-compliant: this action refers to an operation informally instead of using stable metadata.
:Call next event;
stop
@enduml
```

Each Action should make the following information recoverable when relevant:

- display: the exact text shown in the diagram
- kind: the logical category of the action, for example read, update, subactivity_ref, callback, create, or destroy
- ref: the stable name of a referenced operation or sub-activity
- reads: state or variables consumed by the action
- writes: state or variables updated by the action
- in: logical inputs to an operation-like action
- out: logical outputs of an operation-like action
- target: callback target
- arg: callback payload or delivered object

Recommended Action format:

```
display
@kind: <kind>
@ref: <ref>
@reads: [...]
@writes: [...]
@in: [...]
@out: [...]
@target: <target>
@arg: <arg>
```

Examples:

```
ReferenceNextEvent
@kind: subactivity_ref
@ref: EventDataControl.ReferenceNextEvent
@in: [last_reference_time, upper_limit]
@out: [optional_slot]
```

```
Adapt maxSampleCount: min(maxSampleCount, freeSamples)
@kind: update
@reads: [maxSampleCount, freeSamples]
@writes: [maxSampleCount]
```

```
Callback User with SamplePtr
@kind: callback
@ref: App.callback
@target: App
@arg: SamplePtr
```

#### If

Rule statement:

Write an If condition as an explicit observable question.
The condition should describe what is being checked, not a vague outcome.

Rationale:

The If node should expose the predicate behind the branch, not just the fact that a branch exists. This keeps decision logic explicit and comparable.

Examples:

Preferred example:

```text
@startuml
start
' Compliant: the condition is an explicit observable question with stable predicate metadata.
if (reference success?\n@condition_ref: optional_slot.has_value\n@branches: [yes, no]) then (yes)
  :Callback App with SamplePtr;
else (no)
  :Stop processing;
endif
stop
@enduml
```

Avoid example:

```text
@startuml
start
' Non-compliant: "valid?" is too vague and does not identify the checked predicate.
if (valid?) then (yes)
  :Callback App with SamplePtr;
else (no)
  :Stop processing;
endif
' Non-compliant: "continue?" does not explain the branch criterion.
if (continue?) then (yes)
  :ReferenceNextEvent;
else (no)
  :Stop processing;
endif
stop
@enduml
```

Each If should make the following information recoverable:

- display: the exact condition text shown in the diagram
- condition_ref: the stable condition or predicate being checked
- branch outcomes: explicit outcomes such as yes or no, found or not found, success or failure

Recommended If format:

```
display
@condition_ref: <condition_ref>
@branches: [yes, no]
```

Examples:

```
reference success?
@condition_ref: optional_slot.has_value
@branches: [yes, no]
```

```
slot found?
@condition_ref: slot_found
@branches: [yes, no]
```

Avoid vague conditions such as:

- valid?
- okay?
- continue?

#### While

Rule statement:

Write a While condition as an explicit continuation condition.
It should be easy to match to a loop condition in the related sequence diagram.

Rationale:

The While node should show why the loop continues. This gives the loop a stable predicate that can be reviewed and compared.

Examples:

Preferred example:

```text
@startuml
start
' Compliant: the while condition makes the continuation predicate explicit.
while (maxSampleCount not reached?\n@while: returned_count < maxSampleCount\n@condition_ref: returned_count < maxSampleCount) is (yes)
  :ReferenceNextEvent;
endwhile (no)
stop
@enduml
```

Avoid example:

```text
@startuml
start
' Non-compliant: "continue?" does not expose the loop predicate.
while (continue?) is (yes)
  :ReferenceNextEvent;
endwhile (no)
stop
@enduml
```

Each While should make the following information recoverable:

- display: the exact loop condition text shown in the diagram
- while: the logical continuation condition
- condition_ref: the stable predicate behind the loop

Recommended While format:

```
display
@while: <expression>
@condition_ref: <condition_ref>
```

Example:

```
maxSampleCount not reached?
@while: returned_count < maxSampleCount
@condition_ref: returned_count < maxSampleCount
```

#### RepeatWhile

Rule statement:

Write a RepeatWhile condition in the same style as While.
The difference is only that the condition is evaluated after the loop body.

Rationale:

RepeatWhile should expose the post-condition that controls the next iteration. This keeps post-tested loops explicit and comparable.

Examples:

Preferred example:

```text
@startuml
start
repeat
  :ReferenceNextEvent;
' Compliant: the repeat condition exposes the post-tested loop predicate explicitly.
repeat while (maxSampleCount not reached?\n@repeat_while: returned_count < maxSampleCount\n@condition_ref: returned_count < maxSampleCount) is (yes)
stop
@enduml
```

Avoid example:

```text
@startuml
start
repeat
  :ReferenceNextEvent;
' Non-compliant: "loop again?" does not identify the post-condition.
repeat while (loop again?) is (yes)
stop
@enduml
```

Each RepeatWhile should make the following information recoverable:

- display: the exact loop condition text shown in the diagram
- repeat_while or until: the logical post-condition of the loop
- condition_ref: the stable predicate behind the loop

Recommended RepeatWhile format:

```
display
@repeat_while: <expression>
@condition_ref: <condition_ref>
```

Example:

```
maxSampleCount not reached?
@repeat_while: returned_count < maxSampleCount
@condition_ref: returned_count < maxSampleCount
```

## Example: GetNewSamples Fragment

The example below shows how the recommended format from the drawing rules can be applied to a GetNewSamples fragment.

<img alt="GET_NEW_SAMPLES_ACTIVITY" src="https://www.plantuml.com/plantuml/proxy?src=https://raw.githubusercontent.com/eclipse-score/communication/refs/heads/main/score/mw/com/design/events_fields/get_new_samples_activity.puml">

The following illustrates how a free-form activity fragment can be written so that its logic is clear and can later be compared with a sequence diagram.
The field names used below correspond to the recommended format described in the drawing rules above.

Source activity actions:

- Determine point in time of last call to GetNewSamples
- Adapt maxSampleCount: min(maxSampleCount, freeSamples)
- ReferenceNextEvent
- reference success?
- Update UpperLimit with timestamp of referenced event
- Callback User with SamplePtr
- maxSampleCount not reached?

Possible structured description used for comparison:

    - kind: read
      display: Determine point in time of last call to GetNewSamples
      @reads: [last_reference_time]
      @effect_kind: ReadState

    - kind: update
      display: Adapt maxSampleCount: min(maxSampleCount, freeSamples)
      @reads: [max_sample_count, free_samples]
      @writes: [max_sample_count]
      @effect_kind: ValueRestriction

    - kind: subactivity_ref
      display: ReferenceNextEvent
      @ref: EventDataControl.ReferenceNextEvent
      @in: [last_reference_time, upper_limit]
      @out: [optional_slot]
      @effect_kind: ReadSelection

    - kind: check
      display: reference success?
      @condition_ref: optional_slot.has_value
      @branches: [yes, no]

    - kind: update
      display: Update UpperLimit with timestamp of referenced event
      @reads: [referenced_event.timestamp]
      @writes: [upper_limit]
      @effect_kind: StateUpdate

    - kind: callback
      display: Callback User with SamplePtr
      @ref: App.callback
      @target: App
      @arg: SamplePtr
      @effect_kind: UserVisibleDelivery

    - kind: loop
      display: maxSampleCount not reached?
      @repeat_while: returned_count < maxSampleCount
      @condition_ref: returned_count < maxSampleCount

Possible activity diagram written according to this profile:

```text
@startuml get_new_samples_profile_example
title "GetNewSamples (profile example)"

start

:Determine point in time of last call to GetNewSamples\n@kind: read\n@reads: [last_reference_time]\n@effect_kind: ReadState;

:Adapt maxSampleCount: min(maxSampleCount, freeSamples)\n@kind: update\n@reads: [maxSampleCount, freeSamples]\n@writes: [maxSampleCount]\n@effect_kind: ValueRestriction;

repeat
  :ReferenceNextEvent\n@kind: subactivity_ref\n@ref: EventDataControl.ReferenceNextEvent\n@in: [last_reference_time, upper_limit]\n@out: [optional_slot]\n@effect_kind: ReadSelection;

  if (reference success?\n@condition_ref: optional_slot.has_value\n@branches: [yes, no]) then (no)
    break
  else (yes)

    :Update UpperLimit with timestamp of referenced event\n@kind: update\n@reads: [referenced_event.timestamp]\n@writes: [upper_limit]\n@effect_kind: StateUpdate;

    :Callback User with SamplePtr\n@kind: callback\n@ref: App.callback\n@target: App\n@arg: SamplePtr\n@effect_kind: UserVisibleDelivery;
  endif
repeat while (maxSampleCount not reached?\n@repeat_while: returned_count < maxSampleCount\n@condition_ref: returned_count < maxSampleCount) is (yes)

stop

@enduml
```

## What Will Be Compared Against Sequence Diagrams

To support activity-to-sequence consistency checking, the following aspects should be visible in the activity diagram:

- control structure
- referenced entities
- referenced operations or sub-activities
- read and write intent
- callback and creation semantics

The comparison does not require sentence equality.
A single activity step may map to one sequence message or to a small sequence fragment.

Example:

- activity step: ReferenceNextEvent
- sequence fragment:
  - `ProxyEvent -> EventDataControl: referenceNextEvent(...)`
  - `EventDataControl --> ProxyEvent: optional<EventSlotIndexType>`

This is a valid one-to-many mapping.

## Future Work

Comparison against class or model diagrams and against code is not covered by this document yet.
