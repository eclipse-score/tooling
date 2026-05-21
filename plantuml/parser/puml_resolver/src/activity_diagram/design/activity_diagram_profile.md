# Controlled Activity Diagram Profile

## Purpose

This document defines an initial controlled profile for PlantUML activity diagrams.
The goal is to reduce free-form natural language in activity diagrams and make them easier to compare with:

- sequence diagrams

Support for comparisons against class or model diagrams, and against implementation-level code evidence, is intentionally left as future work in this document.

This profile is intentionally not a full NLP solution.
Instead, it defines a small set of drawing rules so that activity diagrams can be compared via stable structure and references rather than raw text.

## Scope

This profile is intended for activity diagrams that describe:

- algorithms
- control flow
- state updates
- callbacks
- interactions with named runtime entities

Typical examples are event access flows such as GetNewSamples, ReferenceNextEvent, allocation, notification, and subscription-related activities.

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

### 1. Supported Activity Elements

Use only the following activity elements:

- Action
- If
- While
- RepeatWhile

### 2. Reuse Stable Names

When possible, reuse the same names that appear in related sequence diagrams or design documents.
This applies in particular to:

- entities
- variables
- operations
- callback targets
- created objects

Prefer concrete names such as:

- EventDataControl
- ProxyEvent
- SamplePtr
- maxSampleCount
- freeSamples
- ReferenceNextEvent
- App.callback

Avoid vague labels such as:

- Process data
- Handle event
- Do callback logic
- Update internal state

### 3. One Action Should Describe One Logical Step

Each Action node should describe one step that can be matched to a single logical step in the sequence diagram.

Good examples:

- ReferenceNextEvent
- Update UpperLimit with timestamp of referenced event
- Callback User with SamplePtr

Avoid combining unrelated effects in a single Action node.

### 4. Make the Intent of Each Action Explicit

For key actions, make the intent clear from the text.
Typical intents are:

- read state
- update state
- call an operation or sub-activity
- check a condition
- callback user logic
- create or destroy an object

When possible, use stable verbs such as:

- Read
- Update
- Call
- Check
- Create
- Destroy
- Callback

### 5. Make Read and Write Targets Visible

If an action reads state, mention what is being read.
If an action updates state, mention what is being written and, where useful, from what source.

Good examples:

- Adapt maxSampleCount: min(maxSampleCount, freeSamples)
- Update UpperLimit with timestamp of referenced event

### 6. Use Explicit Conditions in If Nodes

If nodes should use conditions that make the success or failure criterion visible.

Good examples:

- reference success?
- slot found?
- maxSampleCount not reached?

Avoid vague conditions such as:

- continue?
- valid?
- okay?

### 7. Use Explicit Loop Conditions

While and RepeatWhile should make the loop condition visible.
The loop condition should correspond to a loop or repeated interaction in the related sequence diagram.

Good examples:

- maxSampleCount not reached?
- no new samples available?

### 8. Reuse Named Sub-Activities When Available

If a step is already modeled as a separate activity or is known as a named operation, use that name directly.

Example:

- ReferenceNextEvent

This makes it easier to align one activity step with one or more sequence interactions.

### 9. Make Callback and Creation Semantics Visible

If the flow delivers data to the user, mention the callback target and the delivered object.
If the flow creates an output or helper object, mention that object explicitly.

Good examples:

- Callback User with SamplePtr
- Create SamplePtr
- Create SampleDeleter

### 10. Recommended Format per Activity Element

The rules above describe what should be visible in the diagram.
This section explains how each supported activity element should be written so that a comparison view can recover stable fields such as kind, ref, reads, writes, in, out, target, and arg.

The recommended approach is to write these fields directly in the PlantUML text so that an activity diagram parser can extract them directly.

Recommended pattern:

- human-readable text first
- then one field per line using `@key: value`

General examples:

- `ReferenceNextEvent\n@kind: subactivity_ref\n@ref: EventDataControl.ReferenceNextEvent\n@in: [last_reference_time, upper_limit]\n@out: [optional_slot]`
- `reference success?\n@condition_ref: optional_slot.has_value\n@branches: [yes, no]`
- `maxSampleCount not reached?\n@repeat_while: returned_count < maxSampleCount\n@condition_ref: returned_count < maxSampleCount`

#### Action

Write an Action as one logical step using a stable verb plus a concrete object, target, or state.

Preferred text patterns:

- Read `state`
- Update `target` with `source`
- Call `operation`
- Create `object`
- Destroy `object`
- Callback `target` with `object`

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

- `display\n@kind: <kind>\n@ref: <ref>\n@reads: [...]\n@writes: [...]\n@in: [...]\n@out: [...]\n@target: <target>\n@arg: <arg>`

Examples:

- `ReferenceNextEvent\n@kind: subactivity_ref\n@ref: EventDataControl.ReferenceNextEvent\n@in: [last_reference_time, upper_limit]\n@out: [optional_slot]`

- `Adapt maxSampleCount: min(maxSampleCount, freeSamples)\n@kind: update\n@reads: [maxSampleCount, freeSamples]\n@writes: [maxSampleCount]`

- `Callback User with SamplePtr\n@kind: callback\n@ref: App.callback\n@target: App\n@arg: SamplePtr`

#### If

Write an If condition as an explicit observable question.
The condition should describe what is being checked, not a vague outcome.

Preferred text patterns:

- `condition`?
- `object` found?
- `operation` success?

Each If should make the following information recoverable:

- display: the exact condition text shown in the diagram
- condition_ref: the stable condition or predicate being checked
- branch outcomes: explicit outcomes such as yes or no, found or not found, success or failure

Recommended If format:

- `display\n@condition_ref: <condition_ref>\n@branches: [yes, no]`

Examples:

- `reference success?\n@condition_ref: optional_slot.has_value\n@branches: [yes, no]`

- `slot found?\n@condition_ref: slot_found\n@branches: [yes, no]`

Avoid vague conditions such as:

- valid?
- okay?
- continue?

#### While

Write a While condition as an explicit continuation condition.
It should be easy to match to a loop condition in the related sequence diagram.

Each While should make the following information recoverable:

- display: the exact loop condition text shown in the diagram
- while: the logical continuation condition
- condition_ref: the stable predicate behind the loop

Recommended While format:

- `display\n@while: <expression>\n@condition_ref: <condition_ref>`

Example:

- `maxSampleCount not reached?\n@while: returned_count < maxSampleCount\n@condition_ref: returned_count < maxSampleCount`

#### RepeatWhile

Write a RepeatWhile condition in the same style as While.
The difference is only that the condition is evaluated after the loop body.

Each RepeatWhile should make the following information recoverable:

- display: the exact loop condition text shown in the diagram
- repeat_while or until: the logical post-condition of the loop
- condition_ref: the stable predicate behind the loop

Recommended RepeatWhile format:

- `display\n@repeat_while: <expression>\n@condition_ref: <condition_ref>`

Example:

- `maxSampleCount not reached?\n@repeat_while: returned_count < maxSampleCount\n@condition_ref: returned_count < maxSampleCount`

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

## Example: GetNewSamples Fragment

The example below is based on the following activity diagram:

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
