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

//! Collects API items diagram by diagram.

use std::collections::BTreeMap;

use class_diagram::{ClassDiagram, FreeFunctionDecl, SimpleEntity, Visibility};

use crate::naming::{method_part, name_key, record_name};
use crate::signature::{render_signature, render_template_parameters, SignatureParts};
use crate::{ApiError, ApiMethod};

struct Entry {
    method: ApiMethod,
    /// Declaration the item stems from (`ns::Cls::method`); overloads share it.
    origin: String,
}

/// Accumulates the API items of class diagrams: the public methods of every
/// entity and every free function.
#[derive(Default)]
pub struct ApiCollector {
    /// Items by [`name_key`] (TRLC's name uniqueness rule).
    entries: BTreeMap<String, Entry>,
}

impl ApiCollector {
    /// Adds the items of `model`, declared in the source file `diagram`.
    pub fn add_diagram(&mut self, diagram: &str, model: &ClassDiagram) -> Result<(), ApiError> {
        for entity in &model.entities {
            self.add_entity(diagram, entity)?;
        }
        for function in &model.free_functions {
            self.add_free_function(diagram, function)?;
        }
        Ok(())
    }

    /// The collected items, sorted by record name.
    pub fn into_methods(self) -> Vec<ApiMethod> {
        let mut methods: Vec<ApiMethod> = self.entries.into_values().map(|e| e.method).collect();
        methods.sort_by(|a, b| a.name.cmp(&b.name));
        methods
    }

    fn add_entity(&mut self, diagram: &str, entity: &SimpleEntity) -> Result<(), ApiError> {
        let scope = entity.id.replace('.', "::");
        let mut qualified_scope = scope.clone();
        if let Some(parameters) = &entity.template_parameters {
            qualified_scope.push_str(&render_template_parameters(parameters));
        }
        for method in entity
            .methods
            .iter()
            .filter(|m| m.visibility == Visibility::Public)
        {
            let origin = format!("{scope}::{}", method.name);
            let member = method_part(&method.name, &method.modifiers, &origin)?;
            let record = record_name(Some(&entity.id), &member, &origin)?;
            let signature = render_signature(&SignatureParts {
                qualified_name: &format!("{qualified_scope}::{}", method.name),
                return_type: method.return_type.as_deref(),
                parameters: &method.parameters,
                template_parameters: method.template_parameters.as_deref(),
                modifiers: &method.modifiers,
            });
            self.add(
                record,
                origin,
                signature,
                diagram,
                method.source_location.line,
            )?;
        }
        Ok(())
    }

    fn add_free_function(
        &mut self,
        diagram: &str,
        function: &FreeFunctionDecl,
    ) -> Result<(), ApiError> {
        let namespace = function
            .enclosing_namespace_id
            .as_deref()
            .filter(|ns| !ns.is_empty());
        let qualified = match namespace {
            Some(ns) => format!("{}::{}", ns.replace('.', "::"), function.name),
            None => function.name.clone(),
        };
        let record = record_name(namespace, &function.name, &qualified)?;
        let signature = render_signature(&SignatureParts {
            qualified_name: &qualified,
            return_type: function.return_type.as_deref(),
            parameters: &function.parameters,
            template_parameters: function.template_parameters.as_deref(),
            modifiers: &[],
        });
        self.add(
            record,
            qualified,
            signature,
            diagram,
            function.source_location.line,
        )
    }

