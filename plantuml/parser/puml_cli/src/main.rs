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

mod fta;
mod inputs;
mod parsing;
mod resolving;
mod sinks;

use clap::{ArgGroup, Parser, ValueEnum};
use env_logger::Builder;
use log::debug;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

use puml_parser::Preprocessor;
use puml_utils::LogLevel;

use crate::fta::run_fta;
use crate::inputs::{
    collect_input_files, output_naming_path, output_source_path, require_one_per_file_or_none,
    validate_output_stem, validate_source_name, validate_trlc_package,
};
use crate::parsing::{parse_puml_file, DiagramType, ParsedDiagram};
use crate::resolving::resolve_parsed_diagrams;
use crate::sinks::{
    create_output_dir, DiagramOutput, FbsSink, IdmapSink, OutputSink, PublicApiSink,
};

/// CLI wrapper for LogLevel that implements ValueEnum
#[derive(Copy, Clone, ValueEnum, Debug)]
enum CliLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<CliLogLevel> for LogLevel {
    fn from(cli_level: CliLogLevel) -> Self {
        match cli_level {
            CliLogLevel::Error => LogLevel::Error,
            CliLogLevel::Warn => LogLevel::Warn,
            CliLogLevel::Info => LogLevel::Info,
            CliLogLevel::Debug => LogLevel::Debug,
            CliLogLevel::Trace => LogLevel::Trace,
        }
    }
}

/// PlantUML parser CLI tool
#[derive(Parser, Debug)]
#[command(name = "puml_parser_cli")]
#[command(version = "1.0")]
#[command(about = "Parse and analyze PlantUML component diagrams", long_about = None)]
#[command(group(
    ArgGroup::new("input")
        .required(true)
        .multiple(true)
        .args(&["file", "folders"]),
))]
struct Args {
    /// One or more PUML files to parse (can be repeated)
    #[arg(long)]
    file: Vec<String>,

    /// Folder containing PUML files
    #[arg(long)]
    folders: Option<String>,

    /// Log level: error, warn, info, debug, trace
    #[arg(long, value_enum, default_value = "warn")]
    log_level: CliLogLevel,

    /// Specify Grammar / Diagram type explicitly
    #[arg(long, value_enum, default_value = "none")]
    diagram_type: DiagramType,

    /// Output directory for generated FlatBuffers binary files.
    /// When omitted, no FlatBuffers files are written.
    #[arg(long)]
    fbs_output_dir: Option<String>,

    /// Output file for the generated public API TRLC (optional).
    /// When set, it holds the ``ApiMethod`` records of the public methods and
    /// free functions of all input class diagrams (see ``--api-package``);
    /// any other diagram kind among the inputs is an error.
    #[arg(long, requires = "api_package")]
    api_output_file: Option<String>,

    /// TRLC package name of the ``--api-output-file``; a TRLC identifier.
    /// Required together with ``--api-output-file``.
    #[arg(long, requires = "api_output_file")]
    api_package: Option<String>,

    /// Output directory for Fault-Tree-Analysis artifacts (optional).
    /// When set, every input diagram is treated as an FTA and
    /// ``fta_events.trlc`` (the generated ``RootCause`` records; see
    /// ``--fta-package``) is written to this directory.  No
    /// FlatBuffers / component processing is performed in this mode.
    #[arg(
        long,
        conflicts_with_all = ["fbs_output_dir", "api_output_file", "source_name", "output_stem"]
    )]
    fta_output_dir: Option<String>,

    /// TRLC package name for the generated ``fta_events.trlc`` stub file; a
    /// TRLC identifier. Required together with ``--fta-output-dir``.
    #[arg(long)]
    fta_package: Option<String>,

    /// Output directory for generated idmap sidecar files (optional).
    /// When set, a <stem>.idmap.json is written for each resolved diagram,
    /// recording the defines/references used by the clickable_plantuml
    /// Sphinx extension to resolve cross-diagram links.
    #[arg(long)]
    idmap_output_dir: Option<String>,

    /// Workspace-relative source path embedded in outputs, one per `--file`
    /// (same order); omit entirely or give exactly one per `--file`. Not
    /// combinable with `--folders` or `--fta-output-dir`. Must be relative.
    #[arg(long, conflicts_with = "folders")]
    source_name: Vec<String>,

    /// File stem for every output file, one per `--file` (same order), instead
    /// of each input's own stem; omit entirely or give exactly one per
    /// `--file`. Not combinable with `--folders` or `--fta-output-dir`. Must be
    /// a single path component.
    #[arg(long, conflicts_with = "folders")]
    output_stem: Vec<String>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let log_level: LogLevel = args.log_level.into();
    Builder::new()
        .filter_level(log_level.to_level_filter())
        .init();
    execute(&args, log_level)
}

fn execute(args: &Args, log_level: LogLevel) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = &args.fta_output_dir {
        // FTA mode short-circuits the normal output passes.
        let Some(fta_package) = &args.fta_package else {
            return Err("--fta-package is required together with --fta-output-dir".into());
        };
        validate_trlc_package("fta-package", fta_package)?;
        return run_fta(args, dir, fta_package, log_level);
    }

    if args.fta_package.is_some() {
        return Err("--fta-package requires --fta-output-dir".into());
    }

    for name in &args.source_name {
        validate_source_name(name)?;
    }
    for stem in &args.output_stem {
        validate_output_stem(stem)?;
    }

    let mut sinks = build_sinks(args, log_level)?;
    let (inputs, parsed) = load_diagrams(args, log_level, &mut sinks)?;

    debug!("Resolving {} diagram(s)", parsed.len());
    let source_files: Vec<String> = inputs.iter().map(|d| d.source_file.clone()).collect();
    let resolved_diagrams = resolve_parsed_diagrams(&source_files, &parsed)
        .map_err(|e| format!("Resolve error: {e}"))?;

    for (input, resolved) in inputs.iter().zip(&resolved_diagrams) {
        debug!(
            "Successfully resolved PlantUML document: {}",
            input.source_file
        );
        let output = DiagramOutput {
            naming_path: &input.naming_path,
            source_file: &input.source_file,
            diagram_name: input.diagram_name.as_deref(),
            resolved,
        };
        for sink in sinks.iter_mut() {
            sink.write(&output)?;
        }
    }
    for sink in sinks.iter_mut() {
        sink.finish()?;
    }

    debug!("Parsing completed");
    Ok(())
}

