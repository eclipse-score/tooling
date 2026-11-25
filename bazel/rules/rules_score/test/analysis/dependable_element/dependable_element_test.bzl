# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************
"""L2 tests of the dependable_element analysis contract (rules_testing)."""

load("@rules_testing//lib:analysis_test.bzl", "analysis_test", "test_suite")
load(
    "//lib:analysis.bzl",
    "argv_of",
    "expect_failure_message",
    "expect_maturity_flags",
    "file_write_json",
    "find_action",
    "flag_value",
    "output_path",
    "template_substitutions",
)
load(
    "//lib:fixtures.bzl",
    "fixture_architectural_design",
    "fixture_asr",
    "fixture_component",
    "fixture_component_requirements",
    "fixture_dependable_element",
    "fixture_feature_requirements",
    "fixture_unit",
)

_VERBOSITY_FLAG = str(Label("@score_tooling//bazel/rules/rules_score:verbosity"))
_VALIDATE = "DependableElementValidate"
_ASR_LEVEL = 'requirements "Assumed System Requirements"'
_ASR_TRACE = 'trace to: "Assumed System Requirements"'
_FEAT_TRACE = 'trace to: "Feature Requirements"'

def _traceability_blocks(env, target):
    return template_substitutions(env, target, "de_traceability_config")

def _fixture_label(target, subject_suffix, fixture_name):
    """Label of the fixture `fixture_name` created next to the subject `<prefix><subject_suffix>`."""
    prefix = target.label.name[:-len(subject_suffix)]
    return target.label.same_package_label(prefix + fixture_name)

def _validation_bundle_impl(env, target):
    bundle = file_write_json(env, target, "validation_inputs.json")
    env.expect.that_dict(bundle).keys().contains_exactly(
        ["architecture", "component_diagrams", "sequence_diagrams", "link_diagrams"],
    )
    env.expect.that_str(bundle["architecture"]).equals(
        output_path(env, target, "FileWrite", "architecture.json"),
    )
    env.expect.that_collection(bundle["component_diagrams"]).contains_exactly([])
    env.expect.that_collection(bundle["sequence_diagrams"]).contains_exactly([])
    env.expect.that_collection(bundle["link_diagrams"]).contains_exactly([])

def _validation_bundle_test(name):
    subject = fixture_dependable_element(name + "_de")
    analysis_test(name = name, impl = _validation_bundle_impl, target = subject)

def _validation_bundle_diagrams_impl(env, target):
    bundle = file_write_json(env, target, "validation_inputs.json")
    diagrams = bundle["component_diagrams"]
    env.expect.that_collection(diagrams).has_size(1)
    if len(diagrams) != 1:
        return
    env.expect.that_str(diagrams[0]).contains("design")
    inputs = find_action(env, target, _VALIDATE).inputs.to_list()
    env.expect.that_collection([f.path for f in inputs]).contains(diagrams[0])

def _validation_bundle_diagrams_test(name):
    subject = fixture_dependable_element(
        name + "_de",
        designs = [fixture_architectural_design(name + "_design")],
    )
    analysis_test(name = name, impl = _validation_bundle_diagrams_impl, target = subject)

def _architecture_json_impl(env, target):
    architecture = file_write_json(env, target, "architecture.json")["components"]
    labels = {
        key: _fixture_label(target, "_de_index", "_" + key)
        for key in ["unit_a", "unit_b", "nested", "parent"]
    }
    labels["de"] = _fixture_label(target, "_index", "")
    label = {key: str(value) for key, value in labels.items()}
    design = {key: value.name for key, value in labels.items()}

    env.expect.that_collection(architecture.keys()).contains_exactly(
        [label["de"], label["parent"], label["nested"]],
    )
    env.expect.that_dict(architecture[label["de"]]).contains_exactly({
        "design_name": design["de"],
        "components": [{"label": label["parent"], "design_name": design["parent"]}],
    })
    env.expect.that_dict(architecture[label["parent"]]).contains_exactly({
        "design_name": design["parent"],
        "components": [{"label": label["nested"], "design_name": design["nested"]}],
        "units": [{"label": label["unit_a"], "design_name": design["unit_a"]}],
    })
    env.expect.that_dict(architecture[label["nested"]]).contains_exactly({
        "design_name": design["nested"],
        "units": [{"label": label["unit_b"], "design_name": design["unit_b"]}],
    })

