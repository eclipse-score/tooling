// *******************************************************************************
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// <https://www.apache.org/licenses/LICENSE-2.0>
//
// SPDX-License-Identifier: Apache-2.0
// *******************************************************************************

//! Models for sequence-diagram FlatBuffer inputs used by design verification.

use std::collections::BTreeMap;

use sequence_logic::{
    Block, Interaction, MessageKind, Node, ParticipantType, SequenceParticipant, SequenceTree,
    SourceLocation,
};

use crate::{ErrorBuilder, ErrorCategory, ValidationResult};

/// Collection of sequence diagrams loaded from one or more FlatBuffer files.
pub struct SequenceDiagramInputs {
    pub diagrams: Vec<SequenceTree>,
}

/// Stereotype that marks a participant as a platform or another SEooC
/// (compared case-insensitively).
const EXTERNAL_STEREOTYPE: &str = "external";

/// Label of the unspecified environment behind a gate or lost/found message.
pub const GATE_LABEL: &str = "<gate>";

/// The environment of the described architecture, as declared in the diagram.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
    /// An application using the SEooC (`actor`).
    Actor,
    /// A platform or another SEooC (`participant X <<external>>`).
    External,
    /// An unspecified environment (gate, lost or found message).
    Gate,
}

fn participant_environment(participant: &SequenceParticipant) -> Option<Environment> {
    if participant.participant_type == ParticipantType::Actor {
        Some(Environment::Actor)
    } else if participant
        .stereotype
        .as_deref()
        .is_some_and(|stereotype| stereotype.eq_ignore_ascii_case(EXTERNAL_STEREOTYPE))
    {
        Some(Environment::External)
    } else {
        None
    }
}

impl Environment {
    pub fn label(self) -> &'static str {
        match self {
            Environment::Actor => "an actor",
            Environment::External => "an <<external>> participant",
            Environment::Gate => "a gate",
        }
    }
}

/// One operation call (synchronous or asynchronous message) observed in a
/// sequence diagram. Replies, create and delete messages are not observed.
pub struct ObservedSequenceCall {
    /// Participant uid, or [`GATE_LABEL`] for a gate.
    pub caller: String,
    pub callee: String,
    /// `None` for an architecture element or class lifeline.
    pub caller_environment: Option<Environment>,
    pub callee_environment: Option<Environment>,
    pub method: String,
    pub source_location: SourceLocation,
}

/// A reply, create or delete message. It is not an operation call and is
/// only kept for diagnostics.
pub struct SkippedSequenceMessage {
    /// Participant uid, or [`GATE_LABEL`] for a gate.
    pub sender: String,
    pub receiver: String,
    pub kind: MessageKind,
    pub message: String,
}

/// Validation-only participant metadata keyed by the participant uid used in
/// sequence interactions.
pub struct SequenceParticipantInfo {
    pub source_location: SourceLocation,
    /// `Some` for an `actor` or `<<external>>` participant.
    pub environment: Option<Environment>,
}

impl SequenceDiagramInputs {
    /// Build a [`SequenceDiagramIndex`] from sequence diagram inputs.
    pub fn to_sequence_diagram_index(&self, result: &mut ValidationResult) -> SequenceDiagramIndex {
        SequenceDiagramIndex::from_diagrams(&self.diagrams, result)
    }
}

/// Indexed sequence-diagram data prepared for validators.
pub struct SequenceDiagramIndex {
    participants: BTreeMap<String, SequenceParticipantInfo>,
    observed_calls: Vec<ObservedSequenceCall>,
    skipped_messages: Vec<SkippedSequenceMessage>,
}