/// The sink of every output flag that is set.
fn build_sinks(
    args: &Args,
    log_level: LogLevel,
) -> Result<Vec<Box<dyn OutputSink>>, Box<dyn std::error::Error>> {
    let mut sinks: Vec<Box<dyn OutputSink>> = Vec::new();

    if let Some(dir) = &args.fbs_output_dir {
        let emit_debug_json = log_level.to_level_filter() >= log::LevelFilter::Debug;
        sinks.push(Box::new(FbsSink::new(
            create_output_dir(dir)?,
            emit_debug_json,
        )));
    }
    if let (Some(file), Some(package)) = (&args.api_output_file, &args.api_package) {
        validate_trlc_package("api-package", package)?;
        sinks.push(Box::new(PublicApiSink::new(
            PathBuf::from(file),
            package.clone(),
        )));
    }
    if let Some(dir) = &args.idmap_output_dir {
        sinks.push(Box::new(IdmapSink::new(create_output_dir(dir)?)));
    }

    Ok(sinks)
}

/// What the outputs need to know about an input besides its resolved model.
struct InputDiagram {
    /// Path whose file stem names the output files.
    naming_path: PathBuf,
    /// Source path embedded in the outputs.
    source_file: String,
    /// The `@startuml <name>` of a component diagram; the resolver drops it.
    diagram_name: Option<String>,
}

/// Collects, preprocesses and parses the input files, showing each parsed
/// diagram to `sinks`. Both results hold one entry per input, in input order.
fn load_diagrams(
    args: &Args,
    log_level: LogLevel,
    sinks: &mut [Box<dyn OutputSink>],
) -> Result<(Vec<InputDiagram>, Vec<ParsedDiagram>), Box<dyn std::error::Error>> {
    let file_list = collect_input_files(&args.file, args.folders.as_deref())?;

    if file_list.is_empty() {
        return Err("No valid PUML files found.".into());
    }
    require_one_per_file_or_none(&args.source_name, file_list.len(), "source-name")?;
    require_one_per_file_or_none(&args.output_stem, file_list.len(), "output-stem")?;
    debug!("Collected {} puml files.", file_list.len());

    // Per file: `IncludeExpander` keeps line numbers only for one file.
    debug!("Preprocessing: include expansion");
    let mut preprocessor = Preprocessor::new();
    let mut preprocessed: Vec<(Rc<PathBuf>, String)> = Vec::with_capacity(file_list.len());
    for path in &file_list {
        let singleton: HashSet<Rc<PathBuf>> = std::iter::once(Rc::clone(path)).collect();
        let mut file_map = preprocessor.preprocess(&singleton, log_level)?;
        let content = file_map
            .remove(path)
            .expect("preprocess returns an entry for its single input");
        preprocessed.push((Rc::clone(path), content));
    }

    debug!("Parsing started");
    let mut inputs = Vec::with_capacity(file_list.len());
    let mut parsed_diagrams = Vec::with_capacity(file_list.len());
    for (i, (path, content)) in preprocessed.iter().enumerate() {
        // Parsers only stamp `source_path` into `SourceLocation`s, so a path
        // built from `source_file` suffices.
        let output_stem = args.output_stem.get(i).map(String::as_str);
        let source_name = args.source_name.get(i).map(String::as_str);
        let naming_path = output_naming_path(output_stem, path).into_owned();
        let (source_file, source_path) = output_source_path(source_name, path);

        let parsed = parse_puml_file(&source_path, content, log_level, args.diagram_type)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        for sink in sinks.iter_mut() {
            sink.parsed(&naming_path, &parsed)?;
        }

        let diagram_name = match &parsed {
            ParsedDiagram::Component(doc) => doc.name.clone(),
            _ => None,
        };
        inputs.push(InputDiagram {
            naming_path,
            source_file,
            diagram_name,
        });
        parsed_diagrams.push(parsed);
    }
    Ok((inputs, parsed_diagrams))
}

#[cfg(test)]
mod sequence_pipeline_tests {
    use super::*;
    use crate::resolving::resolve_single;

    /// Parsing a sequence diagram must succeed end-to-end (parse → resolve).
    /// Before the fix this returned Err("Sequence diagrams not implemented").
    #[test]
    fn test_sequence_diagram_resolves_without_error() {
        let content = "\
@startuml
participant A
participant B
A -> B : call
B --> A : reply
@enduml";

        let path = Rc::new(PathBuf::from("test.puml"));
        let parsed = parse_puml_file(&path, content, LogLevel::Info, DiagramType::Sequence)
            .expect("sequence parse must succeed");

        let resolved = resolve_single(parsed);
        assert!(
            resolved.is_ok(),
            "sequence diagram must resolve without error; got: {:?}",
            resolved.err()
        );
    }
}

#[cfg(test)]
mod group_pipeline_tests {
    use super::*;
    use crate::resolving::ResolvedDiagram;

    fn parse(name: &str, content: &str, diagram_type: DiagramType) -> ParsedDiagram {
        let path = Rc::new(PathBuf::from(name));
        parse_puml_file(&path, content, LogLevel::Info, diagram_type)
            .unwrap_or_else(|error| panic!("{name} must parse: {error}"))
    }

