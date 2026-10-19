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
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use activity_diagram::ActivityDiagram;
use activity_serializer::ActivitySerializer;
use class_serializer::ClassSerializer;
use component_serializer::ComponentSerializer;
use sequence_serializer::SequenceSerializer;

use puml_fta::{merge_root_causes, render_trlc_stub, FaultTree, FtaModel};
use puml_idmap::{write_empty_idmap_to_file, write_idmap_to_file, IdMapModel};
use puml_lobster::{write_lobster_to_file, LobsterModel};
use puml_parser::{
    DiagramParser, ErrorLocation, Preprocessor, ProcedureParserService, PumlActivityParser,
    PumlClassParser, PumlComponentParser, PumlSequenceParser,
};
use puml_resolver::{
    ActivityResolver, ClassResolver, ComponentResolver, DiagramResolver, DiagramSource,
    SequenceResolver,
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
    /// When set, every input diagram is treated as an FTA and
    /// ``fta_events.trlc`` (the generated ``RootCause`` records; see
    /// ``--fta-package``) is written to this directory.  No
    /// FlatBuffers / component processing is performed in this mode.
    #[arg(
        long,
        conflicts_with_all = ["fbs_output_dir", "lobster_output_dir", "source_name", "output_stem"]
    )]
    fta_output_dir: Option<String>,

    /// TRLC package name for the generated ``fta_events.trlc`` stub file.
    /// Required together with ``--fta-output-dir``.
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

#[derive(Copy, Clone, ValueEnum, Debug)]
enum DiagramType {
    None,
    Activity,
    Component,
    Deployment,
    Class,
    Sequence,
}

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
    execute(&args, log_level)
}

fn execute(args: &Args, log_level: LogLevel) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = &args.fta_output_dir {
        // FTA mode short-circuits the normal output passes.
        let Some(fta_package) = &args.fta_package else {
            return Err("--fta-package is required together with --fta-output-dir".into());
        };
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

    let fbs_output_dir = prepare_output_dir(&args.fbs_output_dir)?;
    let lobster_output_dir = prepare_output_dir(&args.lobster_output_dir)?;
    let idmap_output_dir = prepare_output_dir(&args.idmap_output_dir)?;

    let emit_debug_json = log_level.to_level_filter() >= log::LevelFilter::Debug;

    // Two input files with the same file stem (in different directories) would
    // otherwise silently overwrite each other's `<stem>.idmap.json`; track the
    // output paths already claimed this run and fail fast on a collision.
    let mut seen_idmap_outputs: HashSet<PathBuf> = HashSet::new();

    let file_list = collect_files_from_args(args)?;

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
    let mut naming_paths: Vec<PathBuf> = Vec::with_capacity(file_list.len());
    let mut source_files: Vec<String> = Vec::with_capacity(file_list.len());
    let mut diagram_names: Vec<Option<String>> = Vec::with_capacity(file_list.len());
    let mut parsed_diagrams: Vec<ParsedDiagram> = Vec::with_capacity(file_list.len());

    for (i, (path, content)) in preprocessed.iter().enumerate() {
        // `naming_path` names the output files; `source_file` is the path
        // embedded in outputs. Parsers only stamp `source_path` into
        // `SourceLocation`s, so a path built from `source_file` suffices.
        let output_stem = args.output_stem.get(i).map(String::as_str);
        let source_name = args.source_name.get(i).map(String::as_str);
        let naming_path = output_naming_path(output_stem, path).into_owned();
        let (source_file, source_path) = output_source_path(source_name, path);

        let parsed_content = parse_puml_file(&source_path, content, log_level, args.diagram_type)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        if emit_debug_json {
            if let Some(ref dir) = fbs_output_dir {
                write_json_to_file(&parsed_content, &naming_path, dir, "raw.ast")?;
            }
        }

        // The resolver discards the `@startuml <name>`; keep it.
        diagram_names.push(match &parsed_content {
            ParsedDiagram::Component(doc) => doc.name.clone(),
            _ => None,
        });
        naming_paths.push(naming_path);
        source_files.push(source_file);
        parsed_diagrams.push(parsed_content);
    }

    debug!("Resolving {} diagram(s)", parsed_diagrams.len());
    let resolved_diagrams = resolve_parsed_diagrams(&source_files, &parsed_diagrams)
        .map_err(|e| format!("Resolve error: {e}"))?;

    for (i, logic_result) in resolved_diagrams.into_iter().enumerate() {
        let path = &file_list[i];
        let naming_path = &naming_paths[i];
        let source_file = &source_files[i];

        debug!(
            "Successfully resolved PlantUML document: {}",
            path.display()
        );
        if emit_debug_json {
            if let Some(ref dir) = fbs_output_dir {
                write_json_to_file(&logic_result, naming_path, dir, "logic.ast")?;
            }
        }

        let fbs_buffer = serialize_resolved_diagram(&logic_result);
        if let Some(ref dir) = fbs_output_dir {
            write_fbs_to_file(&fbs_buffer, naming_path, dir)?;
        }

        if let Some(ldir) = &lobster_output_dir {
            let lobster_model = match &logic_result {
                ResolvedDiagram::Component(model) => LobsterModel::Component(model),
                ResolvedDiagram::Class(model) => LobsterModel::Class(model),
                ResolvedDiagram::Activity(_) => LobsterModel::Empty,
                ResolvedDiagram::Sequence(_) => LobsterModel::Empty,
            };
            write_lobster_to_file(lobster_model, naming_path, source_file, ldir)?;
        }

        if let Some(idir) = &idmap_output_dir {
            claim_idmap_output(naming_path, idir, &mut seen_idmap_outputs, "input")?;

            match idmap_model_for(&logic_result) {
                Some(idmap_model) => {
                    write_idmap_to_file(
                        idmap_model,
                        naming_path,
                        Some(source_file),
                        diagram_names[i].as_deref(),
                        idir,
                    )?;
                }
                None => {
                    write_empty_idmap_to_file(naming_path, Some(source_file), idir)?;
                }
            }
        }
    }

    debug!("Parsing completed");
    Ok(())
}

