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

use clap::{ArgGroup, Parser, ValueEnum};
use env_logger::Builder;
use log::debug;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use activity_diagram::ActivityDiagram;
use activity_serializer::ActivitySerializer;
use class_serializer::ClassSerializer;
use component_serializer::ComponentSerializer;
use sequence_serializer::SequenceSerializer;

use puml_fta::{lobster_document, FtaChain, FtaModel};
use puml_idmap::{write_idmap_to_file, IdMapModel};
use puml_lobster::{write_lobster_to_file, LobsterModel};
use puml_parser::{
    DiagramParser, ErrorLocation, Preprocessor, ProcedureParserService, PumlActivityParser,
    PumlClassParser, PumlComponentParser, PumlSequenceParser,
};
use puml_resolver::{
    ActivityResolver, ClassResolver, ComponentResolver, DiagramResolver, SequenceResolver,
};
use puml_utils::{write_fbs_to_file, write_json_to_file, LogLevel};

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

    /// Output directory for generated lobster files (optional).
    /// When set, a <stem>.lobster is written for each diagram that resolves
    /// to a Component or Class model (independent of --fbs-output-dir).
    /// On resolve errors a placeholder empty .lobster is written so the
    /// build output set is always complete.
    #[arg(long)]
    lobster_output_dir: Option<String>,

    /// Output directory for Fault-Tree-Analysis artifacts (optional).
    /// When set, every input diagram is treated as an FTA: its
    /// ``fta_metamodel.puml`` include is inlined and the metamodel-inlined
    /// ``.puml`` is written to this directory, alongside an aggregated
    /// ``root_causes.lobster`` (lobster-act-trace) and ``fta_chains.json``
    /// describing the per-failure-mode chains.  No FlatBuffers / component
    /// processing is performed in this mode.
    #[arg(long)]
    fta_output_dir: Option<String>,

    /// Output directory for generated idmap sidecar files (optional).
    /// When set, a <stem>.idmap.json is written for each resolved diagram,
    /// recording the defines/references used by the clickable_plantuml
    /// Sphinx extension to resolve cross-diagram links.
    #[arg(long)]
    idmap_output_dir: Option<String>,

    /// Stable workspace-relative source name baked into generated artifacts
    /// (FlatBuffers/lobster/idmap ``source`` field). When omitted, the
    /// filesystem basename is used as a fallback.
    #[arg(long)]
    source_name: Option<String>,
}

#[derive(Copy, Clone, ValueEnum, Debug)]
enum DiagramType {
    None,
    Activity,
    Component,
    Deployment,
    Class,
    Sequence,
}

#[allow(dead_code)] // Class and Sequence variants are WIP
#[derive(Debug, Serialize)]
enum ParsedDiagram {
    Activity(puml_parser::RawActivityDiagram),
    Component(puml_parser::CompPumlDocument),
    Class(puml_parser::ClassUmlFile),
    Sequence(puml_parser::SeqPumlDocument),
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
    if let Some(dir) = &args.fta_output_dir {
        // FTA mode is a self-contained pipeline that short-circuits the normal
        // FlatBuffers/lobster passes; reject co-specified output modes rather
        // than silently ignoring them.
        if args.fbs_output_dir.is_some() || args.lobster_output_dir.is_some() {
            return Err(
                "--fta-output-dir cannot be combined with --fbs-output-dir or \
                        --lobster-output-dir"
                    .into(),
            );
        }
        return run_fta(&args, dir, log_level);
    }

    let emit_debug_json = log_level.to_level_filter() >= log::LevelFilter::Debug;

    let fbs_output_dir: Option<PathBuf> = if let Some(dir) = &args.fbs_output_dir {
        let p = PathBuf::from(dir);
        fs::create_dir_all(&p)?;
        Some(p)
    } else {
        None
    };

    let lobster_output_dir: Option<PathBuf> = if let Some(dir) = &args.lobster_output_dir {
        let p = PathBuf::from(dir);
        fs::create_dir_all(&p)?;
        Some(p)
    } else {
        None
    };

    let idmap_output_dir: Option<PathBuf> = if let Some(dir) = &args.idmap_output_dir {
        let p = PathBuf::from(dir);
        fs::create_dir_all(&p)?;
        Some(p)
    } else {
        None
    };

    let file_list = collect_files_from_args(&args)?;

    if file_list.is_empty() {
        return Err("No valid PUML files found.".into());
    }
    debug!("Collected {} puml files.", file_list.len());

