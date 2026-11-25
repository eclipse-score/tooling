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
"""Shared helpers for L2 analysis-contract tests (rules_testing)."""

load("@rules_testing//lib:truth.bzl", "matching")

# Action mnemonic -> flag that is passed only for maturity = "development".
MATURITY_FLAGS = {
    "ComponentTestCaseCoverageLockCheck": "--allow-check-failures",
    "DependableElementValidate": "--warn-on-errors",
    "TrlcChecks": "--warn-only",
}

def find_action(env, target, mnemonic, output_basename = None):
    """Returns the single action of `target` matching mnemonic and output basename.

    Fails the test if zero or several actions match.
    """
    matches = [
        action
        for action in target.actions
        if action.mnemonic == mnemonic and
           (output_basename == None or
            any([out.basename == output_basename for out in action.outputs.to_list()]))
    ]
    if len(matches) != 1:
        env.fail("expected exactly 1 {} action{}, found {}: {}".format(
            mnemonic,
            "" if output_basename == None else " writing " + output_basename,
            len(matches),
            sorted([
                "{}({})".format(a.mnemonic, ",".join([o.basename for o in a.outputs.to_list()]))
                for a in target.actions
            ]),
        ))
        return None
    return matches[0]

def file_write_json(env, target, output_basename):
    """Returns the decoded JSON content of the FileWrite action producing `output_basename`."""
    action = find_action(env, target, "FileWrite", output_basename)
    if action == None:
        return None
    return json.decode(action.content)

def template_substitutions(env, target, output_basename):
    """Returns the substitutions dict of the TemplateExpand action producing `output_basename`."""
    action = find_action(env, target, "TemplateExpand", output_basename)
    if action == None:
        return {}
    return action.substitutions

def output_path(env, target, mnemonic, output_basename):
    """Returns the exec-root relative path of an action output."""
    action = find_action(env, target, mnemonic, output_basename)
    if action == None:
        return None
    return [o for o in action.outputs.to_list() if o.basename == output_basename][0].path

def argv_of(env, target, mnemonic):
    """Returns the argv list of the single `mnemonic` action, or [] if it is missing."""
    action = find_action(env, target, mnemonic)
    return [] if action == None else action.argv

def flag_value(argv, flag):
    """Returns the argument following `flag` in `argv`, or None if absent."""
    if flag not in argv:
        return None
    index = argv.index(flag) + 1
    return argv[index] if index < len(argv) else None

def expect_maturity_flags(env, target, maturity):
    """Asserts each maturity gate action carries its lenient flag iff maturity is development."""
    for mnemonic in sorted(MATURITY_FLAGS.keys()):
        flag = MATURITY_FLAGS[mnemonic]
        argv = argv_of(env, target, mnemonic)
        if maturity == "development":
            env.expect.that_collection(argv).contains(flag)
        else:
            env.expect.that_collection(argv).contains_none_of([flag])

def expect_failure_message(env, target, message):
    """Asserts the analysis of `target` failed with a message containing `message`."""
    env.expect.that_target(target).failures().contains_predicate(matching.contains(message))