/// Resolves the diagrams of each kind in `parsed` as one group; `names` (one
/// per diagram) attribute errors to files. Results keep the order of `parsed`.
fn resolve_parsed_diagrams(
    names: &[String],
    parsed: &[ParsedDiagram],
) -> Result<Vec<ResolvedDiagram>, Box<dyn std::error::Error>> {
    let mut activity = Vec::new();
    let mut component = Vec::new();
    let mut class = Vec::new();
    let mut sequence = Vec::new();
    for (i, doc) in parsed.iter().enumerate() {
        match doc {
            ParsedDiagram::Activity(d) => activity.push((i, d)),
            ParsedDiagram::Component(d) => component.push((i, d)),
            ParsedDiagram::Class(d) => class.push((i, d)),
            ParsedDiagram::Sequence(d) => sequence.push((i, d)),
        }
    }

    let mut results: Vec<Option<ResolvedDiagram>> = (0..parsed.len()).map(|_| None).collect();
    resolve_bucket::<ActivityResolver>(names, &activity, ResolvedDiagram::Activity, &mut results)?;
    resolve_bucket::<ComponentResolver>(
        names,
        &component,
        ResolvedDiagram::Component,
        &mut results,
    )?;
    resolve_bucket::<ClassResolver>(names, &class, ResolvedDiagram::Class, &mut results)?;
    resolve_bucket::<SequenceResolver>(names, &sequence, ResolvedDiagram::Sequence, &mut results)?;

    Ok(results
        .into_iter()
        .map(|r| r.expect("every diagram is in exactly one bucket"))
        .collect())
}

/// Resolves the `(index, document)` entries of one kind as a group and stores
/// each result at its index.
fn resolve_bucket<R>(
    names: &[String],
    bucket: &[(usize, &R::Document)],
    wrap: fn(R::Output) -> ResolvedDiagram,
    results: &mut [Option<ResolvedDiagram>],
) -> Result<(), Box<dyn std::error::Error>>
where
    R: DiagramResolver + Default,
{
    if bucket.is_empty() {
        return Ok(());
    }
    let sources: Vec<DiagramSource<'_, R::Document>> = bucket
        .iter()
        .map(|&(i, document)| DiagramSource {
            name: names[i].as_str(),
            document,
        })
        .collect();
    let resolved = R::resolve_group(&sources)?;
    for (&(i, _), output) in bucket.iter().zip(resolved) {
        results[i] = Some(wrap(output));
    }
    Ok(())
}