def _architecture_json_test(name):
    unit_a = fixture_unit(name + "_unit_a")
    unit_b = fixture_unit(name + "_unit_b")
    nested = fixture_component(name + "_nested", components = [unit_b])
    parent = fixture_component(name + "_parent", components = [nested, unit_a])
    subject = fixture_dependable_element(name + "_de", components = [parent])
    analysis_test(name = name, impl = _architecture_json_impl, target = subject)

def _validate_argv_impl(env, target):
    argv = find_action(env, target, _VALIDATE).argv
    env.expect.that_str(flag_value(argv, "--profile")).equals("dependable-element")
    env.expect.that_str(flag_value(argv, "--log-level")).equals("warn")
    env.expect.that_str(flag_value(argv, "--inputs")).contains("validation_inputs.json")
    env.expect.that_str(flag_value(argv, "--output")).contains("validation.log")

def _validate_argv_test(name):
    subject = fixture_dependable_element(name + "_de")
    analysis_test(name = name, impl = _validate_argv_impl, target = subject)

def _maturity_subject(name, maturity):
    return fixture_dependable_element(
        name + "_de",
        maturity = maturity,
        requirements = [fixture_asr(name + "_asr")],
        components = [fixture_component(name + "_component", with_coverage_lock = True)],
    )

def _maturity_release_impl(env, target):
    expect_maturity_flags(env, target, "release")

def _maturity_release_test(name):
    analysis_test(name = name, impl = _maturity_release_impl, target = _maturity_subject(name, "release"))

def _maturity_development_impl(env, target):
    expect_maturity_flags(env, target, "development")

def _maturity_development_test(name):
    analysis_test(name = name, impl = _maturity_development_impl, target = _maturity_subject(name, "development"))

def _coverage_lock_input_impl(env, target):
    inputs = find_action(env, target, "ComponentTestCaseCoverageLockCheck").inputs.to_list()
    env.expect.that_collection([f.basename for f in inputs]).contains("test_case_coverage.lock.yaml")

def _coverage_lock_input_test(name):
    subject = fixture_dependable_element(
        name + "_de",
        components = [fixture_component(name + "_component", with_coverage_lock = True)],
    )
    analysis_test(name = name, impl = _coverage_lock_input_impl, target = subject)

def _integrity_violation_impl(env, target):
    low = _fixture_label(target, "_high_index", "_low_index")
    expect_failure_message(
        env,
        target,
        "Integrity level violation: '{}' (level B) depends on '{}' (level A). ".format(target.label, low) +
        "A dependable element must not depend on elements with a lower integrity level.",
    )

def _integrity_violation_test(name):
    fixture_dependable_element(name + "_low", integrity_level = "A")
    subject = fixture_dependable_element(
        name + "_high",
        integrity_level = "B",
        deps = [":" + name + "_low"],
    )
    analysis_test(
        name = name,
        impl = _integrity_violation_impl,
        target = subject,
        expect_failure = True,
    )

def _integrity_violation_development_impl(env, target):
    env.expect.that_target(target).failures().contains_exactly([])

def _integrity_violation_development_test(name):
    fixture_dependable_element(name + "_low", integrity_level = "A", maturity = "development")
    subject = fixture_dependable_element(
        name + "_high",
        integrity_level = "B",
        maturity = "development",
        deps = [":" + name + "_low"],
    )
    analysis_test(name = name, impl = _integrity_violation_development_impl, target = subject)

def _verbosity_impl(env, target):
    env.expect.that_str(flag_value(argv_of(env, target, _VALIDATE), "--log-level")).equals("debug")

def _verbosity_test(name):
    subject = fixture_dependable_element(name + "_de")
    analysis_test(
        name = name,
        impl = _verbosity_impl,
        target = subject,
        config_settings = {_VERBOSITY_FLAG: "debug"},
    )