    debug!("Preprocessing: include expansion");
    let mut preprocessor = Preprocessor::new();
    let preprocessed_files = preprocessor.preprocess(&file_list, log_level)?;

    debug!("Parsing started");
    for (path, content) in &preprocessed_files {
        let parsed_content = parse_puml_file(path, content, log_level, args.diagram_type)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        if emit_debug_json {
            if let Some(ref dir) = fbs_output_dir {
                write_json_to_file(&parsed_content, path, dir, "raw.ast")?;
            }
        }

        match resolve_parsed_diagram(parsed_content) {
            Ok(logic_result) => {
                debug!(
                    "Successfully resolved PlantUML document: {}",
                    path.display()
                );
                if emit_debug_json {
                    if let Some(ref dir) = fbs_output_dir {
                        write_json_to_file(&logic_result, path, dir, "logic.ast")?;
                    }
                }

                // Prefer the stable workspace-relative --source-name when
                // provided; fall back to the filesystem basename (legacy).
                let source_file: &str = args
                    .source_name
                    .as_deref()
                    .or_else(|| path.file_name().and_then(|n| n.to_str()))
                    .unwrap_or_default();
                let fbs_buffer = serialize_resolved_diagram(&logic_result, source_file);
                if let Some(ref dir) = fbs_output_dir {
                    write_fbs_to_file(&fbs_buffer, path, dir)?;
                }

                if let Some(ldir) = &lobster_output_dir {
                    let lobster_model = match &logic_result {
                        ResolvedDiagram::Component(model) => LobsterModel::Component(model),
                        ResolvedDiagram::Class(model) => LobsterModel::Class(model),
                        ResolvedDiagram::Activity(_) => LobsterModel::Empty,
                        ResolvedDiagram::Sequence(_) => LobsterModel::Empty,
                    };
                    write_lobster_to_file(lobster_model, path, Some(source_file), ldir)?;
                }

                if let Some(idir) = &idmap_output_dir {
                    let idmap_model = match &logic_result {
                        ResolvedDiagram::Component(model) => IdMapModel::Component(model),
                        ResolvedDiagram::Class(model) => IdMapModel::Class(model),
                        ResolvedDiagram::Activity(_) => IdMapModel::Empty,
                        ResolvedDiagram::Sequence(model) => IdMapModel::Sequence(model),
                    };
                    write_idmap_to_file(idmap_model, path, Some(source_file), idir)?;
                }
            }
            Err(e) => {
                return Err(format!("Resolve error in {}: {}", path.display(), e).into());
            }
        }
    }

    debug!("Parsing completed");
    Ok(())
}

/// FTA processing pipeline: inline the metamodel, parse the fault-tree macro
/// calls, and emit the metamodel-inlined `.puml`, an aggregated
/// `root_causes.lobster`, and `fta_chains.json`.
fn run_fta(
    args: &Args,
    output_dir: &str,
    log_level: LogLevel,
) -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(output_dir);
    fs::create_dir_all(&out)?;

    let inputs = collect_files_from_args(args)?;
    if inputs.is_empty() {
        return Err("No valid PUML files found.".into());
    }

    // Process inputs in a deterministic order for reproducible output.
    let mut sorted: Vec<Rc<PathBuf>> = inputs.into_iter().collect();
    sorted.sort();

    let mut all_items: Vec<serde_json::Value> = Vec::new();
    let mut all_chains: Vec<FtaChain> = Vec::new();
    // Chains/lobster items reference each diagram by basename; two inputs sharing
    // a basename (in different directories) would be indistinguishable downstream.
    let mut seen_basenames: HashSet<String> = HashSet::new();

    for file in &sorted {
        let basename = file.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            format!(
                "input path has no valid UTF-8 file name: {}",
                file.display()
            )
        })?;
        // The metamodel carries only `!procedure` definitions, no fault tree.
        if basename == "fta_metamodel.puml" {
            continue;
        }
        if !seen_basenames.insert(basename.to_string()) {
            return Err(format!(
                "duplicate FTA diagram basename {:?}: inputs in different directories \
                 would be indistinguishable in {}",
                basename, output_dir,
            )
            .into());
        }

        // Analysis only: the fault-tree topology comes entirely from the
        // `$TopEvent(...)` / `$BasicEvent(...)` / gate macro *calls*, which the
        // procedure parser reads straight from the source (the `!include
        // fta_metamodel.puml` line is inert text).  The diagram is rendered
        // as-authored by Sphinx/PlantUML, which finds the metamodel via the
        // toolchain's global `plantuml.include.path`.
        let source = fs::read_to_string(file.as_path())?;
        let parsed = ProcedureParserService.parse_file(file, &source, log_level)?;
        let model = FtaModel::from_procedure_file(&parsed)?;
        all_items.extend(model.lobster_items(basename));
        all_chains.extend(model.chains(basename));
        debug!("Processed FTA diagram: {}", file.display());
    }

    let lobster = lobster_document(all_items);
    fs::write(
        out.join("root_causes.lobster"),
        serde_json::to_string_pretty(&lobster)? + "\n",
    )?;
    fs::write(
        out.join("fta_chains.json"),
        serde_json::to_string_pretty(&all_chains)? + "\n",
    )?;
    Ok(())
}