/// Creates the output directory of an `--*-output-dir` flag, if given.
fn prepare_output_dir(dir: &Option<String>) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    match dir {
        Some(dir) => {
            let p = PathBuf::from(dir);
            fs::create_dir_all(&p)?;
            Ok(Some(p))
        }
        None => Ok(None),
    }
}

/// Claims `naming_path`'s `<stem>.idmap.json` path; errors if another
/// `input_kind` file claimed it this run.
fn claim_idmap_output(
    naming_path: &Path,
    idir: &Path,
    seen_idmap_outputs: &mut HashSet<PathBuf>,
    input_kind: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = puml_idmap::idmap_output_path(naming_path, idir);
    if !seen_idmap_outputs.insert(output_path.clone()) {
        return Err(format!(
            "duplicate idmap output {}: multiple {} files map to the \
             same <stem>.idmap.json in {}; rename one of the inputs so \
             their file stems differ",
            output_path.display(),
            input_kind,
            idir.display(),
        )
        .into());
    }
    Ok(())
}

/// FTA processing pipeline: parse the fault-tree macro calls straight from
/// each source diagram (the metamodel's ``!procedure`` definitions carry no
/// fault-tree topology themselves) and emit the generated ``fta_events.trlc``
/// stub. Diagrams are analysis-only here; they are staged/rendered as-authored
/// by the Bazel rule, not rewritten by this pipeline.
fn run_fta(
    args: &Args,
    output_dir: &str,
    fta_package: &str,
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

    let mut all_trees: Vec<FaultTree> = Vec::new();
    // Fault trees reference each diagram by basename; two inputs sharing a
    // basename (in different directories) would be indistinguishable downstream.
    let mut seen_basenames: HashSet<String> = HashSet::new();
    // Mirrors the collision guard in the main resolve path: two input files
    // whose file *stem* collides (independent of the basename check above,
    // which only fires on an exact basename match) would otherwise silently
    // overwrite each other's `<stem>.idmap.json`.
    let mut seen_idmap_outputs: HashSet<PathBuf> = HashSet::new();
    // Created once up front (rather than per-file inside the loop below) since
    // the directory is the same for every file this run.
    if let Some(ref idir_str) = args.idmap_output_dir {
        fs::create_dir_all(idir_str)?;
    }

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
        // `$FailureMode(...)` / `$RootCause(...)` / gate macro *calls*, which the
        // procedure parser reads straight from the source (the `!include
        // fta_metamodel.puml` line is inert text).  The diagram is rendered
        // as-authored by Sphinx/PlantUML, which finds the metamodel via the
        // toolchain's global `plantuml.include.path`.
        let source = fs::read_to_string(file.as_path())?;
        let parsed = ProcedureParserService.parse_file(file, &source, log_level)?;
        let model = FtaModel::from_procedure_file(&parsed)?;
        all_trees.extend(model.fault_trees(basename)?);

        if let Some(ref idir_str) = args.idmap_output_dir {
            let idir = PathBuf::from(idir_str);
            claim_idmap_output(file, &idir, &mut seen_idmap_outputs, "FTA input")?;
            let source_file = source_path_for_output(file);
            write_idmap_to_file(
                IdMapModel::Fta(&model),
                file,
                Some(&source_file),
                None,
                &idir,
            )?;
        }

        debug!("Processed FTA diagram: {}", file.display());
    }

    let root_causes = merge_root_causes(&all_trees)?;
    fs::write(
        out.join("fta_events.trlc"),
        render_trlc_stub(fta_package, &root_causes),
    )?;
    Ok(())
}

