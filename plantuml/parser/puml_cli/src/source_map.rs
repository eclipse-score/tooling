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

//! Source files and lines of an include-expanded diagram.

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};
use std::path::Path;

use puml_parser::SourceLine;

use crate::parsing::ParsedDiagram;

/// Where each line of an include-expanded diagram was written.
pub struct SourceMap {
    /// File and line per line of the expanded text; empty when every line is
    /// where the parser saw it.
    lines: Vec<(String, u32)>,
}

impl SourceMap {
    /// Maps nothing.
    pub fn identity() -> Self {
        Self { lines: Vec::new() }
    }

    /// The map of the expanded text of `entry`, whose lines come from
    /// `origins`; `name_of` names a file.
    pub fn new(entry: &Path, origins: Vec<SourceLine>, name_of: impl Fn(&Path) -> String) -> Self {
        let unchanged = origins
            .iter()
            .zip(1u32..)
            .all(|(origin, line)| origin.file.as_path() == entry && origin.line == line);
        if unchanged {
            return Self::identity();
        }

        Self {
            lines: origins
                .into_iter()
                .map(|origin| (name_of(&origin.file), origin.line))
                .collect(),
        }
    }

    /// File and line of the 1-based `line` of the expanded text.
    pub fn locate(&self, line: usize) -> Option<(&str, u32)> {
        let (file, line) = self.lines.get(line.checked_sub(1)?)?;
        Some((file, *line))
    }

    /// Rewrites the source locations in `parsed`, which the parser stamped
    /// with `parser_file` and the line of the expanded text.
    pub fn apply(
        &self,
        parsed: &mut ParsedDiagram,
        parser_file: &str,
    ) -> Result<(), serde_json::Error> {
        if self.lines.is_empty() {
            return Ok(());
        }
        match parsed {
            // Activity diagrams carry no source locations.
            ParsedDiagram::Activity(_) => Ok(()),
            ParsedDiagram::Component(document) => self.apply_to(document, parser_file),
            ParsedDiagram::Class(document) => self.apply_to(document, parser_file),
            ParsedDiagram::Sequence(document) => self.apply_to(document, parser_file),
        }
    }

    fn apply_to<T: Serialize + DeserializeOwned>(
        &self,
        document: &mut T,
        parser_file: &str,
    ) -> Result<(), serde_json::Error> {
        let mut json = serde_json::to_value(&*document)?;
        self.rewrite(&mut json, parser_file);
        *document = serde_json::from_value(json)?;
        Ok(())
    }

    fn rewrite(&self, value: &mut Value, parser_file: &str) {
        match value {
            Value::Object(object) => match self.relocated(object, parser_file) {
                Some((file, line)) => {
                    object.insert("file".into(), Value::from(file));
                    object.insert("line".into(), Value::from(line));
                }
                None => object
                    .values_mut()
                    .for_each(|member| self.rewrite(member, parser_file)),
            },
            Value::Array(items) => items
                .iter_mut()
                .for_each(|item| self.rewrite(item, parser_file)),
            _ => {}
        }
    }