fn serialize_resolved_diagram(resolved_content: &ResolvedDiagram, source_file: &str) -> Vec<u8> {
    match resolved_content {
        ResolvedDiagram::Activity(resolved_content) => {
            ActivitySerializer::serialize(resolved_content, source_file)
        }
        ResolvedDiagram::Component(resolved_content) => {
            ComponentSerializer::serialize(resolved_content, source_file)
        }
        ResolvedDiagram::Class(resolved_content) => {
            ClassSerializer::serialize(resolved_content, source_file)
        }
        ResolvedDiagram::Sequence(resolved_content) => {
            SequenceSerializer::serialize(resolved_content, source_file)
        }
    }
}

#[derive(Debug, Serialize)]
pub enum ResolvedDiagram {
    Activity(ActivityDiagram),
    Component(HashMap<String, component_diagram::LogicComponent>),
    Class(class_diagram::ClassDiagram),
    Sequence(sequence_logic::SequenceTree),
}

fn resolve_parsed_diagram(
    parsed_content: ParsedDiagram,
) -> Result<ResolvedDiagram, Box<dyn std::error::Error>> {
    match parsed_content {
        ParsedDiagram::Activity(parsed_content) => {
            let mut resolver = ActivityResolver::new();
            puml_resolver(&mut resolver, &parsed_content).map(ResolvedDiagram::Activity)
        }
        ParsedDiagram::Component(parsed_content) => {
            let mut resolver = ComponentResolver::new();
            puml_resolver(&mut resolver, &parsed_content).map(ResolvedDiagram::Component)
        }
        ParsedDiagram::Class(parsed_content) => {
            let mut resolver = ClassResolver::new();
            puml_resolver(&mut resolver, &parsed_content).map(ResolvedDiagram::Class)
        }
        ParsedDiagram::Sequence(parsed_content) => {
            let mut resolver = SequenceResolver;
            puml_resolver(&mut resolver, &parsed_content).map(ResolvedDiagram::Sequence)
        }
    }
}

fn puml_resolver<Resolver>(
    resolver: &mut Resolver,
    parsed_content: &Resolver::Document,
) -> Result<Resolver::Output, Box<dyn std::error::Error>>
where
    Resolver: DiagramResolver,
    Resolver::Output: std::fmt::Debug,
    Resolver::Error: std::error::Error + 'static,
{
    let logic_result = resolver
        .resolve(parsed_content)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    Ok(logic_result)
}

fn parse_with_parser<P>(
    parser: &mut P,
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
) -> Result<P::Output, Box<dyn std::error::Error>>
where
    P: DiagramParser,
    P::Output: std::fmt::Debug,
    P::Error: std::error::Error + 'static,
{
    let parsed_content = parser
        .parse_file(path, content, log_level)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    debug!("Successfully parsed PlantUML document: {}", path.display());
    Ok(parsed_content)
}

// lobster-trace: Tools.ArchitectureModelingSyntax
fn parse_puml_file(
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
    diagram_type: DiagramType,
) -> Result<ParsedDiagram, Box<dyn std::error::Error>> {
    match diagram_type {
        DiagramType::Activity => {
            parse_with_parser(&mut PumlActivityParser, path, content, log_level)
                .map(ParsedDiagram::Activity)
        }
        DiagramType::Component | DiagramType::Deployment => {
            parse_with_parser(&mut PumlComponentParser, path, content, log_level)
                .map(ParsedDiagram::Component)
        }
        DiagramType::Class => parse_with_parser(&mut PumlClassParser, path, content, log_level)
            .map(ParsedDiagram::Class),
        DiagramType::Sequence => {
            parse_with_parser(&mut PumlSequenceParser, path, content, log_level)
                .map(ParsedDiagram::Sequence)
        }
        DiagramType::None => parse_in_order(path, content, log_level),
    }
}

type ParseAttempt<'a> = (&'a str, Box<dyn std::error::Error>, Option<(usize, usize)>);

