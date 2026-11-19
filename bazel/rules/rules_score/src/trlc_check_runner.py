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

"""``trlc`` with additional Python checks.

Runs the stock ``trlc`` CLI and, after its user-defined checks ran, calls every
plugin named with ``--check``. A plugin is a function
``check(symbols, mh) -> bool`` that receives the resolved symbol table and the
message handler. It reports findings through ``mh.error(..., fatal=False)`` /
``mh.warning(...)``, so the summary, ``--log`` and the exit code behave as for
built-in checks. No plugin is run unless it is named; the caller (the Bazel
rule) lists them.

Additional options:

``--check MODULE[:FUNC]``
    Plugin to run (``FUNC`` defaults to ``check``); can be given more than once.
``--warn-only``
    Report plugin findings but exit with 0. Errors of ``trlc`` itself (parse,
    resolve and model ``checks`` errors) still fail the run.
``--log FILE``
    Write all output to FILE (one value, unlike ``trlc --log``, so it cannot
    swallow the input files). When the run reports errors the log is also
    echoed to stderr.

All other arguments are passed on to ``trlc``.
"""

import importlib
import sys

from trlc import trlc as trlc_cli

_DEFAULT_FUNC = "check"


def _load_plugin(spec):
    module_name, _, func_name = spec.partition(":")
    try:
        module = importlib.import_module(module_name)
        return getattr(module, func_name or _DEFAULT_FUNC)
    except (ImportError, AttributeError) as error:
        raise SystemExit("trlc_check_runner: cannot load check '%s': %s" % (spec, error))


def _split_args(argv):
    """Return (plugin specs, warn_only, log path, arguments for trlc)."""
    specs = []
    warn_only = False
    log_path = None
    rest = []
    args = iter(argv)
    for arg in args:
        if arg in ("--check", "--log"):
            try:
                value = next(args)
            except StopIteration:
                raise SystemExit("trlc_check_runner: %s requires a value" % arg)
            if arg == "--check":
                specs.append(value)
            else:
                log_path = value
        elif arg.startswith("--check="):
            specs.append(arg.split("=", 1)[1])
        elif arg.startswith("--log="):
            log_path = arg.split("=", 1)[1]
        elif arg == "--warn-only":
            warn_only = True
        else:
            rest.append(arg)
    if log_path:
        rest.append("--log=" + log_path)
    return specs, warn_only, log_path, rest


def _run_trlc(plugins, args):
    """Run trlc with the plugins; return (exit code, whether trlc's own checks passed)."""
    original = trlc_cli.Source_Manager.perform_checks
    state = {"trlc_ok": False}

    def perform_checks(self):
        ok = original(self)
        state["trlc_ok"] = ok
        for plugin in plugins:
            ok = bool(plugin(self.stab, self.mh)) and ok
        return ok

    saved_argv = sys.argv
    trlc_cli.Source_Manager.perform_checks = perform_checks
    sys.argv = ["trlc"] + args
    try:
        return trlc_cli.main(), state["trlc_ok"]
    finally:
        sys.argv = saved_argv
        trlc_cli.Source_Manager.perform_checks = original


def main(argv=None):
    specs, warn_only, log_path, args = _split_args(sys.argv[1:] if argv is None else argv)
    if not hasattr(trlc_cli.Source_Manager, "perform_checks"):
        raise SystemExit("trlc_check_runner: unsupported TRLC version, Source_Manager.perform_checks is missing")
    plugins = []
    for spec in specs:
        plugin = _load_plugin(spec)
        if plugin not in plugins:
            plugins.append(plugin)

    exit_code, trlc_ok = _run_trlc(plugins, args)

    if exit_code != 0 and log_path:
        with open(log_path, encoding="utf-8") as log:
            print(log.read(), file=sys.stderr)
    return 0 if warn_only and trlc_ok else exit_code


if __name__ == "__main__":
    sys.exit(main())
