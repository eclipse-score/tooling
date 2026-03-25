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
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use component_parser::PumlComponentParser;
use parser_core::DiagramParser;
use puml_utils::LogLevel;

fn test_file_with_golden(path: &str, golden: &str) -> Result<(), String> {
    let mut parser = PumlComponentParser;

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read test puml file {}: {}", path, e))?;

    let ast = parser
        .parse_file(&Rc::new(PathBuf::from(path)), &content, LogLevel::Error)
        .map_err(|e| format!("Parse failed for {}: {:?}", path, e))?;

    let ast_str = format!("{:#?}", ast);

    if ast_str != golden {
        return Err(format!(
            "Golden test failed for {}.\nExpected:\n{}\n\nFound:\n{}",
            path, golden, ast_str
        ));
    }

    Ok(())
}

#[test]
fn test_component_golden() {
    let mut golden_map: HashMap<&str, &str> = HashMap::new();

    golden_map.insert(
        "plantuml/parser/integration_test/component_diagram/plantuml/basic_example.puml",
        r#"CompPumlDocument {
    name: None,
    statements: [
        Relation(
            Relation {
                lhs: "DataAccess",
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "-",
                    },
                    middle: None,
                    right: None,
                },
                rhs: "[First Component]",
                style: None,
                description: None,
            },
        ),
        Relation(
            Relation {
                lhs: "[First Component]",
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "..",
                    },
                    middle: None,
                    right: Some(
                        ArrowDecor {
                            raw: ">",
                        },
                    ),
                },
                rhs: "HTTP",
                style: None,
                description: Some(
                    "use",
                ),
            },
        ),
    ],
}"#,
    );

    golden_map.insert(
        "plantuml/parser/integration_test/component_diagram/plantuml/changing_arrows_direction.puml",
        r#"CompPumlDocument {
    name: None,
    statements: [
        Relation(
            Relation {
                lhs: "[Component]",
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "--",
                    },
                    middle: None,
                    right: Some(
                        ArrowDecor {
                            raw: ">",
                        },
                    ),
                },
                rhs: "Interface1",
                style: None,
                description: None,
            },
        ),
        Relation(
            Relation {
                lhs: "[Component]",
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "-",
                    },
                    middle: None,
                    right: Some(
                        ArrowDecor {
                            raw: ">",
                        },
                    ),
                },
                rhs: "Interface2",
                style: None,
                description: None,
            },
        ),
    ],
}"#,
    );

    golden_map.insert(
        "plantuml/parser/integration_test/component_diagram/plantuml/changing_arrows_direction3.puml",
        r#"CompPumlDocument {
    name: None,
    statements: [
        Relation(
            Relation {
                lhs: "[Component]",
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "--",
                    },
                    middle: Some(
                        ArrowMiddle {
                            style: None,
                            direction: Some(
                                Left,
                            ),
                            decorator: None,
                        },
                    ),
                    right: Some(
                        ArrowDecor {
                            raw: ">",
                        },
                    ),
                },
                rhs: "left",
                style: None,
                description: None,
            },
        ),
        Relation(
            Relation {
                lhs: "[Component]",
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "--",
                    },
                    middle: Some(
                        ArrowMiddle {
                            style: None,
                            direction: Some(
                                Right,
                            ),
                            decorator: None,
                        },
                    ),
                    right: Some(
                        ArrowDecor {
                            raw: ">",
                        },
                    ),
                },
                rhs: "right",
                style: None,
                description: None,
            },
        ),
        Relation(
            Relation {
                lhs: "[Component]",
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "--",
                    },
                    middle: Some(
                        ArrowMiddle {
                            style: None,
                            direction: Some(
                                Up,
                            ),
                            decorator: None,
                        },
                    ),
                    right: Some(
                        ArrowDecor {
                            raw: ">",
                        },
                    ),
                },
                rhs: "up",
                style: None,
                description: None,
            },
        ),
        Relation(
            Relation {
                lhs: "[Component]",
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "--",
                    },
                    middle: Some(
                        ArrowMiddle {
                            style: None,
                            direction: Some(
                                Down,
                            ),
                            decorator: None,
                        },
                    ),
                    right: Some(
                        ArrowDecor {
                            raw: ">",
                        },
                    ),
                },
                rhs: "down",
                style: None,
                description: None,
            },
        ),
    ],
}"#,
    );

    golden_map.insert(
        "plantuml/parser/integration_test/component_diagram/plantuml/component.puml",
        r#"CompPumlDocument {
    name: None,
    statements: [
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "First component",
                ),
                alias: None,
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "Another component",
                ),
                alias: Some(
                    "Comp2",
                ),
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "Comp3",
                ),
                alias: None,
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "Last\\ncomponent",
                ),
                alias: Some(
                    "Comp4",
                ),
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
    ],
}"#,
    );

    golden_map.insert(
        "plantuml/parser/integration_test/component_diagram/plantuml/grouping_components.puml",
        r#"CompPumlDocument {
    name: None,
    statements: [
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "First component",
                ),
                alias: None,
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "Another component",
                ),
                alias: Some(
                    "Comp2",
                ),
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "Comp3",
                ),
                alias: None,
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "Last\\ncomponent",
                ),
                alias: Some(
                    "Comp4",
                ),
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
    ],
}"#,
    );

    golden_map.insert(
        "plantuml/parser/integration_test/component_diagram/plantuml/hide_or_remove_unlinked_component.puml",
        r#"CompPumlDocument {
    name: None,
    statements: [
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "C1",
                ),
                alias: None,
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "C2",
                ),
                alias: None,
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "C3",
                ),
                alias: None,
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Relation(
            Relation {
                lhs: "C1",
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "--",
                    },
                    middle: None,
                    right: None,
                },
                rhs: "C2",
                style: None,
                description: None,
            },
        ),
    ],
}"#,
    );

    golden_map.insert(
        "plantuml/parser/integration_test/component_diagram/plantuml/individual_colors.puml",
        r#"CompPumlDocument {
    name: None,
    statements: [
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "Web Server",
                ),
                alias: None,
                stereotype: None,
                style: Some(
                    ComponentStyle {
                        color: None,
                        attributes: [],
                    },
                ),
                statements: [],
            },
        ),
    ],
}"#,
    );

    golden_map.insert(
        "plantuml/parser/integration_test/component_diagram/plantuml/interfaces.puml",
        r#"CompPumlDocument {
    name: None,
    statements: [
        Component(
            Component {
                component_type: "interface",
                name: Some(
                    "\"First Interface\"",
                ),
                alias: None,
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "interface",
                name: Some(
                    "\"Another interface\"",
                ),
                alias: Some(
                    "Interf2",
                ),
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "interface",
                name: Some(
                    "Interf3",
                ),
                alias: None,
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "interface",
                name: Some(
                    "Last\\ninterface",
                ),
                alias: Some(
                    "Interf4",
                ),
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "component",
                ),
                alias: None,
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
    ],
}"#,
    );

    golden_map.insert(
        "plantuml/parser/integration_test/component_diagram/plantuml/long_description.puml",
        r#"CompPumlDocument {
    name: None,
    statements: [
        Component(
            Component {
                component_type: "component",
                name: Some(
                    "comp1",
                ),
                alias: None,
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
    ],
}"#,
    );

    golden_map.insert(
        "plantuml/parser/integration_test/component_diagram/plantuml/use_uml2_notation.puml",
        r#"CompPumlDocument {
    name: None,
    statements: [
        Component(
            Component {
                component_type: "interface",
                name: Some(
                    "Data Access",
                ),
                alias: Some(
                    "DA",
                ),
                stereotype: None,
                style: None,
                statements: [],
            },
        ),
        Relation(
            Relation {
                lhs: "DA",
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "-",
                    },
                    middle: None,
                    right: None,
                },
                rhs: "[First Component]",
                style: None,
                description: None,
            },
        ),
        Relation(
            Relation {
                lhs: "[First Component]",
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "..",
                    },
                    middle: None,
                    right: Some(
                        ArrowDecor {
                            raw: ">",
                        },
                    ),
                },
                rhs: "HTTP",
                style: None,
                description: Some(
                    "use",
                ),
            },
        ),
    ],
}"#,
    );

    let mut passed = 0;
    let mut failed = 0;

    for (file, golden) in golden_map.iter() {
        match test_file_with_golden(file, golden) {
            Ok(_) => {
                passed += 1;
                println!("✔ PASS {}", file);
            }
            Err(e) => {
                failed += 1;
                println!("✘ FAIL {}", file);
                println!("{}", e);
            }
        }
    }

    println!("Passed: {}, Failed: {}", passed, failed);
    assert_eq!(failed, 0, "Some component golden tests failed");
}