    /// One call covering several kinds resolves each kind as its own group
    /// and returns the results in input order.
    #[test]
    fn mixed_kinds_resolve_per_kind_in_input_order() {
        let names: Vec<String> = [
            "scenario.puml",
            "class_detail.puml",
            "component_overview.puml",
            "cast.puml",
            "class_overview.puml",
            "component_detail.puml",
        ]
        .map(String::from)
        .to_vec();
        let parsed = vec![
            parse(
                "scenario.puml",
                "@startuml\nu -> u : call()\nactivate u\n@enduml",
                DiagramType::Sequence,
            ),
            parse(
                "class_detail.puml",
                "@startuml\nclass Sink {\n  +Write() : void\n}\n@enduml",
                DiagramType::Class,
            ),
            parse(
                "component_overview.puml",
                "@startuml\npackage \"pkg\" as pkg <<SEooC>> {\n  component \"Comp\" as Comp <<component>>\n}\n@enduml",
                DiagramType::Component,
            ),
            parse(
                "cast.puml",
                "@startuml\nparticipant \"pkg::Comp\" as u\n@enduml",
                DiagramType::Sequence,
            ),
            parse(
                "class_overview.puml",
                "@startuml\npackage logging {\n  class Sink\n}\n@enduml",
                DiagramType::Class,
            ),
            parse(
                "component_detail.puml",
                "@startuml\ncomponent \"Comp\" as Comp <<component>> {\n  component \"Unit\" as Unit <<unit>>\n}\n@enduml",
                DiagramType::Component,
            ),
        ];

        let resolved = resolve_parsed_diagrams(&names, &parsed).expect("group must resolve");

        assert!(matches!(resolved[0], ResolvedDiagram::Sequence(_)));
        assert!(matches!(resolved[1], ResolvedDiagram::Class(_)));
        assert!(matches!(resolved[2], ResolvedDiagram::Component(_)));
        assert!(matches!(resolved[3], ResolvedDiagram::Sequence(_)));
        assert!(matches!(resolved[4], ResolvedDiagram::Class(_)));
        assert!(matches!(resolved[5], ResolvedDiagram::Component(_)));

        let ResolvedDiagram::Class(detail) = &resolved[1] else {
            unreachable!()
        };
        assert_eq!(detail.entities[0].id, "logging.Sink");
    }

    /// A group error names the file it occurs in, not the first file of the
    /// call.
    #[test]
    fn group_error_names_the_failing_file() {
        let names: Vec<String> = ["fine.puml", "broken.puml"].map(String::from).to_vec();
        let parsed = vec![
            parse(
                "fine.puml",
                "@startuml\nclass Sink {\n  +Write() : void\n}\n@enduml",
                DiagramType::Class,
            ),
            parse(
                "broken.puml",
                "@startuml\nghost -> ghost : call()\n@enduml",
                DiagramType::Component,
            ),
        ];

        let error = resolve_parsed_diagrams(&names, &parsed)
            .expect_err("an unresolved reference must fail the group")
            .to_string();

        assert!(error.contains("broken.puml"), "got: {error}");
        assert!(!error.contains("fine.puml"), "got: {error}");
    }
}

#[cfg(test)]
mod cli_input_tests {
    use super::*;
    use clap::Parser;
    use std::fs;
    use std::path::Path;

    fn unique_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("puml_cli_{}_{}_{}", tag, std::process::id(), nanos));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, content: &str) {
        fs::write(path, content).unwrap();
    }

    fn execute_with(argv: &[&str]) -> Result<(), String> {
        let args = Args::parse_from(std::iter::once("puml_cli").chain(argv.iter().copied()));
        execute(&args, LogLevel::Warn).map_err(|error| error.to_string())
    }

    /// `--folders` files of one kind are one group: `b.puml` relates to a
    /// component only `a.puml` declares.
    #[test]
    fn folder_files_resolve_as_one_group() {
        let dir = unique_dir("folders");
        write(
            &dir.join("a.puml"),
            "@startuml\ncomponent A <<component>>\n@enduml",
        );
        write(
            &dir.join("b.puml"),
            "@startuml\ncomponent B <<component>>\nB --> A\n@enduml",
        );
        let out = dir.join("out");

        let result = execute_with(&[
            "--folders",
            dir.to_str().unwrap(),
            "--diagram-type",
            "component",
            "--fbs-output-dir",
            out.to_str().unwrap(),
        ]);

        fs::remove_dir_all(&dir).ok();
        assert_eq!(result, Ok(()));
    }

    /// The same files without `--folders` grouping each fail alone.
    #[test]
    fn separate_invocations_do_not_share_a_group() {
        let dir = unique_dir("single");
        write(
            &dir.join("b.puml"),
            "@startuml\ncomponent B <<component>>\nB --> A\n@enduml",
        );

        let result = execute_with(&[
            "--file",
            dir.join("b.puml").to_str().unwrap(),
            "--diagram-type",
            "component",
        ]);

        fs::remove_dir_all(&dir).ok();
        assert!(result.unwrap_err().contains("A"));
    }

    #[test]
    fn duplicate_file_is_an_error() {
        let dir = unique_dir("duplicate");
        write(&dir.join("a.puml"), "@startuml\ncomponent A\n@enduml");
        let file = dir.join("a.puml");

        let result = execute_with(&[
            "--file",
            file.to_str().unwrap(),
            "--file",
            file.to_str().unwrap(),
        ]);

        fs::remove_dir_all(&dir).ok();
        assert!(result.unwrap_err().starts_with("Duplicate --file entry: "));
    }

    #[test]
    fn source_name_count_must_match_file_count() {
        let dir = unique_dir("source_name");
        write(&dir.join("a.puml"), "@startuml\ncomponent A\n@enduml");
        write(&dir.join("b.puml"), "@startuml\ncomponent B\n@enduml");

        let result = execute_with(&[
            "--file",
            dir.join("a.puml").to_str().unwrap(),
            "--file",
            dir.join("b.puml").to_str().unwrap(),
            "--source-name",
            "pkg/a.puml",
        ]);

        fs::remove_dir_all(&dir).ok();
        assert!(result
            .unwrap_err()
            .starts_with("--source-name must be given exactly once per input file"));
    }

    #[test]
    fn plantuml_extension_is_rejected() {
        let dir = unique_dir("extension");
        write(&dir.join("a.plantuml"), "@startuml\ncomponent A\n@enduml");
        let file = dir.join("a.plantuml");

        let result = execute_with(&["--file", file.to_str().unwrap()]);

        fs::remove_dir_all(&dir).ok();
        assert!(result
            .unwrap_err()
            .starts_with("File is not a .puml file: "));
    }

    #[test]
    fn other_diagram_kind_is_named_when_component_is_expected() {
        for (content, kind) in [
            (
                "@startuml\nclass Sink {\n  +Write() : void\n}\n@enduml",
                "Class",
            ),
            ("@startuml\nstart\n:Do work;\nstop\n@enduml", "Activity"),
            (
                "@startuml\nparticipant A\nA -> A : call\n@enduml",
                "Sequence",
            ),
        ] {
            let path = Rc::new(PathBuf::from("d.puml"));
            let error = parse_puml_file(&path, content, LogLevel::Warn, DiagramType::Component)
                .err()
                .unwrap_or_else(|| panic!("{kind} diagram must not parse as Component"))
                .to_string();

            assert!(
                error.starts_with(&format!(
                    "d.puml: expected a Component diagram, found a {kind} diagram\n  Component parser error: "
                )),
                "{error}"
            );
        }
    }
}