impl SequenceDiagramIndex {
    /// An actor or `<<external>>` declaration in any diagram applies to the
    /// uid in every diagram; conflicting declarations are reported.
    fn from_diagrams(diagrams: &[SequenceTree], result: &mut ValidationResult) -> Self {
        let mut participants: BTreeMap<String, SequenceParticipantInfo> = BTreeMap::new();
        let mut declared: BTreeMap<String, (Environment, SourceLocation)> = BTreeMap::new();

        for diagram in diagrams {
            for participant in &diagram.participants {
                if participant.uid.is_empty() {
                    result.add_failure(empty_uid_error(participant));
                    continue;
                }

                if let Some(environment) = participant_environment(participant) {
                    match declared.get(&participant.uid) {
                        Some((first, first_location)) if *first != environment => {
                            result.add_failure(conflicting_environment_error(
                                participant,
                                environment,
                                *first,
                                first_location,
                            ));
                        }
                        Some(_) => {}
                        None => {
                            declared.insert(
                                participant.uid.clone(),
                                (environment, participant.source_location.clone()),
                            );
                        }
                    }
                }

                // Keep the first declaration location when a participant is
                // declared in more than one input diagram.
                participants
                    .entry(participant.uid.clone())
                    .or_insert_with(|| SequenceParticipantInfo {
                        source_location: participant.source_location.clone(),
                        environment: None,
                    });
            }
        }

        let mut environments: BTreeMap<&str, Environment> = BTreeMap::new();
        for (uid, (environment, _)) in &declared {
            environments.insert(uid.as_str(), *environment);
            if let Some(info) = participants.get_mut(uid) {
                info.environment = Some(*environment);
            }
        }

        let mut collector = MessageCollector {
            environments: &environments,
            calls: Vec::new(),
            skipped: Vec::new(),
        };
        for diagram in diagrams {
            collector.block(&diagram.root);
        }
        let MessageCollector { calls, skipped, .. } = collector;

        Self {
            participants,
            observed_calls: calls,
            skipped_messages: skipped,
        }
    }

    pub fn participants(&self) -> &BTreeMap<String, SequenceParticipantInfo> {
        &self.participants
    }

    pub fn declared_participants(&self) -> impl Iterator<Item = &str> {
        self.participants.keys().map(String::as_str)
    }

    pub fn observed_calls(&self) -> &[ObservedSequenceCall] {
        &self.observed_calls
    }

    pub fn skipped_messages(&self) -> &[SkippedSequenceMessage] {
        &self.skipped_messages
    }
}

fn conflicting_environment_error(
    participant: &SequenceParticipant,
    environment: Environment,
    first_environment: Environment,
    first_location: &SourceLocation,
) -> String {
    let (source_file, source_line) = participant.source_location.display();
    let (first_file, first_line) = first_location.display();
    let uid = &participant.uid;

    ErrorBuilder::new(ErrorCategory::Naming)
        .title(format!(
            "sequence participant \"{uid}\" is declared as {} and as {}",
            first_environment.label(),
            environment.label()
        ))
        .field("participant", format!("\"{uid}\""))
        .field("first declaration source file", format!("\"{first_file}\""))
        .field("first declaration source line", first_line.to_string())
        .field("conflicting source file", format!("\"{source_file}\""))
        .field("conflicting source line", source_line.to_string())
        .fix(format!(
            "declare participant \"{uid}\" with the same kind (actor or <<external>>) in every sequence diagram, or rename one of them"
        ))
        .build()
}

fn empty_uid_error(participant: &SequenceParticipant) -> String {
    let (source_file, source_line) = participant.source_location.display();

    ErrorBuilder::new(ErrorCategory::Naming)
        .title(format!(
            "sequence participant \"{}\" has an empty uid",
            participant.display_name
        ))
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string())
        .fix("regenerate the sequence diagram so that every participant carries a uid")
        .build()
}

struct MessageCollector<'a> {
    environments: &'a BTreeMap<&'a str, Environment>,
    calls: Vec<ObservedSequenceCall>,
    skipped: Vec<SkippedSequenceMessage>,
}

