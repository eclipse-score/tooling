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
#
# Minimal sun.awt.FontConfiguration properties file, mapping every logical
# Java font (Serif, SansSerif, Monospaced, Dialog, DialogInput) to the single
# bundled LiberationSans-Regular.ttf fallback font.
#
# Why this exists: OpenJDK on Linux normally builds its logical-font mapping
# by querying the native libfontconfig library and the host's installed
# fonts. In a minimal container/toolchain that has neither, that query fails
# and BOTH the native path and this file's absence cause
# sun.awt.FontConfiguration to throw "Fontconfig head is null, check your
# fonts or fonts configuration" the first time any AWT font metric is
# requested (see PlantUML's Run.forceOpenJdkResourceLoad, which calls
# Font.getStringBounds() specifically to surface this early). Pointing the
# JVM at this file via -Dsun.awt.fontconfig=<resolved path> makes
# sun.awt.X11FontManager use it directly instead of querying the native
# library, so PlantUML gets usable (if visually approximate) text metrics
# regardless of what fonts, if any, the host/container provides.
#
# {font_path} is substituted at Sphinx-config time (see
# sphinx_conf_helpers.resolve_plantuml_fontconfig) with the absolute,
# execroot-resolved path to the bundled LiberationSans-Regular.ttf runfile -- it cannot
# be a path literal here because the actual on-disk location depends on the
# Bazel sandbox/runfiles layout of whichever action executes PlantUML.
version=1

sequence.allfonts=default

allfonts.default=default

filename.default={font_path}
