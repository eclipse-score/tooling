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

//! Outputs written for the resolved diagrams of a run. Each output is an
//! [`OutputSink`]; the run feeds every enabled sink the same diagrams.

use std::fs;
use std::path::{Path, PathBuf};

use activity_serializer::ActivitySerializer;
use class_diagram::ClassDiagram;
use class_serializer::ClassSerializer;
use component_serializer::ComponentSerializer;
use sequence_logic::SequenceTree;
use sequence_serializer::SequenceSerializer;

use puml_api::{render_trlc, ApiCollector};
use puml_utils::{write_fbs_to_file, write_json_to_file};

use crate::parsing::ParsedDiagram;
use crate::resolving::ResolvedDiagram;

type SinkResult = Result<(), Box<dyn std::error::Error>>;

/// One resolved diagram as handed to the sinks.
pub struct DiagramOutput<'a> {
    /// Path whose file stem names the output files.
    pub naming_path: &'a Path,
    /// Source path embedded in the outputs.
    pub source_file: &'a str,
    /// `@startuml <name>` of the diagram, if it has one.
    pub diagram_name: Option<&'a str>,
    pub resolved: &'a ResolvedDiagram,
}

pub trait OutputSink {
    /// Sees the raw AST of a diagram, before the run resolves any.
    fn parsed(&mut self, _naming_path: &Path, _parsed: &ParsedDiagram) -> SinkResult {
        Ok(())
    }

    fn write(&mut self, diagram: &DiagramOutput<'_>) -> SinkResult;

    /// Called once after the last diagram.
    fn finish(&mut self) -> SinkResult {
        Ok(())
    }
}

/// Creates the output directory of an `--*-output-dir` flag.
pub fn create_output_dir(dir: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = PathBuf::from(dir);
    fs::create_dir_all(&path)?;
    Ok(path)
}

/// FlatBuffers binary per diagram; with `debug_json` also the raw and resolved
/// ASTs as JSON.
pub struct FbsSink {
    dir: PathBuf,
    debug_json: bool,
}

impl FbsSink {
    pub fn new(dir: PathBuf, debug_json: bool) -> Self {
        Self { dir, debug_json }
    }
}

impl OutputSink for FbsSink {
    fn parsed(&mut self, naming_path: &Path, parsed: &ParsedDiagram) -> SinkResult {
        if self.debug_json {
            write_json_to_file(parsed, naming_path, &self.dir, "raw.ast")?;
        }
        Ok(())
    }

    fn write(&mut self, diagram: &DiagramOutput<'_>) -> SinkResult {
        if self.debug_json {
            write_json_to_file(
                diagram.resolved,
                diagram.naming_path,
                &self.dir,
                "logic.ast",
            )?;
        }
        let buffer =
            serialize_resolved_diagram(diagram.resolved, diagram.diagram_name, diagram.source_file);
        write_fbs_to_file(&buffer, diagram.naming_path, &self.dir)?;
        Ok(())
    }
}

/// FlatBuffers binary of `resolved`; the root carries `source` and, for a
/// component diagram, the `@startuml` name.
pub fn serialize_resolved_diagram(
    resolved: &ResolvedDiagram,
    diagram_name: Option<&str>,
    source: &str,
) -> Vec<u8> {
    match resolved {
        ResolvedDiagram::Activity(model) => ActivitySerializer::serialize(model),
        ResolvedDiagram::Component(model) => {
            ComponentSerializer::serialize(model, diagram_name, Some(source))
        }
        ResolvedDiagram::Class(model) => ClassSerializer::serialize(&ClassDiagram {
            source: Some(source.to_string()),
            ..model.clone()
        }),
        ResolvedDiagram::Sequence(model) => SequenceSerializer::serialize(&SequenceTree {
            source: Some(source.to_string()),
            ..model.clone()
        }),
    }
}

/// One TRLC file with the `ApiMethod` records of the public methods and free
/// functions of all class diagrams. Other diagram kinds are an error: a
/// silently empty API would hide a wrongly wired input.
pub struct PublicApiSink {
    file: PathBuf,
    package: String,
    collector: ApiCollector,
}

impl PublicApiSink {
    pub fn new(file: PathBuf, package: String) -> Self {
        Self {
            file,
            package,
            collector: ApiCollector::default(),
        }
    }
}

impl OutputSink for PublicApiSink {
    fn write(&mut self, diagram: &DiagramOutput<'_>) -> SinkResult {
        match diagram.resolved {
            ResolvedDiagram::Class(model) => {
                self.collector.add_diagram(diagram.source_file, model)?;
                Ok(())
            }
            other => Err(format!(
                "{}: the public API is collected from class diagrams, found a {} diagram",
                diagram.source_file,
                other.kind_name()
            )
            .into()),
        }
    }

    fn finish(&mut self) -> SinkResult {
        let methods = std::mem::take(&mut self.collector).into_methods();
        if let Some(parent) = self.file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.file, render_trlc(&self.package, &methods))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::{parse_puml_file, DiagramType};
    use crate::resolving::resolve_single;
    use crate::source_map::SourceMap;
    use puml_utils::LogLevel;
    use std::rc::Rc;

    fn resolved(content: &str, diagram_type: DiagramType) -> ResolvedDiagram {
        let path = Rc::new(PathBuf::from("d.puml"));
        let parsed = parse_puml_file(
            &path,
            content,
            &SourceMap::identity(),
            LogLevel::Warn,
            diagram_type,
        )
        .unwrap();
        resolve_single(parsed).unwrap()
    }

    fn output<'a>(resolved: &'a ResolvedDiagram, source_file: &'a str) -> DiagramOutput<'a> {
        DiagramOutput {
            naming_path: Path::new("d.puml"),
            source_file,
            diagram_name: None,
            resolved,
        }
    }

    #[test]
    fn public_api_sink_rejects_non_class_diagrams() {
        let component = resolved(
            "@startuml\ncomponent A <<component>>\n@enduml",
            DiagramType::Component,
        );
        let mut sink = PublicApiSink::new(PathBuf::from("unused.trlc"), "Pkg".to_string());

        let error = sink.write(&output(&component, "pkg/c.puml")).unwrap_err();

        assert_eq!(
            error.to_string(),
            "pkg/c.puml: the public API is collected from class diagrams, found a Component diagram"
        );
    }

    #[test]
    fn public_api_sink_writes_one_file_over_all_diagrams() {
        let dir = std::env::temp_dir().join(format!("puml_cli_sink_{}", std::process::id()));
        let file = dir.join("nested").join("api.trlc");
        let first = resolved(
            "@startuml\nclass A {\n+ F()\n}\n@enduml",
            DiagramType::Class,
        );
        let second = resolved(
            "@startuml\nclass B {\n+ G()\n}\n@enduml",
            DiagramType::Class,
        );
        let mut sink = PublicApiSink::new(file.clone(), "Pkg".to_string());

        sink.write(&output(&first, "a.puml")).unwrap();
        sink.write(&output(&second, "b.puml")).unwrap();
        sink.finish().unwrap();

        let trlc = fs::read_to_string(&file).unwrap();
        fs::remove_dir_all(&dir).ok();
        assert!(trlc.contains("ScoreReq.ApiMethod A_F {"), "got: {trlc}");
        assert!(trlc.contains("ScoreReq.ApiMethod B_G {"), "got: {trlc}");
    }
}