fn parse_in_order(
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
) -> Result<ParsedDiagram, Box<dyn std::error::Error>> {
    // Each attempt records the parser name, the boxed error, and the source
    // location extracted from the concrete type before boxing.
    let mut attempts: Vec<ParseAttempt<'_>> = Vec::new();

    match PumlComponentParser.parse_file(path, content, log_level) {
        Ok(doc) => {
            debug!("Successfully detected as Component diagram");
            return Ok(ParsedDiagram::Component(doc));
        }
        Err(e) => {
            let loc = e.error_location();
            debug!("Component parser failed at {:?}: {}", loc, e);
            attempts.push(("Component", Box::new(e), loc));
        }
    }

    match PumlActivityParser.parse_file(path, content, log_level) {
        Ok(doc) => {
            debug!("Successfully detected as Activity diagram");
            return Ok(ParsedDiagram::Activity(doc));
        }
        Err(e) => {
            let loc = e.error_location();
            debug!("Activity parser failed at {:?}: {}", loc, e);
            attempts.push(("Activity", Box::new(e), loc));
        }
    }

    match PumlClassParser.parse_file(path, content, log_level) {
        Ok(doc) => {
            debug!("Successfully detected as Class diagram");
            return Ok(ParsedDiagram::Class(doc));
        }
        Err(e) => {
            let loc = e.error_location();
            debug!("Class parser failed at {:?}: {}", loc, e);
            attempts.push(("Class", Box::new(e), loc));
        }
    }

    match PumlSequenceParser.parse_file(path, content, log_level) {
        Ok(doc) => {
            debug!("Successfully detected as Sequence diagram");
            return Ok(ParsedDiagram::Sequence(doc));
        }
        Err(e) => {
            let loc = e.error_location();
            debug!("Sequence parser failed at {:?}: {}", loc, e);
            attempts.push(("Sequence", Box::new(e), loc));
        }
    }

    // The parser that reached the furthest line is the most informative one.
    let best = attempts
        .iter()
        .max_by_key(|(_, _, loc)| loc.map_or(0, |(line, _)| line));

    let tried_names: Vec<&str> = attempts.iter().map(|(n, _, _)| *n).collect();

    let detail = match best {
        Some((best_name, best_err, Some((line_no, _col)))) => {
            let source_line = content
                .lines()
                .nth(line_no - 1)
                .unwrap_or("<unknown>")
                .trim();
            format!(
                "\n  Parsers tried: {}\n  Parser with longest match: {}\n  Failed at line {}: {}\n  Error: {}",
                tried_names.join(", "),
                best_name,
                line_no,
                source_line,
                best_err,
            )
        }
        Some((best_name, best_err, None)) => {
            format!(
                "\n  Parsers tried: {}\n  Parser with longest match: {}\n  Error: {}",
                tried_names.join(", "),
                best_name,
                best_err,
            )
        }
        None => String::new(),
    };

    Err(format!(
        "Failed to parse {} with any available parser{}",
        path.display(),
        detail,
    )
    .into())
}

fn collect_files_from_args(
    args: &Args,
) -> Result<HashSet<Rc<PathBuf>>, Box<dyn std::error::Error>> {
    let mut file_list: HashSet<Rc<PathBuf>> = HashSet::new();

    // Collect individual files from --file arguments (may be repeated)
    for file_path in &args.file {
        add_single_file(Path::new(file_path), &mut file_list)?;
    }

    // Collect files from folders using --folders argument
    if let Some(folder_path) = &args.folders {
        collect_puml_files_from_folder(Path::new(folder_path), &mut file_list)?;
    }

    Ok(file_list)
}

fn resolve_path(path: &Path) -> PathBuf {
    // When running with 'bazel run', use BUILD_WORKSPACE_DIRECTORY
    let base_dir = std::env::var("BUILD_WORKSPACE_DIRECTORY")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn add_single_file(
    path: &Path,
    file_list: &mut HashSet<Rc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = resolve_path(path);

    if !abs_path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()).into());
    }
    if abs_path.extension().and_then(|ext| ext.to_str()) != Some("puml") {
        return Err(format!("File is not a .puml file: {}", path.display()).into());
    }
    file_list.insert(Rc::new(abs_path));
    Ok(())
}

fn collect_puml_files_from_folder(
    dir: &Path,
    file_list: &mut HashSet<Rc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let abs_dir = resolve_path(dir);

    if !abs_dir.is_dir() {
        return Err(format!("Path is not a directory: {}", dir.display()).into());
    }
    collect_puml_files(&abs_dir, file_list)?;
    Ok(())
}