def _asr_level_impl(env, target):
    blocks = _traceability_blocks(env, target)
    asr = blocks["{ASSUMED_SYSTEM_REQ_BLOCK}"]
    env.expect.that_str(asr).contains(_ASR_LEVEL)
    env.expect.that_str(asr).contains("asr.lobster")
    env.expect.that_str(asr).contains('requires: "Feature Requirements" or "Component Requirements"')
    env.expect.that_bool("asr.lobster" in blocks["{FEAT_REQ_BLOCK}"]).equals(False)
    env.expect.that_str(blocks["{FEAT_REQ_BLOCK}"]).contains(_ASR_TRACE)
    env.expect.that_str(blocks["{COMP_REQ_BLOCK}"]).contains(_FEAT_TRACE)
    env.expect.that_str(blocks["{COMP_REQ_BLOCK}"]).contains(_ASR_TRACE)

def _asr_level_test(name):
    asr = fixture_asr(name + "_asr")
    feat = fixture_feature_requirements(name + "_feat", deps = [asr])
    comp_req = fixture_component_requirements(name + "_comp_req", deps = [asr, feat])
    subject = fixture_dependable_element(
        name + "_de",
        maturity = "development",
        requirements = [feat],
        components = [fixture_component(name + "_component", requirements = [comp_req])],
    )
    analysis_test(name = name, impl = _asr_level_impl, target = subject)

def _asr_direct_impl(env, target):
    blocks = _traceability_blocks(env, target)
    asr = blocks["{ASSUMED_SYSTEM_REQ_BLOCK}"]
    comp = blocks["{COMP_REQ_BLOCK}"]
    env.expect.that_str(asr).contains("asr.lobster")
    env.expect.that_bool("requires:" in asr).equals(False)
    env.expect.that_str(blocks["{FEAT_REQ_BLOCK}"]).equals("")
    env.expect.that_str(comp).contains(_ASR_TRACE)
    env.expect.that_bool(_FEAT_TRACE in comp).equals(False)

def _asr_direct_test(name):
    asr = fixture_asr(name + "_asr")
    comp_req = fixture_component_requirements(name + "_comp_req", deps = [asr])
    subject = fixture_dependable_element(
        name + "_de",
        maturity = "development",
        requirements = [asr],
        components = [fixture_component(name + "_component", requirements = [comp_req])],
    )
    analysis_test(name = name, impl = _asr_direct_impl, target = subject)

def _asr_only_impl(env, target, expect_checking_level):
    comp = _traceability_blocks(env, target)["{COMP_REQ_BLOCK}"]
    if expect_checking_level:
        env.expect.that_str(comp).contains('requirements "Component Requirements"')
        env.expect.that_str(comp).contains(_ASR_TRACE)
    else:
        env.expect.that_str(comp).equals("")

def _asr_only_release_impl(env, target):
    _asr_only_impl(env, target, True)

def _asr_only_release_test(name):
    subject = fixture_dependable_element(name + "_de", requirements = [fixture_asr(name + "_asr")])
    analysis_test(name = name, impl = _asr_only_release_impl, target = subject)

def _asr_only_development_impl(env, target):
    _asr_only_impl(env, target, False)

def _asr_only_development_test(name):
    subject = fixture_dependable_element(
        name + "_de",
        maturity = "development",
        requirements = [fixture_asr(name + "_asr")],
    )
    analysis_test(name = name, impl = _asr_only_development_impl, target = subject)

def dependable_element_test_suite(name):
    """Registers the dependable_element analysis tests as test suite `name`."""
    test_suite(
        name = name,
        tests = [
            _validation_bundle_test,
            _validation_bundle_diagrams_test,
            _architecture_json_test,
            _validate_argv_test,
            _maturity_release_test,
            _maturity_development_test,
            _coverage_lock_input_test,
            _integrity_violation_test,
            _integrity_violation_development_test,
            _verbosity_test,
            _asr_level_test,
            _asr_direct_test,
            _asr_only_release_test,
            _asr_only_development_test,
        ],
    )
