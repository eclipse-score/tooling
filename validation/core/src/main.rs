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

//! Validation CLI entrypoint.
//!
//! Supports architecture and design validations inferred from provided
//! input types.

use std::fs;
use std::mem;
use std::process;

use clap::Parser;
use validation::{
    validate_bazel_component, validate_component_class, BazelArchitecture, BazelInput, BazelReader,
    ClassDiagramIndex, ClassDiagramInputs, ClassDiagramReader, ComponentDiagramArchitecture,
    ComponentDiagramInputs, ComponentDiagramReader, Errors, Reader, RequiredInput,
    SelectedValidator, ValidatorSpec, ALL_VALIDATORS,
};

#[derive(Parser, Debug)]
#[command(name = "validation")]
#[command(version = "1.0")]
#[command(about = "Validate architecture and design consistency from PlantUML exports")]
struct Args {
    #[arg(long)]
    architecture_json: Option<String>,

    #[arg(long = "component-fbs", num_args = 1..)]
    component_fbs: Option<Vec<String>>,

    #[arg(long = "class-fbs", num_args = 1..)]
    class_fbs: Option<Vec<String>>,

    #[arg(long)]
    output: Option<String>,
}

struct ValidationCliInputs {
    architecture_json: Option<String>,
    component_fbs: Vec<String>,
    class_fbs: Vec<String>,
}

struct ValidationContext {
    base_errors: Errors,
    bazel: Option<BazelArchitecture>,
    component: Option<ComponentDiagramArchitecture>,
    class: Option<ClassDiagramIndex>,
}

impl ValidationContext {
    fn has_input(&self, input: RequiredInput) -> bool {
        match input {
            RequiredInput::Bazel => self.bazel.is_some(),
            RequiredInput::Component => self.component.is_some(),
            RequiredInput::Class => self.class.is_some(),
        }
    }
}

fn read_and_convert<R, O>(
    input: &R::Input,
    errors: &mut Errors,
    convert: impl Fn(R::Raw, &mut Errors) -> O,
) -> Result<Option<O>, String>
where
    R: Reader,
{
    if !R::is_present(input) {
        return Ok(None);
    }

    let raw = R::read(input).map_err(|e| e.to_string())?;
    Ok(Some(convert(raw, errors)))
}

fn run(args: Args) -> Result<(), String> {
    let inputs = ValidationCliInputs {
        architecture_json: args.architecture_json,
        component_fbs: args.component_fbs.unwrap_or_default(),
        class_fbs: args.class_fbs.unwrap_or_default(),
    };

    let mut context = build_validation_context(inputs)?;
    let validators = resolve_validators(&context)?;

    run_selected_validators(args.output.as_deref(), &validators, &mut context)
}

fn resolve_validators(context: &ValidationContext) -> Result<Vec<SelectedValidator>, String> {
    let inferred = ALL_VALIDATORS
        .iter()
        .copied()
        .filter(|validator| validator.can_run(|input| context.has_input(input)))
        .collect::<Vec<_>>();

    if inferred.is_empty() {
        Err(
            "Unable to infer any validation to run from inputs. Provide compatible input files (for example: --architecture-json with --component-fbs, or --component-fbs with --class-fbs)."
                .to_string(),
        )
    } else {
        Ok(inferred)
    }
}

fn run_selected_validators(
    output_path: Option<&str>,
    validators: &[SelectedValidator],
    context: &mut ValidationContext,
) -> Result<(), String> {
    let mut errors = mem::take(&mut context.base_errors);

    for validator in validators {
        merge_errors(&mut errors, run_validator(*validator, context));
    }

    finish_validation(output_path, &errors)
}

fn run_validator(validator: SelectedValidator, context: &ValidationContext) -> Errors {
    match validator {
        SelectedValidator::BazelComponent => {
            let (bazel, component) = bazel_component_refs(context)
                .expect("BazelComponent validator requires Bazel and component inputs");
            validate_bazel_component(bazel, component, Errors::default())
        }
        SelectedValidator::ComponentClass => {
            let (component, class) = component_class_refs(context)
                .expect("ComponentClass validator requires component and class inputs");
            validate_component_class(component, class, Errors::default())
        }
    }
}

fn build_validation_context(inputs: ValidationCliInputs) -> Result<ValidationContext, String> {
    let mut errors = Errors::default();
    let bazel = match inputs.architecture_json.as_deref() {
        Some(path) => read_and_convert::<BazelReader, BazelArchitecture>(
            path,
            &mut errors,
            |raw: BazelInput, errs| raw.to_bazel_architecture(errs),
        )?,
        None => None,
    };
    let component = read_and_convert::<ComponentDiagramReader, ComponentDiagramArchitecture>(
        inputs.component_fbs.as_slice(),
        &mut errors,
        |raw: ComponentDiagramInputs, errs| raw.to_diagram_architecture(errs),
    )?;
    let class = read_and_convert::<ClassDiagramReader, ClassDiagramIndex>(
        inputs.class_fbs.as_slice(),
        &mut errors,
        |raw: ClassDiagramInputs, errs| raw.to_class_diagram_index(errs),
    )?;

    Ok(ValidationContext {
        base_errors: errors,
        bazel,
        component,
        class,
    })
}

fn bazel_component_refs(
    context: &ValidationContext,
) -> Option<(&BazelArchitecture, &ComponentDiagramArchitecture)> {
    Some((context.bazel.as_ref()?, context.component.as_ref()?))
}

fn component_class_refs(
    context: &ValidationContext,
) -> Option<(&ComponentDiagramArchitecture, &ClassDiagramIndex)> {
    Some((context.component.as_ref()?, context.class.as_ref()?))
}

fn merge_errors(target: &mut Errors, incoming: Errors) {
    target.messages.extend(incoming.messages);
    if !incoming.debug_output.is_empty() {
        if !target.debug_output.is_empty() {
            target.debug_output.push_str("\n\n");
        }
        target.debug_output.push_str(&incoming.debug_output);
    }
}

fn finish_validation(output_path: Option<&str>, errors: &Errors) -> Result<(), String> {
    if let Some(path) = output_path {
        write_log(path, errors)?;
    }

    if errors.is_empty() {
        Ok(())
    } else {
        let details = errors
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| format!("  [{}] {}", i + 1, msg))
            .collect::<Vec<_>>()
            .join("\n\n");
        let output = format!(
            "Verification FAILED ({} error(s)):\n\n{}",
            errors.messages.len(),
            details
        );
        Err(output)
    }
}

fn write_log(path: &str, errors: &Errors) -> Result<(), String> {
    let content = if errors.is_empty() {
        format!("PASS\n\n{}", errors.debug_output)
    } else {
        let details = errors
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| format!("[{}] {}", i + 1, msg))
            .collect::<Vec<_>>()
            .join("\n\n");
        let mut s = format!(
            "FAILED ({} error(s)):\n\n{}",
            errors.messages.len(),
            details
        );
        s.push_str("\n--- Debug Information ---\n\n");
        s.push_str(&errors.debug_output);
        s
    };
    fs::write(path, content).map_err(|e| format!("Failed to write output file {path}: {e}"))
}

fn main() {
    if let Err(msg) = run(Args::parse()) {
        eprintln!("{msg}");
        process::exit(1);
    }
}