    /// Adds `signature` to the item `record`, creating it on first sight.
    fn add(
        &mut self,
        record: String,
        origin: String,
        signature: String,
        diagram: &str,
        line: u32,
    ) -> Result<(), ApiError> {
        let key = name_key(&record);
        match self.entries.get_mut(&key) {
            None => {
                self.entries.insert(
                    key,
                    Entry {
                        method: ApiMethod {
                            name: record,
                            signatures: vec![signature],
                            diagram: diagram.to_string(),
                            line,
                        },
                        origin,
                    },
                );
            }
            Some(entry) if entry.origin == origin => {
                // const and non-const overloads yield equal signatures.
                if !entry.method.signatures.contains(&signature) {
                    entry.method.signatures.push(signature);
                }
            }
            Some(entry) => {
                return Err(ApiError::NameCollision {
                    record,
                    first: entry.origin.clone(),
                    first_location: format!("{}:{}", entry.method.diagram, entry.method.line),
                    second: origin,
                    second_location: format!("{diagram}:{line}"),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use class_diagram::{
        FunctionArgument, Method, MethodModifier, SourceLocation, TemplateParameter,
    };

    fn method(name: &str, line: u32) -> Method {
        Method {
            name: name.to_string(),
            source_location: SourceLocation::new("d.puml", line),
            ..Default::default()
        }
    }

    fn param(ty: &str) -> FunctionArgument {
        FunctionArgument {
            param_type: Some(ty.to_string()),
            ..Default::default()
        }
    }

    fn entity(id: &str, methods: Vec<Method>) -> SimpleEntity {
        SimpleEntity {
            id: id.to_string(),
            name: id.rsplit('.').next().unwrap().to_string(),
            methods,
            ..Default::default()
        }
    }

    fn diagram(entities: Vec<SimpleEntity>, free_functions: Vec<FreeFunctionDecl>) -> ClassDiagram {
        ClassDiagram {
            name: "d".to_string(),
            entities,
            free_functions,
        }
    }

    fn collect_all(diagrams: &[(&str, &ClassDiagram)]) -> Result<Vec<ApiMethod>, ApiError> {
        let mut collector = ApiCollector::default();
        for (file, model) in diagrams {
            collector.add_diagram(file, model)?;
        }
        Ok(collector.into_methods())
    }

    fn collect(model: &ClassDiagram) -> Result<Vec<ApiMethod>, ApiError> {
        collect_all(&[("pkg/d.puml", model)])
    }

    #[test]
    fn overloads_collapse_into_one_record_with_one_signature_each() {
        let mut overload = method("OfferService", 15);
        overload.return_type = Some("Result".to_string());
        overload.parameters = vec![param("int")];
        let mut first = method("OfferService", 14);
        first.return_type = Some("Result".to_string());
        let model = diagram(
            vec![entity("score.mw.Skeleton", vec![first, overload])],
            vec![],
        );

        let items = collect(&model).unwrap();

        assert_eq!(
            items,
            vec![ApiMethod {
                name: "score_mw_Skeleton_OfferService".to_string(),
                signatures: vec![
                    "Result score::mw::Skeleton::OfferService()".to_string(),
                    "Result score::mw::Skeleton::OfferService(int)".to_string(),
                ],
                diagram: "pkg/d.puml".to_string(),
                line: 14,
            }]
        );
    }

    #[test]
    fn identical_signatures_are_listed_once() {
        let model = diagram(
            vec![entity("A", vec![method("f", 3), method("f", 4)])],
            vec![],
        );

        let items = collect(&model).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].signatures, vec!["A::f()".to_string()]);
    }

    #[test]
    fn non_public_methods_are_skipped() {
        let mut private = method("hidden", 3);
        private.visibility = Visibility::Private;
        let mut protected = method("guarded", 4);
        protected.visibility = Visibility::Protected;
        let model = diagram(
            vec![entity("A", vec![private, protected, method("shown", 5)])],
            vec![],
        );

        let names: Vec<_> = collect(&model)
            .unwrap()
            .into_iter()
            .map(|m| m.name)
            .collect();

        assert_eq!(names, vec!["A_shown".to_string()]);
    }

    #[test]
    fn entities_without_methods_emit_nothing() {
        let model = diagram(vec![entity("A", vec![])], vec![]);

        assert!(collect(&model).unwrap().is_empty());
    }

    #[test]
    fn operators_use_the_operator_table() {
        let model = diagram(
            vec![entity(
                "ns.A",
                vec![
                    method("operator==", 3),
                    method("operator[]", 4),
                    method("operator()", 5),
                    method("operator&&", 6),
                    method("operator&", 7),
                ],
            )],
            vec![],
        );

        let names: Vec<_> = collect(&model)
            .unwrap()
            .into_iter()
            .map(|m| m.name)
            .collect();

        assert_eq!(
            names,
            vec![
                "ns_A_operator_and",
                "ns_A_operator_bitand",
                "ns_A_operator_call",
                "ns_A_operator_eq",
                "ns_A_operator_subscript",
            ]
        );
    }

    #[test]
    fn unknown_operator_is_an_error() {
        let model = diagram(vec![entity("A", vec![method("operator@", 3)])], vec![]);

        assert_eq!(
            collect(&model),
            Err(ApiError::UnsupportedOperator {
                origin: "A::operator@".to_string(),
                operator: "@".to_string(),
            })
        );
    }

    #[test]
    fn operator_prefixed_identifier_is_a_plain_method() {
        let model = diagram(vec![entity("A", vec![method("operatorX", 3)])], vec![]);

        assert_eq!(collect(&model).unwrap()[0].name, "A_operatorX");
    }

    #[test]
    fn constructor_uses_class_name_and_destructor_is_named_destructor() {
        let mut ctor = method("A", 3);
        ctor.modifiers = vec![MethodModifier::Constructor];
        let mut dtor = method("~A", 4);
        dtor.modifiers = vec![MethodModifier::Destructor];
        let model = diagram(vec![entity("ns.A", vec![ctor, dtor])], vec![]);

        let items = collect(&model).unwrap();

        assert_eq!(items[0].name, "ns_A_A");
        assert_eq!(items[1].name, "ns_A_destructor");
        assert_eq!(items[1].signatures, vec!["ns::A::~A()".to_string()]);
    }

    #[test]
    fn static_noexcept_and_template_are_part_of_the_signature() {
        let mut m = method("Create", 3);
        m.return_type = Some("A".to_string());
        m.modifiers = vec![MethodModifier::Static, MethodModifier::Noexcept];
        m.template_parameters = Some(vec![
            TemplateParameter::Type {
                name: "T".to_string(),
                is_pack: false,
            },
            TemplateParameter::Type {
                name: "Ts".to_string(),
                is_pack: true,
            },
        ]);
        m.parameters = vec![FunctionArgument {
            name: "args".to_string(),
            param_type: Some("Ts&&".to_string()),
            is_pack_expansion: true,
            ..Default::default()
        }];
        let model = diagram(vec![entity("A", vec![m])], vec![]);

        assert_eq!(
            collect(&model).unwrap()[0].signatures,
            vec!["static A A::Create<T, Ts...>(Ts&&... args) noexcept".to_string()]
        );
    }

    #[test]
    fn entity_template_parameters_are_part_of_the_signature() {
        let mut cls = entity("ns.Box", vec![method("Get", 3)]);
        cls.template_parameters = Some(vec![
            TemplateParameter::Type {
                name: "T".to_string(),
                is_pack: false,
            },
            TemplateParameter::NonType {
                name: "N".to_string(),
                value_type: "int".to_string(),
                is_pack: false,
            },
        ]);
        let model = diagram(vec![cls], vec![]);

        let items = collect(&model).unwrap();

        assert_eq!(items[0].name, "ns_Box_Get");
        assert_eq!(
            items[0].signatures,
            vec!["ns::Box<T, N>::Get()".to_string()]
        );
    }

    #[test]
    fn c_variadic_parameter_is_rendered() {
        let mut m = method("log", 3);
        m.parameters = vec![
            param("const char*"),
            FunctionArgument {
                is_variadic: true,
                ..Default::default()
            },
        ];
        let model = diagram(vec![entity("A", vec![m])], vec![]);

        assert_eq!(
            collect(&model).unwrap()[0].signatures,
            vec!["A::log(const char*, ...)".to_string()]
        );
    }

    #[test]
    fn free_functions_are_named_after_their_namespace() {
        let model = diagram(
            vec![],
            vec![
                FreeFunctionDecl {
                    name: "clamp".to_string(),
                    enclosing_namespace_id: Some("amp.algorithm".to_string()),
                    return_type: Some("int".to_string()),
                    parameters: vec![param("int")],
                    source_location: SourceLocation::new("d.puml", 7),
                    ..Default::default()
                },
                FreeFunctionDecl {
                    name: "main".to_string(),
                    source_location: SourceLocation::new("d.puml", 9),
                    ..Default::default()
                },
            ],
        );

        let items = collect(&model).unwrap();

        assert_eq!(items[0].name, "Free_main");
        assert_eq!(items[0].signatures, vec!["main()".to_string()]);
        assert_eq!(items[1].name, "amp_algorithm_clamp");
        assert_eq!(
            items[1].signatures,
            vec!["int amp::algorithm::clamp(int)".to_string()]
        );
        assert_eq!(items[1].line, 7);
    }

    #[test]
    fn free_function_overloads_collapse() {
        let overload = |line| FreeFunctionDecl {
            name: "f".to_string(),
            enclosing_namespace_id: Some("ns".to_string()),
            source_location: SourceLocation::new("d.puml", line),
            ..Default::default()
        };
        let mut second = overload(8);
        second.parameters = vec![param("int")];
        let model = diagram(vec![], vec![overload(7), second]);

        let items = collect(&model).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].signatures.len(), 2);
    }

    #[test]
    fn names_differing_only_in_case_or_underscores_collide() {
        let model = diagram(
            vec![entity("A", vec![method("GetX", 3), method("get_x", 4)])],
            vec![],
        );

        let error = collect(&model).unwrap_err();

        assert!(matches!(error, ApiError::NameCollision { .. }), "{error}");
        assert!(error.to_string().contains("A_get_x"));
    }

    #[test]
    fn different_origins_mapping_to_one_record_collide() {
        let model = diagram(
            vec![
                entity("ns.A", vec![method("b_c", 3)]),
                entity("ns.A_b", vec![method("c", 4)]),
            ],
            vec![],
        );

        assert!(matches!(
            collect(&model),
            Err(ApiError::NameCollision { .. })
        ));
    }

    #[test]
    fn invalid_identifier_characters_are_an_error() {
        let model = diagram(vec![entity("ns.Foo<T>", vec![method("f", 3)])], vec![]);

        assert!(matches!(collect(&model), Err(ApiError::InvalidName { .. })));
    }

    #[test]
    fn leading_underscore_is_an_error() {
        let model = diagram(vec![entity("_A", vec![method("f", 3)])], vec![]);

        assert!(matches!(collect(&model), Err(ApiError::InvalidName { .. })));
    }

    #[test]
    fn qualified_method_names_are_joined_with_underscores() {
        let model = diagram(vec![entity("A", vec![method("ns::f", 3)])], vec![]);

        assert_eq!(collect(&model).unwrap()[0].name, "A_ns_f");
    }

    #[test]
    fn items_of_several_diagrams_are_merged_and_sorted() {
        let a = diagram(vec![entity("B", vec![method("f", 3)])], vec![]);
        let b = diagram(vec![entity("A", vec![method("f", 4)])], vec![]);

        let items = collect_all(&[("a.puml", &a), ("b.puml", &b)]).unwrap();

        let located: Vec<_> = items
            .iter()
            .map(|m| (m.name.as_str(), m.diagram.as_str()))
            .collect();
        assert_eq!(located, vec![("A_f", "b.puml"), ("B_f", "a.puml")]);
    }
}