impl MessageCollector<'_> {
    fn block(&mut self, block: &Block) {
        for node in &block.items {
            self.node(node);
        }
    }

    fn node(&mut self, node: &Node) {
        match node {
            Node::Interaction(interaction) => self.interaction(interaction),
            Node::Branch(branch) => {
                for case in &branch.cases {
                    self.block(&case.block);
                }
            }
            Node::Loop(loop_node) => self.block(&loop_node.block),
            Node::Parallel(parallel) => {
                for branch in &parallel.branches {
                    self.block(&branch.block);
                }
            }
            Node::EarlyExit(early_exit) => self.block(&early_exit.block),
            Node::Lifecycle(_) | Node::Reference(_) => {}
        }
    }

    fn interaction(&mut self, interaction: &Interaction) {
        let endpoint = |participant: Option<&str>| match participant {
            Some(uid) => (uid.to_string(), self.environments.get(uid).copied()),
            None => (GATE_LABEL.to_string(), Some(Environment::Gate)),
        };
        let (caller, caller_environment) = endpoint(interaction.sender.as_deref());
        let (callee, callee_environment) = endpoint(interaction.receiver.as_deref());
        let method = interaction.message.clone().unwrap_or_default();

        // Replies, create and delete messages are not operation calls.
        if !matches!(
            interaction.kind,
            MessageKind::SynchCall | MessageKind::AsynchCall
        ) {
            self.skipped.push(SkippedSequenceMessage {
                sender: caller,
                receiver: callee,
                kind: interaction.kind,
                message: method,
            });
            return;
        }

        self.calls.push(ObservedSequenceCall {
            caller,
            callee,
            caller_environment,
            callee_environment,
            method,
            source_location: interaction.source_location.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validators::fixtures::dummy_source_location;
    use sequence_logic::{Branch, BranchCase, Interaction, ParticipantType};

    fn interaction(caller: Option<&str>, callee: Option<&str>, method: &str) -> Node {
        interaction_of_kind(caller, callee, method, MessageKind::SynchCall)
    }

    fn interaction_of_kind(
        caller: Option<&str>,
        callee: Option<&str>,
        method: &str,
        kind: MessageKind,
    ) -> Node {
        Node::Interaction(Interaction {
            sender: caller.map(|caller| caller.to_string().into()),
            receiver: callee.map(|callee| callee.to_string().into()),
            message: Some(method.to_string()),
            kind,
            source_location: dummy_source_location(),
        })
    }

    fn branch(items: Vec<Node>) -> Node {
        Node::Branch(Branch {
            cases: vec![BranchCase {
                condition: Some("case".to_string()),
                block: Block { items },
                source_location: dummy_source_location(),
            }],
        })
    }

    #[test]
    fn sequence_index_collects_nested_calls_recursively() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: Vec::new(),
                root: Block {
                    items: vec![
                        interaction(Some("unit_1"), Some("unit_2"), "GetData()"),
                        branch(vec![interaction(
                            Some("unit_2"),
                            Some("unit_3"),
                            "Forward()",
                        )]),
                    ],
                },
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert!(result.is_empty());
        assert_eq!(index.observed_calls().len(), 2);
        assert_eq!(index.observed_calls()[0].caller, "unit_1");
        assert_eq!(index.observed_calls()[0].callee, "unit_2");
        assert_eq!(index.observed_calls()[0].method, "GetData()");
        assert_eq!(index.observed_calls()[1].caller, "unit_2");
        assert_eq!(index.observed_calls()[1].callee, "unit_3");
        assert_eq!(index.observed_calls()[1].method, "Forward()");
    }

    fn participant(display_name: &str, alias: Option<&str>, uid: &str) -> SequenceParticipant {
        SequenceParticipant {
            display_name: display_name.to_string(),
            alias: alias.map(str::to_string),
            uid: uid.to_string(),
            participant_type: ParticipantType::Participant,
            source_location: dummy_source_location(),
            stereotype: None,
        }
    }

    #[test]
    fn sequence_index_keys_participants_by_uid() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: vec![
                    participant("Unit 1", Some("unit_1"), "unit_1"),
                    participant("comp::unit_2", Some("u2"), "comp.unit_2"),
                ],
                root: Block::default(),
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert!(result.is_empty());
        assert_eq!(
            index.declared_participants().collect::<Vec<_>>(),
            vec!["comp.unit_2", "unit_1"]
        );
    }

    #[test]
    fn sequence_index_reports_participant_with_empty_uid() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: vec![participant("Unit 1", Some("unit_1"), "")],
                root: Block::default(),
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("has an empty uid"));
        assert_eq!(index.declared_participants().count(), 0);
    }

    #[test]
    fn sequence_index_maps_missing_caller_to_gate() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: Vec::new(),
                root: Block {
                    items: vec![interaction(None, Some("unit_2"), "GetData()")],
                },
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert!(result.is_empty());
        assert_eq!(index.observed_calls()[0].caller, GATE_LABEL);
        assert_eq!(
            index.observed_calls()[0].caller_environment,
            Some(Environment::Gate)
        );
        assert_eq!(index.observed_calls()[0].callee, "unit_2");
        assert_eq!(index.observed_calls()[0].callee_environment, None);
    }

    #[test]
    fn sequence_index_maps_missing_callee_to_gate() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: Vec::new(),
                root: Block {
                    items: vec![interaction(Some("unit_1"), None, "GetData()")],
                },
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert!(result.is_empty());
        assert_eq!(index.observed_calls()[0].caller, "unit_1");
        assert_eq!(index.observed_calls()[0].callee, GATE_LABEL);
        assert_eq!(
            index.observed_calls()[0].callee_environment,
            Some(Environment::Gate)
        );
    }

    #[test]
    fn sequence_index_classifies_actor_and_external_participants() {
        let mut external = participant("Platform", None, "platform");
        external.stereotype = Some("external".to_string());
        let mut actor = participant("App", None, "app");
        actor.participant_type = ParticipantType::Actor;

        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: vec![actor, external, participant("Unit 1", None, "unit_1")],
                root: Block {
                    items: vec![
                        interaction(Some("app"), Some("unit_1"), "Run()"),
                        interaction(Some("unit_1"), Some("platform"), "Send()"),
                    ],
                },
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert!(result.is_empty());
        let calls = index.observed_calls();
        assert_eq!(calls[0].caller_environment, Some(Environment::Actor));
        assert_eq!(calls[0].callee_environment, None);
        assert_eq!(calls[1].callee_environment, Some(Environment::External));
        assert_eq!(
            index.participants()["app"].environment,
            Some(Environment::Actor)
        );
        assert_eq!(index.participants()["unit_1"].environment, None);
    }

    #[test]
    fn sequence_index_applies_environment_declared_in_another_diagram() {
        let mut actor = participant("App", None, "app");
        actor.participant_type = ParticipantType::Actor;
        let diagram = |participants, items| SequenceTree {
            name: Some("seq".to_string()),
            participants,
            root: Block { items },
        };
        let inputs = SequenceDiagramInputs {
            diagrams: vec![
                diagram(vec![actor], Vec::new()),
                diagram(
                    vec![participant("App", None, "app")],
                    vec![interaction(Some("app"), Some("unit_1"), "Run()")],
                ),
            ],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert!(result.is_empty(), "{:?}", result.failures);
        assert_eq!(
            index.observed_calls()[0].caller_environment,
            Some(Environment::Actor)
        );
        assert_eq!(
            index.participants()["app"].environment,
            Some(Environment::Actor)
        );
    }

    #[test]
    fn sequence_index_reports_conflicting_environment_declarations() {
        let mut actor = participant("App", None, "app");
        actor.participant_type = ParticipantType::Actor;
        let mut external = participant("App", None, "app");
        external.stereotype = Some("external".to_string());
        let diagram = |participant| SequenceTree {
            name: Some("seq".to_string()),
            participants: vec![participant],
            root: Block::default(),
        };
        let inputs = SequenceDiagramInputs {
            diagrams: vec![diagram(actor), diagram(external)],
        };

        let mut result = ValidationResult::default();
        inputs.to_sequence_diagram_index(&mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0]
            .contains("\"app\" is declared as an actor and as an <<external>> participant"));
    }

    #[test]
    fn sequence_index_observes_only_operation_calls() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: Vec::new(),
                root: Block {
                    items: vec![
                        interaction_of_kind(Some("a"), Some("b"), "Sync()", MessageKind::SynchCall),
                        interaction_of_kind(
                            Some("a"),
                            Some("b"),
                            "Async()",
                            MessageKind::AsynchCall,
                        ),
                        interaction_of_kind(Some("b"), Some("a"), "ok", MessageKind::Reply),
                        interaction_of_kind(Some("a"), Some("c"), "new", MessageKind::Create),
                        interaction_of_kind(Some("a"), Some("c"), "del", MessageKind::Delete),
                    ],
                },
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        let methods: Vec<&str> = index
            .observed_calls()
            .iter()
            .map(|call| call.method.as_str())
            .collect();
        assert_eq!(methods, vec!["Sync()", "Async()"]);

        let skipped: Vec<(&str, MessageKind)> = index
            .skipped_messages()
            .iter()
            .map(|message| (message.message.as_str(), message.kind))
            .collect();
        assert_eq!(
            skipped,
            vec![
                ("ok", MessageKind::Reply),
                ("new", MessageKind::Create),
                ("del", MessageKind::Delete)
            ]
        );
    }

    #[test]
    fn external_stereotype_is_case_insensitive() {
        let mut external = participant("Platform", None, "platform");
        external.stereotype = Some("External".to_string());

        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: vec![external],
                root: Block::default(),
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert_eq!(
            index.participants()["platform"].environment,
            Some(Environment::External)
        );
    }
}