#[cfg(test)]
mod fta_pipeline_tests {
    use super::*;
    use clap::Parser;
    use puml_utils::LogLevel;
    use std::fs;
    use std::path::Path;

    // Minimal metamodel: just enough procedure definitions for the include
    // expander to inline.  Topology is read from the original macro calls, so
    // the bodies are irrelevant.
    const TEST_METAMODEL: &str = "@startuml\n\
        !procedure $FailureMode($name, $alias)\n\
        rectangle \"$name\" as $alias\n\
        !endprocedure\n\
        !procedure $OrGate($alias, $connection)\n\
        rectangle \" \" as $alias\n\
        !endprocedure\n\
        !procedure $TransferInGate($alias, $connection)\n\
        rectangle \" \" as $alias\n\
        !endprocedure\n\
        !procedure $RootCause($name, $alias, $connection)\n\
        usecase \"$name\" as $alias\n\
        !endprocedure\n\
        @enduml\n";

    fn unique_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("puml_fta_{}_{}_{}", tag, std::process::id(), nanos));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn diagram(fm_name: &str, fm_fqn: &str, cm_alias: &str) -> String {
        format!(
            "@startuml\n!include fta_metamodel.puml\n\
             $FailureMode(\"{fm_name}\", \"{fm_fqn}\")\n\
             $OrGate(\"OG\", \"{fm_fqn}\")\n\
             $RootCause(\"a control measure\", \"{cm_alias}\", \"OG\")\n@enduml\n",
        )
    }

    fn args_for(out: &Path, files: &[&Path]) -> Args {
        let mut argv: Vec<String> = vec!["puml_cli".to_string()];
        for f in files {
            argv.push("--file".to_string());
            argv.push(f.to_str().unwrap().to_string());
        }
        argv.push("--fta-output-dir".to_string());
        argv.push(out.to_str().unwrap().to_string());
        argv.push("--fta-package".to_string());
        argv.push("TestFta".to_string());
        Args::parse_from(argv)
    }

    #[test]
    fn run_fta_emits_trlc_stub_without_rewriting_diagram() {
        let dir = unique_dir("emit");
        write(&dir.join("fta_metamodel.puml"), TEST_METAMODEL);
        let a = dir.join("a.puml");
        write(&a, &diagram("FM A", "Lib.FmA", "CmA"));
        let out = dir.join("out");

        let args = args_for(&out, &[&a]);
        run_fta(&args, out.to_str().unwrap(), "TestFta", LogLevel::Warn).expect("run_fta");

        // FTA mode is analysis-only: it does not rewrite or emit the diagram
        // (Sphinx/PlantUML renders the authored .puml, resolving the metamodel
        // via the global include path).
        assert!(!out.join("a.puml").exists());

        // TRLC stub: one RootCause record linking straight to the failure mode.
        let trlc = fs::read_to_string(out.join("fta_events.trlc")).unwrap();
        assert!(trlc.contains("package TestFta"));
        assert!(trlc.contains("import Lib"));
        assert!(trlc.contains("ScoreReq.RootCause CmA {"));
        assert!(trlc.contains("failure_modes = [Lib.FmA]"));
        assert!(!trlc.contains("FtaFailureMode"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn run_fta_aggregates_multiple_diagrams() {
        let dir = unique_dir("multi");
        write(&dir.join("fta_metamodel.puml"), TEST_METAMODEL);
        let a = dir.join("a.puml");
        let b = dir.join("b.puml");
        write(&a, &diagram("FM A", "Lib.FmA", "CmA"));
        write(&b, &diagram("FM B", "Lib.FmB", "CmB"));
        let out = dir.join("out");

        let args = args_for(&out, &[&a, &b]);
        run_fta(&args, out.to_str().unwrap(), "TestFta", LogLevel::Warn).expect("run_fta");

        let trlc = fs::read_to_string(out.join("fta_events.trlc")).unwrap();
        assert!(trlc.contains("ScoreReq.RootCause CmA {"));
        assert!(trlc.contains("failure_modes = [Lib.FmA]"));
        assert!(trlc.contains("ScoreReq.RootCause CmB {"));
        assert!(trlc.contains("failure_modes = [Lib.FmB]"));

        fs::remove_dir_all(&dir).ok();
    }

    fn transfer_diagram(fm_fqn: &str, targets: &[&str]) -> String {
        let gates: String = targets
            .iter()
            .map(|target| format!("$TransferInGate(\"{target}\", \"OG\")\n"))
            .collect();
        format!(
            "@startuml\n!include fta_metamodel.puml\n\
             $FailureMode(\"transferring\", \"{fm_fqn}\")\n\
             $OrGate(\"OG\", \"{fm_fqn}\")\n{gates}@enduml\n",
        )
    }

    fn run_transfer_fta(tag: &str, diagrams: &[(&str, String)]) -> (PathBuf, Result<(), String>) {
        let dir = unique_dir(tag);
        write(&dir.join("fta_metamodel.puml"), TEST_METAMODEL);
        let files: Vec<PathBuf> = diagrams
            .iter()
            .map(|(name, content)| {
                let path = dir.join(name);
                write(&path, content);
                path
            })
            .collect();
        let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
        let out = dir.join("out");
        let args = args_for(&out, &refs);
        let result = run_fta(&args, out.to_str().unwrap(), "TestFta", LogLevel::Warn)
            .map_err(|e| e.to_string());
        (dir, result)
    }

    #[test]
    fn run_fta_transfer_gate_inherits_root_causes_across_diagrams() {
        let (dir, result) = run_transfer_fta(
            "transfer",
            &[
                ("a_fta.puml", transfer_diagram("Lib.FmA", &["Lib.FmB2"])),
                (
                    "b_fta.puml",
                    "@startuml\n!include fta_metamodel.puml\n\
                     $FailureMode(\"FM B\", \"Lib.FmB1\", \"Lib.FmB2\")\n\
                     $OrGate(\"OG\", \"Lib.FmB1\")\n\
                     $RootCause(\"a control measure\", \"CmB\", \"OG\")\n@enduml\n"
                        .to_string(),
                ),
            ],
        );
        result.expect("run_fta");

        let trlc = fs::read_to_string(dir.join("out").join("fta_events.trlc")).unwrap();
        assert!(trlc.contains("ScoreReq.RootCause CmB {"));
        assert!(trlc.contains("failure_modes = [Lib.FmA, Lib.FmB1, Lib.FmB2]"));
        assert!(trlc.contains("// b_fta.puml:5\n"), "{trlc}");
        assert!(
            trlc.contains("diagram = \"b_fta.puml\"\n  line = 5\n"),
            "{trlc}"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn run_fta_transfer_gate_into_non_first_failure_mode_of_same_diagram() {
        let (dir, result) = run_transfer_fta(
            "transfer_same_diagram",
            &[(
                "a_fta.puml",
                "@startuml\n!include fta_metamodel.puml\n\
                 $FailureMode(\"FM A\", \"Lib.FmA\")\n\
                 $OrGate(\"OGA\", \"Lib.FmA\")\n\
                 $TransferInGate(\"Lib.FmB2\", \"OGA\")\n\
                 $FailureMode(\"FM B\", \"Lib.FmB1\", \"Lib.FmB2\")\n\
                 $OrGate(\"OGB\", \"Lib.FmB1\")\n\
                 $RootCause(\"a control measure\", \"CmB\", \"OGB\")\n@enduml\n"
                    .to_string(),
            )],
        );
        result.expect("run_fta");

        let trlc = fs::read_to_string(dir.join("out").join("fta_events.trlc")).unwrap();
        assert!(
            trlc.contains("failure_modes = [Lib.FmA, Lib.FmB1, Lib.FmB2]"),
            "{trlc}"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn run_fta_rejects_transfer_gate_aliasing_failure_mode_of_same_diagram() {
        let (dir, result) = run_transfer_fta(
            "transfer_alias_collision",
            &[(
                "a_fta.puml",
                "@startuml\n!include fta_metamodel.puml\n\
                 $FailureMode(\"FM A\", \"Lib.FmA\")\n\
                 $OrGate(\"OGA\", \"Lib.FmA\")\n\
                 $TransferInGate(\"Lib.FmB\", \"OGA\")\n\
                 $FailureMode(\"FM B\", \"Lib.FmB\")\n\
                 $OrGate(\"OGB\", \"Lib.FmB\")\n\
                 $RootCause(\"a control measure\", \"CmB\", \"OGB\")\n@enduml\n"
                    .to_string(),
            )],
        );
        let err = result.unwrap_err();
        assert!(err.contains("a_fta.puml:5"), "{err}");
        assert!(err.contains("Lib.FmB"), "{err}");
        assert!(err.contains("same alias"), "{err}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn run_fta_rejects_transfer_to_unknown_failure_mode() {
        let (dir, result) = run_transfer_fta(
            "transfer_unknown",
            &[
                ("a_fta.puml", transfer_diagram("Lib.FmA", &["Lib.Missing"])),
                ("b_fta.puml", diagram("FM B", "Lib.FmB", "CmB")),
            ],
        );
        let err = result.unwrap_err();
        assert!(err.contains("a_fta.puml"), "{err}");
        assert!(err.contains("Lib.Missing"), "{err}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn run_fta_rejects_transfer_gate_without_failure_mode_fqn() {
        let (dir, result) = run_transfer_fta(
            "transfer_alias",
            &[("a_fta.puml", transfer_diagram("Lib.FmA", &["Transfer1"]))],
        );
        let err = result.unwrap_err();
        assert!(err.contains("a_fta.puml"), "{err}");
        assert!(err.contains("Transfer1"), "{err}");
        assert!(err.contains("fully-qualified"), "{err}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn run_fta_rejects_transfer_cycle_across_diagrams() {
        let (dir, result) = run_transfer_fta(
            "transfer_cycle",
            &[
                ("a_fta.puml", transfer_diagram("Lib.FmA", &["Lib.FmB"])),
                ("b_fta.puml", transfer_diagram("Lib.FmB", &["Lib.FmA"])),
            ],
        );
        let err = result.unwrap_err();
        assert!(err.contains("cycle"), "{err}");
        assert!(err.contains("a_fta.puml:"), "{err}");
        assert!(err.contains("b_fta.puml:"), "{err}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn run_fta_rejects_basename_collision() {
        let dir = unique_dir("collision");
        // Same basename "fta.puml" under two sub-directories.
        let d1 = dir.join("d1");
        let d2 = dir.join("d2");
        write(&d1.join("fta_metamodel.puml"), TEST_METAMODEL);
        write(&d2.join("fta_metamodel.puml"), TEST_METAMODEL);
        let f1 = d1.join("fta.puml");
        let f2 = d2.join("fta.puml");
        write(&f1, &diagram("FM A", "Lib.FmA", "CmA"));
        write(&f2, &diagram("FM B", "Lib.FmB", "CmB"));
        let out = dir.join("out");

        let args = args_for(&out, &[&f1, &f2]);
        let err = run_fta(&args, out.to_str().unwrap(), "TestFta", LogLevel::Warn).unwrap_err();
        assert!(err.to_string().contains("duplicate FTA diagram basename"));

        fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod idmap_wiring_tests {
    use super::*;
    use crate::inputs::source_path_for_output;
    use crate::resolving::{resolve_single, ResolvedDiagram};
    use crate::sinks::{idmap_model_for, serialize_resolved_diagram};
    use puml_idmap::{write_empty_idmap_to_file, write_idmap_to_file};
    use std::fs;
    use std::path::Path;

    fn unique_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "puml_idmap_wiring_{}_{}_{}",
            tag,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Remove a test's temp dir, surfacing (rather than silently swallowing)
    /// any cleanup failure so a locked/undeletable dir doesn't go unnoticed.
    fn cleanup_dir(dir: &Path) {
        if let Err(e) = fs::remove_dir_all(dir) {
            eprintln!(
                "warning: failed to remove temp dir {}: {}",
                dir.display(),
                e
            );
        }
    }

    /// Activity diagrams carry no cross-linkable elements, so the CLI routes
    /// them to the explicit empty-writer API rather than a model converter.
    /// This verifies both the routing predicate (the diagram resolves to
    /// `ResolvedDiagram::Activity`) and the resulting on-disk empty idmap.
    #[test]
    fn activity_diagram_routes_to_empty_idmap_writer() {
        let content = "@startuml\nstart\n:Do work;\nstop\n@enduml";
        let path = Rc::new(PathBuf::from("flow/activity.puml"));

        let parsed = parse_puml_file(&path, content, LogLevel::Info, DiagramType::Activity)
            .expect("activity parse must succeed");
        let resolved = resolve_single(parsed).expect("activity must resolve");
        assert!(
            matches!(resolved, ResolvedDiagram::Activity(_)),
            "activity diagram must resolve to the Activity variant that triggers the empty writer"
        );

        // Mirror the CLI dispatch for the Activity arm.
        let dir = unique_dir("activity");
        let source_file = source_path_for_output(&path);
        let output = write_empty_idmap_to_file(&path, Some(&source_file), &dir)
            .expect("empty idmap must be written");

        assert_eq!(
            output.file_name().and_then(|n| n.to_str()),
            Some("activity.idmap.json")
        );
        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(json["source"], "flow/activity.puml");
        assert!(json["defines"].as_array().unwrap().is_empty());
        assert!(json["references"].as_array().unwrap().is_empty());

        cleanup_dir(&dir);
    }

    /// An absolute path outside the workspace must never leak into the emitted
    /// `source` field; the fallback collapses to the file name only.
    #[test]
    fn source_path_for_output_does_not_leak_absolute_paths() {
        let outside = Path::new("/nonexistent/abs/dir/diagram.puml");
        let source = source_path_for_output(outside);
        assert_eq!(source, "diagram.puml");
        assert!(
            !source.starts_with('/'),
            "absolute path must not leak into source, got: {source}"
        );
    }

    /// Component diagrams route through `idmap_model_for` to the
    /// `IdMapModel::Component` dispatch arm and `write_idmap_to_file`, unlike
    /// the Activity arm covered above.
    #[test]
    fn component_diagram_routes_to_idmap_model_dispatch() {
        let content = "@startuml\n\
             component \"Client\" as Client <<component>> {\n\
                 portout out1\n\
             }\n\
             component \"Server\" as Server <<component>> {\n\
                 portin in1\n\
             }\n\
             out1 --> in1 : calls\n\
             @enduml";
        let path = Rc::new(PathBuf::from("comp/client_server.puml"));

        let parsed = parse_puml_file(&path, content, LogLevel::Info, DiagramType::Component)
            .expect("component parse must succeed");
        let resolved = resolve_single(parsed).expect("component must resolve");
        assert!(matches!(resolved, ResolvedDiagram::Component(_)));

        let idmap_model =
            idmap_model_for(&resolved).expect("component diagrams must dispatch to an IdMapModel");

        let dir = unique_dir("component");
        let source_file = source_path_for_output(&path);
        let output = write_idmap_to_file(idmap_model, &path, Some(&source_file), None, &dir)
            .expect("component idmap must be written");

        assert_eq!(
            output.file_name().and_then(|n| n.to_str()),
            Some("client_server.idmap.json")
        );
        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(json["source"], "comp/client_server.puml");

        cleanup_dir(&dir);
    }

    /// Class diagrams route through `idmap_model_for` to the
    /// `IdMapModel::Class` dispatch arm and `write_idmap_to_file`.
    #[test]
    fn class_diagram_routes_to_idmap_model_dispatch() {
        let content = "@startuml\nclass A {\n    +a\n}\n@enduml";
        let path = Rc::new(PathBuf::from("cls/a.puml"));

        let parsed = parse_puml_file(&path, content, LogLevel::Info, DiagramType::Class)
            .expect("class parse must succeed");
        let resolved = resolve_single(parsed).expect("class must resolve");
        assert!(matches!(resolved, ResolvedDiagram::Class(_)));

        let idmap_model =
            idmap_model_for(&resolved).expect("class diagrams must dispatch to an IdMapModel");

        let dir = unique_dir("class");
        let source_file = source_path_for_output(&path);
        let output = write_idmap_to_file(idmap_model, &path, Some(&source_file), None, &dir)
            .expect("class idmap must be written");

        assert_eq!(
            output.file_name().and_then(|n| n.to_str()),
            Some("a.idmap.json")
        );
        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(json["source"], "cls/a.puml");

        cleanup_dir(&dir);
    }

    /// With `--source-name`, `SourceLocation.file` (and thus the FlatBuffers
    /// output) must carry the corrected `source_file`, never the staging
    /// `path` handed to `parse_puml_file`. Mirrors the `output_source_path`
    /// call from the main parsing loop rather than spawning the binary.
    #[test]
    fn source_location_uses_corrected_source_not_staging_path() {
        let staging_path = Rc::new(PathBuf::from("_puml_inputs/dir_part__foo.puml"));
        let content = "@startuml\nclass Foo {\n}\n@enduml";

        let (_source_file, source_path) =
            output_source_path(Some("real/pkg/foo.puml"), &staging_path);

        let parsed = parse_puml_file(&source_path, content, LogLevel::Info, DiagramType::Class)
            .expect("class parse must succeed");
        let resolved = resolve_single(parsed).expect("class diagram must resolve");
        let fbs_bytes = serialize_resolved_diagram(&resolved);
        let fbs_text = String::from_utf8_lossy(&fbs_bytes);

        assert!(
            fbs_text.contains("real/pkg/foo.puml"),
            "expected the corrected source_file to be embedded in the FBS output"
        );
        assert!(
            !fbs_text.contains("_puml_inputs"),
            "the staging path must never leak into the FBS output, got: {fbs_text}"
        );
    }

    /// Without `--source-name`, `source_file` is
    /// `source_path_for_output`'s workspace-relative computation while
    /// `SourceLocation`s keep using the input path itself.
    #[test]
    fn source_location_falls_back_to_relative_path_without_source_name() {
        let path = Rc::new(PathBuf::from("relative/dir/foo.puml"));

        let (source_file, source_path) = output_source_path(None, &path);

        assert_eq!(source_file, source_path_for_output(&path));
        assert!(Rc::ptr_eq(&source_path, &path));
    }

    /// `--output-stem` must override every output file's name, independent of
    /// the input path's own basename.
    #[test]
    fn output_stem_overrides_output_filenames() {
        let content = "@startuml\nclass A {\n    +a\n}\n@enduml";
        let path = Rc::new(PathBuf::from("cls/original_name.puml"));

        let parsed = parse_puml_file(&path, content, LogLevel::Info, DiagramType::Class)
            .expect("class parse must succeed");
        let resolved = resolve_single(parsed).expect("class must resolve");
        let idmap_model =
            idmap_model_for(&resolved).expect("class diagrams must dispatch to an IdMapModel");

        let naming_path = output_naming_path(Some("overridden_stem"), &path);
        assert_eq!(
            naming_path.file_name().and_then(|n| n.to_str()),
            Some("overridden_stem")
        );

        let dir = unique_dir("output_stem");
        let source_file = source_path_for_output(&path);
        let output = write_idmap_to_file(idmap_model, &naming_path, Some(&source_file), None, &dir)
            .expect("idmap must be written");

        assert_eq!(
            output.file_name().and_then(|n| n.to_str()),
            Some("overridden_stem.idmap.json")
        );
        // The embedded `source` field is unaffected by the naming override.
        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(json["source"], "cls/original_name.puml");

        cleanup_dir(&dir);
    }

    /// `--output-stem` must be given once per input file, or not at all.
    #[test]
    fn output_stem_rejected_with_mismatched_count() {
        let err = require_one_per_file_or_none(&["a".to_string()], 2, "output-stem")
            .expect_err("must reject a count that matches neither 0 nor the file count");
        assert!(err.to_string().contains("--output-stem"));
        assert!(err.to_string().contains("2 input file"));

        require_one_per_file_or_none(&["a".to_string(), "b".to_string()], 2, "output-stem")
            .expect("one entry per input file must be fine");
        require_one_per_file_or_none(&[], 2, "output-stem")
            .expect("omitting the flag entirely must never trigger the check");
    }

    /// `--output-stem` must be a single path component: no separators, and
    /// not `.`/`..`.
    #[test]
    fn output_stem_validation_rejects_paths_and_empty_values() {
        validate_output_stem("valid_stem").expect("a plain identifier must be accepted");
        assert!(validate_output_stem("").is_err());
        assert!(validate_output_stem("has/slash").is_err());
        assert!(validate_output_stem(".").is_err());
        assert!(validate_output_stem("..").is_err());
    }

    #[test]
    fn sequence_diagram_routes_to_idmap_model_dispatch() {
        let content =
            "@startuml\nparticipant Alice\nparticipant Bob\nAlice -> Bob : hello\n@enduml";
        let path = Rc::new(PathBuf::from("seq/hello.puml"));

        let parsed = parse_puml_file(&path, content, LogLevel::Info, DiagramType::Sequence)
            .expect("sequence parse must succeed");
        let resolved = resolve_single(parsed).expect("sequence must resolve");
        assert!(matches!(resolved, ResolvedDiagram::Sequence(_)));

        let idmap_model =
            idmap_model_for(&resolved).expect("sequence diagrams must dispatch to an IdMapModel");

        let dir = unique_dir("sequence");
        let source_file = source_path_for_output(&path);
        let output = write_idmap_to_file(idmap_model, &path, Some(&source_file), None, &dir)
            .expect("sequence idmap must be written");

        assert_eq!(
            output.file_name().and_then(|n| n.to_str()),
            Some("hello.idmap.json")
        );
        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(json["source"], "seq/hello.puml");
        assert!(json["defines"].as_array().unwrap().is_empty());
        assert_eq!(json["references"].as_array().unwrap().len(), 2);

        cleanup_dir(&dir);
    }

    /// Regression test for cross-diagram-type FQN linking: a component
    /// diagram's `interface` reference and a class/internal-API diagram's
    /// `interface` define must resolve to the *identical* FQN `id` when both
    /// declare the same enclosing `package`.
    ///
    /// Both `ComponentResolver` and `ClassResolver` build ids as dot-joined
    /// FQNs rooted at the enclosing `package`/namespace name (not at the
    /// `@startuml <name>`), so as long as two diagrams share the same
    /// package name for a jointly-referenced element, `puml_idmap`'s
    /// FQN-first matching in `clickable_plantuml` links them correctly
    /// without falling back to (collision-prone) alias-only matching. This
    /// mirrors the real `component_internal_api/positive_interface_match`
    /// integration test fixture.
    #[test]
    fn component_reference_and_class_define_share_fqn_for_same_package_interface() {
        let component_content = "@startuml\n\
             package package_a {\n\
                 component component_a <<component>> {\n\
                     component unit_1 <<unit>>\n\
                 }\n\
                 interface \"InternalInterface\" as InternalInterface\n\
                 unit_1 -( InternalInterface\n\
             }\n\
             @enduml";
        let component_path = Rc::new(PathBuf::from("comp/component_diagram.puml"));
        let component_parsed = parse_puml_file(
            &component_path,
            component_content,
            LogLevel::Info,
            DiagramType::Component,
        )
        .expect("component parse must succeed");
        let component_resolved = resolve_single(component_parsed).expect("component must resolve");
        let component_idmap_model = idmap_model_for(&component_resolved)
            .expect("component diagrams must dispatch to an IdMapModel");

        let class_content = "@startuml\n\
             package package_a {\n\
                 interface \"InternalInterface\" as InternalInterface {\n\
                     +GetData()\n\
                 }\n\
             }\n\
             @enduml";
        let class_path = Rc::new(PathBuf::from("cls/internal_api_diagram.puml"));
        let class_parsed = parse_puml_file(
            &class_path,
            class_content,
            LogLevel::Info,
            DiagramType::Class,
        )
        .expect("class parse must succeed");
        let class_resolved = resolve_single(class_parsed).expect("class must resolve");
        let class_idmap_model = idmap_model_for(&class_resolved)
            .expect("class diagrams must dispatch to an IdMapModel");

        let dir = unique_dir("cross_type_fqn");
        let component_output = write_idmap_to_file(
            component_idmap_model,
            &component_path,
            Some(&source_path_for_output(&component_path)),
            None,
            &dir,
        )
        .expect("component idmap must be written");
        let class_output = write_idmap_to_file(
            class_idmap_model,
            &class_path,
            Some(&source_path_for_output(&class_path)),
            None,
            &dir,
        )
        .expect("class idmap must be written");

        let component_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&component_output).unwrap()).unwrap();
        let class_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&class_output).unwrap()).unwrap();

        let component_ref_id = component_json["references"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["alias"] == "InternalInterface")
            .expect("component diagram must reference InternalInterface")["id"]
            .as_str()
            .unwrap()
            .to_string();
        let class_define_id = class_json["defines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["alias"] == "InternalInterface")
            .expect("class diagram must define InternalInterface")["id"]
            .as_str()
            .unwrap()
            .to_string();

        assert_eq!(
            component_ref_id, class_define_id,
            "component reference FQN and class define FQN must match exactly \
             for clickable_plantuml's FQN-first resolution to link them"
        );
        assert_eq!(component_ref_id, "package_a.InternalInterface");

        cleanup_dir(&dir);
    }
}

#[cfg(test)]
mod public_api_tests {
    use super::*;
    use clap::Parser;
    use std::fs;
    use std::path::Path;

    const CLASS_DIAGRAM: &str = "@startuml\n\
        namespace score.mw {\n\
        interface Skeleton {\n\
        + OfferService() : Result\n\
        + OfferService(int) : Result\n\
        - Hidden()\n\
        }\n\
        }\n\
        @enduml\n";

    fn unique_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "puml_cli_api_{}_{}_{}",
            tag,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn run(file: &Path, out: &Path, diagram_type: &str) -> Result<(), String> {
        run_with_package(file, out, diagram_type, "Pkg_api")
    }

    fn run_with_package(
        file: &Path,
        out: &Path,
        diagram_type: &str,
        package: &str,
    ) -> Result<(), String> {
        let args = Args::parse_from([
            "puml_cli",
            "--file",
            file.to_str().unwrap(),
            "--source-name",
            "pkg/api.puml",
            "--diagram-type",
            diagram_type,
            "--api-output-file",
            out.join("public_api.trlc").to_str().unwrap(),
            "--api-package",
            package,
        ]);
        execute(&args, LogLevel::Warn).map_err(|e| e.to_string())
    }

    #[test]
    fn class_diagram_emits_api_method_records() {
        let dir = unique_dir("emit");
        let file = dir.join("api.puml");
        fs::write(&file, CLASS_DIAGRAM).unwrap();
        let out = dir.join("out");

        run(&file, &out, "class").unwrap();

        let trlc = fs::read_to_string(out.join("public_api.trlc")).unwrap();
        fs::remove_dir_all(&dir).ok();
        assert!(trlc.contains("package Pkg_api\n"), "got: {trlc}");
        assert!(trlc.contains("import ScoreReq\n"), "got: {trlc}");
        assert!(
            trlc.contains("ScoreReq.ApiMethod score_mw_Skeleton_OfferService {"),
            "got: {trlc}"
        );
        assert!(trlc.contains("diagram = \"pkg/api.puml\""), "got: {trlc}");
        assert!(
            trlc.contains(
                "signatures = [\"`Result score::mw::Skeleton::OfferService()`\", \
                 \"`Result score::mw::Skeleton::OfferService(int)`\"]"
            ),
            "got: {trlc}"
        );
        assert!(!trlc.contains("Hidden"), "got: {trlc}");
    }

    #[test]
    fn non_class_diagrams_are_rejected() {
        let dir = unique_dir("non_class");
        let file = dir.join("c.puml");
        fs::write(&file, "@startuml\ncomponent A <<component>>\n@enduml").unwrap();

        let result = run(&file, &dir.join("out"), "none");

        fs::remove_dir_all(&dir).ok();
        assert_eq!(
            result.unwrap_err(),
            "pkg/api.puml: the public API is collected from class diagrams, found a Component diagram"
        );
    }

    #[test]
    fn explicit_class_type_rejects_other_diagram_kinds() {
        let dir = unique_dir("explicit_type");
        let file = dir.join("c.puml");
        fs::write(&file, "@startuml\ncomponent A <<component>>\n@enduml").unwrap();

        let result = run(&file, &dir.join("out"), "class");

        fs::remove_dir_all(&dir).ok();
        let error = result.unwrap_err();
        assert!(
            error.starts_with(
                "pkg/api.puml: expected a Class diagram, found a Component diagram\n  Class parser error: "
            ),
            "{error}"
        );
    }

    #[test]
    fn package_must_be_a_trlc_identifier() {
        let dir = unique_dir("package");
        let file = dir.join("api.puml");
        fs::write(&file, CLASS_DIAGRAM).unwrap();

        let result = run_with_package(&file, &dir.join("out"), "class", "_pkg");

        fs::remove_dir_all(&dir).ok();
        assert!(result
            .unwrap_err()
            .starts_with("--api-package must be a TRLC identifier"));
    }

    #[test]
    fn colliding_record_names_fail_the_run() {
        let dir = unique_dir("collision");
        let file = dir.join("api.puml");
        fs::write(
            &file,
            "@startuml\nclass A {\n+ GetX()\n+ get_x()\n}\n@enduml\n",
        )
        .unwrap();

        let result = run(&file, &dir.join("out"), "class");

        fs::remove_dir_all(&dir).ok();
        assert!(result.unwrap_err().contains("A_get_x"));
    }

    #[test]
    fn api_flags_require_each_other() {
        assert!(
            Args::try_parse_from(["puml_cli", "--file", "a.puml", "--api-output-file", "out"])
                .is_err()
        );
        assert!(
            Args::try_parse_from(["puml_cli", "--file", "a.puml", "--api-package", "P"]).is_err()
        );
    }

    #[test]
    fn api_output_conflicts_with_fta_mode() {
        assert!(Args::try_parse_from([
            "puml_cli",
            "--file",
            "a.puml",
            "--fta-output-dir",
            "out",
            "--fta-package",
            "F",
            "--api-output-file",
            "out",
            "--api-package",
            "P",
        ])
        .is_err());
    }
}
