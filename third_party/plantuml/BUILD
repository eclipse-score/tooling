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

load("@rules_java//java:defs.bzl", "java_binary")

java_binary(
    name = "plantuml",
    main_class = "net.sourceforge.plantuml.Run",
    visibility = ["//visibility:public"],
    runtime_deps = [
        "@blueprint_maven_dependencies//:net_sourceforge_plantuml_plantuml",
    ],
)

# Fallback font + sun.awt.FontConfiguration template so PlantUML gets usable
# text metrics even when the execution environment has no native fontconfig
# library and/or no installed fonts at all (e.g. a minimal Docker build
# image). Without this, the JVM's AWT font manager fails hard the first time
# PlantUML asks for a font metric ("Fontconfig head is null, check your
# fonts or fonts configuration") regardless of -Djava.awt.headless.
# Consumed by sphinx_module.bzl's _hermetic_tool_env() /
# sphinx_conf_helpers.resolve_plantuml_fontconfig(), which resolves
# fontconfig.properties.tpl's {font_path} placeholder to
# LiberationSans-Regular.ttf's absolute runtime path and passes the result
# via -Dsun.awt.fontconfig.
# See LiberationSans-LICENSE.txt for the font's (SIL OFL 1.1) license.
filegroup(
    name = "fontconfig_fallback",
    srcs = [
        "LiberationSans-Regular.ttf",
        "fontconfig.properties.tpl",
    ],
    visibility = ["//visibility:public"],
)