    /// The new file and line of `object` if it is a `SourceLocation` stamped
    /// by the parser, i.e. exactly `{file, line}` with `file == parser_file`.
    fn relocated(&self, object: &Map<String, Value>, parser_file: &str) -> Option<(String, u32)> {
        if object.len() != 2 || object.get("file")?.as_str()? != parser_file {
            return None;
        }
        let line = usize::try_from(object.get("line")?.as_u64()?).ok()?;
        let (file, line) = self.locate(line)?;
        Some((file.to_string(), line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use source_location::SourceLocation;
    use std::path::PathBuf;
    use std::rc::Rc;

    fn line(file: &str, line: u32) -> SourceLine {
        SourceLine {
            file: Rc::new(PathBuf::from(file)),
            line,
        }
    }

    // `SourceMap::relocated` recognises a location by exactly these two keys.
    #[test]
    fn source_location_serializes_to_exactly_file_and_line() {
        assert_eq!(
            serde_json::to_value(SourceLocation::new("a.puml", 3)).unwrap(),
            serde_json::json!({"file": "a.puml", "line": 3})
        );
    }

    #[test]
    fn expansion_that_changed_no_line_maps_nothing() {
        let map = SourceMap::new(
            Path::new("main.puml"),
            vec![line("main.puml", 1), line("main.puml", 2)],
            |path| path.display().to_string(),
        );

        assert_eq!(map.locate(1), None);
    }

    #[test]
    fn locate_names_the_file_and_line_of_each_expanded_line() {
        let map = SourceMap::new(
            Path::new("main.puml"),
            vec![
                line("main.puml", 1),
                line("sub/frag.puml", 7),
                line("main.puml", 5),
            ],
            |path| format!("ws/{}", path.display()),
        );

        assert_eq!(map.locate(1), Some(("ws/main.puml", 1)));
        assert_eq!(map.locate(2), Some(("ws/sub/frag.puml", 7)));
        assert_eq!(map.locate(3), Some(("ws/main.puml", 5)));
        assert_eq!(map.locate(0), None);
        assert_eq!(map.locate(4), None);
    }

    #[test]
    fn only_locations_stamped_by_the_parser_are_rewritten() {
        let map = SourceMap::new(
            Path::new("main.puml"),
            vec![line("main.puml", 1), line("frag.puml", 3)],
            |path| path.display().to_string(),
        );
        let mut json = serde_json::json!({
            "name": "x",
            "source_location": {"file": "parser.puml", "line": 2},
            "nested": [{"file": "other.puml", "line": 2}, {"file": "parser.puml", "line": 2, "extra": 1}],
        });

        map.rewrite(&mut json, "parser.puml");

        assert_eq!(
            json["source_location"],
            serde_json::json!({"file": "frag.puml", "line": 3})
        );
        assert_eq!(
            json["nested"][0],
            serde_json::json!({"file": "other.puml", "line": 2})
        );
        assert_eq!(json["nested"][1]["file"], "parser.puml");
    }

    // `apply` goes through JSON; a document must survive it unchanged when
    // every location maps to itself.
    #[test]
    fn rewriting_locations_loses_nothing_of_the_document() {
        use crate::parsing::{parse_puml_file, DiagramType};
        use puml_utils::LogLevel;

        let samples = [
            (
                DiagramType::Class,
                "@startuml\npackage p {\nabstract class A<T> <<s>> {\n  -x : int\n  +{static} Run(in a : int[1..*], ...) const : void\n  +operator int()\n}\nenum E {\n  V = 1\n}\n}\ninterface I\nA ..|> I\nA \"1\" --> \"*\" E : uses\n@enduml\n",
            ),
            (
                DiagramType::Component,
                "@startuml c\npackage \"pkg\" as pkg <<SEooC>> {\n  component \"Comp\" as Comp <<component>> {\n    component \"Unit\" as Unit <<unit>>\n  }\n}\ncomponent B <<component>>\nB --> Comp : call\n@enduml\n",
            ),
            (
                DiagramType::Sequence,
                "@startuml\nparticipant A\nparticipant B\nactivate A\nA -> B : call\nalt ok\n  B --> A : yes\nelse fail\n  B --> A : no\nend\nloop 3\n  A -> A : again\nend\ndeactivate A\n@enduml\n",
            ),
        ];

        for (diagram_type, text) in samples {
            let path = Rc::new(PathBuf::from("d.puml"));
            let mut parsed = parse_puml_file(
                &path,
                text,
                &SourceMap::identity(),
                LogLevel::Warn,
                diagram_type,
            )
            .unwrap_or_else(|error| panic!("{diagram_type:?} sample: {error}"));
            let before = format!("{parsed:?}");
            let lines = (1..=text.lines().count() as u32)
                .map(|line| ("d.puml".to_string(), line))
                .collect();

            SourceMap { lines }.apply(&mut parsed, "d.puml").unwrap();

            assert_eq!(format!("{parsed:?}"), before, "{diagram_type:?}");
        }
    }
}