fn serialize_resolved_diagram(resolved_content: &ResolvedDiagram) -> Vec<u8> {
    match resolved_content {
        ResolvedDiagram::Activity(resolved_content) => {
            ActivitySerializer::serialize(resolved_content)
        }
        ResolvedDiagram::Component(resolved_content) => {
            ComponentSerializer::serialize(resolved_content)
        }
        ResolvedDiagram::Class(resolved_content) => ClassSerializer::serialize(resolved_content),
        ResolvedDiagram::Sequence(resolved_content) => {
            SequenceSerializer::serialize(resolved_content)
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

/// Map a resolved diagram to the `IdMapModel` variant used to build its
/// `.idmap.json`, or `None` for diagram kinds with no linkable elements
/// (currently only `Activity`), which the caller must route to
/// [`write_empty_idmap_to_file`] instead.
fn idmap_model_for(resolved: &ResolvedDiagram) -> Option<IdMapModel<'_>> {
    match resolved {
        ResolvedDiagram::Component(model) => Some(IdMapModel::Component(model)),
        ResolvedDiagram::Class(model) => Some(IdMapModel::Class(model)),
        ResolvedDiagram::Sequence(model) => Some(IdMapModel::Sequence(model)),
        ResolvedDiagram::Activity(_) => None,
    }
}

/// Resolves one diagram alone.
#[cfg(test)]
fn resolve_single(parsed: ParsedDiagram) -> Result<ResolvedDiagram, Box<dyn std::error::Error>> {
    resolve_parsed_diagrams(&[String::new()], &[parsed]).map(|mut resolved| resolved.remove(0))
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
            match PumlComponentParser.parse_file(path, content, log_level) {
                Ok(doc) => {
                    debug!("Successfully parsed PlantUML document: {}", path.display());
                    Ok(ParsedDiagram::Component(doc))
                }
                Err(component_err) => {
                    Err(diagnose_wrong_kind(path, content, log_level, component_err))
                }
            }
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

/// Names the kind of a file that is not a Component diagram, when another
/// parser accepts it; otherwise returns the component parse error.
fn diagnose_wrong_kind(
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
    component_err: <PumlComponentParser as DiagramParser>::Error,
) -> Box<dyn std::error::Error>
where
    <PumlComponentParser as DiagramParser>::Error: std::error::Error + 'static,
{
    let kind = if PumlClassParser.parse_file(path, content, log_level).is_ok() {
        "Class"
    } else if PumlActivityParser
        .parse_file(path, content, log_level)
        .is_ok()
    {
        "Activity"
    } else if PumlSequenceParser
        .parse_file(path, content, log_level)
        .is_ok()
    {
        "Sequence"
    } else {
        return Box::new(component_err);
    };
    format!(
        "{}: expected a Component diagram, found a {kind} diagram",
        path.display()
    )
    .into()
}

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

/// Collects the input files, deduplicated: every `--file` in the given order,
/// then the `--folders` files sorted. All of them are resolved as one group
/// per diagram kind.
fn collect_files_from_args(args: &Args) -> Result<Vec<Rc<PathBuf>>, Box<dyn std::error::Error>> {
    let mut ordered: Vec<Rc<PathBuf>> = Vec::new();
    let mut seen: HashSet<Rc<PathBuf>> = HashSet::new();

    for file_path in &args.file {
        let abs_path = Rc::new(resolve_puml_file(Path::new(file_path))?);
        if !seen.insert(Rc::clone(&abs_path)) {
            return Err(format!("Duplicate --file entry: {file_path}").into());
        }
        ordered.push(abs_path);
    }

    if let Some(folder_path) = &args.folders {
        let mut folder_files: HashSet<Rc<PathBuf>> = HashSet::new();
        collect_puml_files_from_folder(Path::new(folder_path), &mut folder_files)?;
        let mut folder_files: Vec<Rc<PathBuf>> = folder_files.into_iter().collect();
        folder_files.sort();
        for path in folder_files {
            if seen.insert(Rc::clone(&path)) {
                ordered.push(path);
            }
        }
    }

    Ok(ordered)
}

fn resolve_path(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    // When running with 'bazel run', use BUILD_WORKSPACE_DIRECTORY
    let base_dir = match std::env::var("BUILD_WORKSPACE_DIRECTORY") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => std::env::current_dir()?,
    };

    Ok(base_dir.join(path))
}

/// Rejects a per-file flag given neither zero times nor once per input file.
fn require_one_per_file_or_none(
    values: &[String],
    file_count: usize,
    flag_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !values.is_empty() && values.len() != file_count {
        return Err(format!(
            "--{flag_name} must be given exactly once per input file (in the same \
             order as --file) or not at all; got {file_count} input file(s), \
             {} --{flag_name}",
            values.len(),
        )
        .into());
    }
    Ok(())
}

/// Validate a user-supplied `--source-name` value.
///
/// Rejects empty strings and absolute paths: an absolute path would leak the
/// local machine's directory layout into build outputs (breaking build
/// reproducibility) and would defeat the whole purpose of the flag, which is
/// to supply a *stable, workspace-relative* key for cross-diagram matching.
fn validate_source_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if name.is_empty() {
        return Err("--source-name must not be empty".into());
    }
    if Path::new(name).is_absolute() {
        return Err(format!(
            "--source-name must be a workspace-relative path, got an absolute path: {name}"
        )
        .into());
    }
    Ok(())
}

fn source_path_for_output(path: &Path) -> String {
    let workspace_root = std::env::var("BUILD_WORKSPACE_DIRECTORY")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok());

    // Prefer a stable, workspace-relative path so the embedded `source` is
    // reproducible across machines and portable outside Bazel.
    if let Some(root) = workspace_root {
        if let Ok(rel) = path.strip_prefix(&root) {
            return rel.to_string_lossy().into_owned();
        }
    }

    // Already relative: keep it verbatim (still portable).
    if path.is_relative() {
        return path.to_string_lossy().into_owned();
    }

    // Absolute path outside the workspace: never embed the machine-specific
    // absolute path — it leaks local directory layout and breaks build
    // reproducibility. Fall back to the file name only.
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Validate a user-supplied `--output-stem` value.
///
/// Unlike `--source-name` (a full relative path embedded *inside* outputs),
/// this becomes the output *filename* stem itself, so it must be a single
/// path component: rejects empty strings and anything containing a `/` or
/// resolving to `.`/`..`.
fn validate_output_stem(stem: &str) -> Result<(), Box<dyn std::error::Error>> {
    if stem.is_empty() {
        return Err("--output-stem must not be empty".into());
    }
    let mut components = Path::new(stem).components();
    let is_single_normal_component =
        matches!(components.next(), Some(std::path::Component::Normal(_)))
            && components.next().is_none();
    if !is_single_normal_component {
        return Err(format!(
            "--output-stem must be a single path component (no '/', '.', or '..'), got: {stem}"
        )
        .into());
    }
    Ok(())
}

/// Compute the corrected, stable `source_file` string (embedded *inside*
/// outputs) and the `Rc<PathBuf>` that parsers use to stamp `SourceLocation`s,
/// for one input `path`.
///
/// With `--source-name`, both derive from that name so every `SourceLocation`
/// carries the corrected path too. Without it, `source_file` is
/// `source_path_for_output(path)` and `SourceLocation`s keep using `path`
/// itself.
fn output_source_path(source_name: Option<&str>, path: &Rc<PathBuf>) -> (String, Rc<PathBuf>) {
    match source_name {
        Some(name) => {
            let source_file = name.to_string();
            let source_path = Rc::new(PathBuf::from(&source_file));
            (source_file, source_path)
        }
        None => (source_path_for_output(path), Rc::clone(path)),
    }
}

/// Compute the path whose file stem drives every output *filename* for one
/// input `path` (the `write_*_to_file` helpers all derive their output name
/// from a passed-in path's file stem alone).
///
/// `output_stem` (`--output-stem`) overrides the stem outright; otherwise
/// `path` itself is used unchanged.
fn output_naming_path<'a>(output_stem: Option<&str>, path: &'a Path) -> Cow<'a, Path> {
    match output_stem {
        Some(stem) => Cow::Owned(path.with_file_name(stem)),
        None => Cow::Borrowed(path),
    }
}

/// Resolve a `--file`/folder-entry path to an absolute path, checking it is
/// an existing `.puml` file.
fn resolve_puml_file(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let abs_path = resolve_path(path)?;

    if !abs_path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()).into());
    }
    if abs_path.extension().and_then(|ext| ext.to_str()) != Some("puml") {
        return Err(format!("File is not a .puml file: {}", path.display()).into());
    }
    Ok(abs_path)
}

fn collect_puml_files_from_folder(
    dir: &Path,
    file_list: &mut HashSet<Rc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let abs_dir = resolve_path(dir)?;

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
            "--lobster-output-dir",
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

            assert_eq!(
                error,
                format!("d.puml: expected a Component diagram, found a {kind} diagram")
            );
        }
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
        !procedure $FailureMode($name, $alias)\n\
        rectangle \"$name\" as $alias\n\
        !endprocedure\n\
        !procedure $OrGate($alias, $connection)\n\
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