fn collect_puml_files(
    dir: &Path,
    file_list: &mut HashSet<Rc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_puml_files(&path, file_list)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("puml") {
            file_list.insert(Rc::new(path.to_path_buf()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod sequence_pipeline_tests {
    use super::*;

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

        let resolved = resolve_parsed_diagram(parsed);
        assert!(
            resolved.is_ok(),
            "sequence diagram must resolve without error; got: {:?}",
            resolved.err()
        );
    }
}

#[cfg(test)]
mod fta_pipeline_tests {
    use super::*;
    use clap::Parser;
    use puml_utils::LogLevel;

    // Minimal metamodel: just enough procedure definitions for the include
    // expander to inline.  Topology is read from the original macro calls, so
    // the bodies are irrelevant.
    const TEST_METAMODEL: &str = "@startuml\n\
        !procedure $TopEvent($name, $alias)\n\
        rectangle \"$name\" as $alias\n\
        !endprocedure\n\
        !procedure $OrGate($alias, $connection)\n\
        rectangle \" \" as $alias\n\
        !endprocedure\n\
        !procedure $BasicEvent($name, $alias, $connection)\n\
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

    fn diagram(fm_name: &str, fm_fqn: &str, cm_fqn: &str) -> String {
        format!(
            "@startuml\n!include fta_metamodel.puml\n\
             $TopEvent(\"{fm_name}\", \"{fm_fqn}\")\n\
             $OrGate(\"OG\", \"{fm_fqn}\")\n\
             $BasicEvent(\"a control measure\", \"{cm_fqn}\", \"OG\")\n@enduml\n",
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
        Args::parse_from(argv)
    }

    #[test]
    fn run_fta_emits_lobster_and_chains_without_rewriting_diagram() {
        let dir = unique_dir("emit");
        write(&dir.join("fta_metamodel.puml"), TEST_METAMODEL);
        let a = dir.join("a.puml");
        write(&a, &diagram("FM A", "Lib.FmA", "Lib.CmA"));
        let out = dir.join("out");

        let args = args_for(&out, &[&a]);
        run_fta(&args, out.to_str().unwrap(), LogLevel::Warn).expect("run_fta");

        // FTA mode is analysis-only: it does not rewrite or emit the diagram
        // (Sphinx/PlantUML renders the authored .puml, resolving the metamodel
        // via the global include path).
        assert!(!out.join("a.puml").exists());

        // Lobster: act-trace envelope with a top + basic event.
        let lobster: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(out.join("root_causes.lobster")).unwrap())
                .unwrap();
        assert_eq!(lobster["schema"], "lobster-act-trace");
        let data = lobster["data"].as_array().unwrap();
        assert_eq!(data.len(), 2);

        // Chains: one failure-mode chain carrying its control measure.
        let chains: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(out.join("fta_chains.json")).unwrap())
                .unwrap();
        let chains = chains.as_array().unwrap();
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0]["fm_fqn"], "Lib.FmA");
        assert_eq!(chains[0]["control_measures"][0], "Lib.CmA");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn run_fta_aggregates_multiple_diagrams() {
        let dir = unique_dir("multi");
        write(&dir.join("fta_metamodel.puml"), TEST_METAMODEL);
        let a = dir.join("a.puml");
        let b = dir.join("b.puml");
        write(&a, &diagram("FM A", "Lib.FmA", "Lib.CmA"));
        write(&b, &diagram("FM B", "Lib.FmB", "Lib.CmB"));
        let out = dir.join("out");

        let args = args_for(&out, &[&a, &b]);
        run_fta(&args, out.to_str().unwrap(), LogLevel::Warn).expect("run_fta");

        let chains: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(out.join("fta_chains.json")).unwrap())
                .unwrap();
        let fqns: Vec<&str> = chains
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["fm_fqn"].as_str().unwrap())
            .collect();
        assert!(fqns.contains(&"Lib.FmA"));
        assert!(fqns.contains(&"Lib.FmB"));

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
        write(&f1, &diagram("FM A", "Lib.FmA", "Lib.CmA"));
        write(&f2, &diagram("FM B", "Lib.FmB", "Lib.CmB"));
        let out = dir.join("out");

        let args = args_for(&out, &[&f1, &f2]);
        let err = run_fta(&args, out.to_str().unwrap(), LogLevel::Warn).unwrap_err();
        assert!(err.to_string().contains("duplicate FTA diagram basename"));

        fs::remove_dir_all(&dir).ok();
    }
}
