#!/bin/sh

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

set -eu

TOOL_PATH="${PLANTUML_BIN:-$0}"
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
if [ "${TOOL_PATH#/}" = "${TOOL_PATH}" ]; then
  SEARCH_ROOT="$(pwd -P)"
  while [ "${SEARCH_ROOT}" != "/" ]; do
    if [ -e "${SEARCH_ROOT}/${TOOL_PATH}" ]; then
      SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${SEARCH_ROOT}/${TOOL_PATH}")" && pwd)"
      break
    elif [ -n "${PLANTUML_BIN_RLOC:-}" ] && [ -e "${SEARCH_ROOT}/external/${PLANTUML_BIN_RLOC}" ]; then
      SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${SEARCH_ROOT}/external/${PLANTUML_BIN_RLOC}")" && pwd)"
      break
    fi
    SEARCH_ROOT="$(dirname -- "${SEARCH_ROOT}")"
  done
fi

if [ -d "${SCRIPT_DIR}/plantuml.runfiles" ]; then
  RUNFILES_DIR="${SCRIPT_DIR}/plantuml.runfiles"
  export RUNFILES_DIR
  unset RUNFILES_MANIFEST_FILE 2>/dev/null || true
elif [ -f "${SCRIPT_DIR}/plantuml.runfiles_manifest" ]; then
  RUNFILES_MANIFEST_FILE="${SCRIPT_DIR}/plantuml.runfiles_manifest"
  export RUNFILES_MANIFEST_FILE
  unset RUNFILES_DIR 2>/dev/null || true
fi

if [ -z "${RUNFILES_DIR:-}" ] && [ -z "${RUNFILES_MANIFEST_FILE:-}" ]; then
  case "${SCRIPT_DIR}" in
    */bazel-out/*/bin/external/score_tooling+/*)
      RUNFILES_DIR="${SCRIPT_DIR%%/bazel-out/*}/external"
      export RUNFILES_DIR
      export JAVA_RUNFILES="${RUNFILES_DIR}"
      ;;
  esac
fi

rlocation() {
  location="$1"

  # Try RUNFILES_DIR first (directory-based)
  if [ -n "${RUNFILES_DIR:-}" ]; then
    if [ -e "${RUNFILES_DIR}/${location}" ]; then
      echo "${RUNFILES_DIR}/${location}"
      return 0
    fi
    # Also try with absolute path resolution for symlinks
    if [ -x "${RUNFILES_DIR}/${location}" ] 2>/dev/null; then
      echo "${RUNFILES_DIR}/${location}"
      return 0
    fi
  fi

  # Try RUNFILES_MANIFEST_FILE (manifest-based)
  if [ -f "${RUNFILES_MANIFEST_FILE:-}" ]; then
    grep "^${location} " "${RUNFILES_MANIFEST_FILE}" | head -1 | cut -d' ' -f2-
  fi
}

# Try to locate plantuml_java and sysroot via runfiles manifest/directory first
PLANTUML_JAVA="$(rlocation 'score_tooling/third_party/plantuml/plantuml_java')"
SYSROOT="$(rlocation 'score_tooling/third_party/plantuml/plantuml_sysroot_sysroot')"

# Bazel uses module names (with + for bzlmod) in runfiles paths
if [ -z "${PLANTUML_JAVA}" ]; then
  PLANTUML_JAVA="$(rlocation 'score_tooling+/third_party/plantuml/plantuml_java')"
fi
if [ -z "${SYSROOT}" ]; then
  SYSROOT="$(rlocation 'score_tooling+/third_party/plantuml/plantuml_sysroot_sysroot')"
fi

# Also try with _main (directory-based runfiles) and external prefixes
if [ -z "${PLANTUML_JAVA}" ]; then
  PLANTUML_JAVA="$(rlocation '_main/third_party/plantuml/plantuml_java')"
fi
if [ -z "${SYSROOT}" ]; then
  SYSROOT="$(rlocation '_main/third_party/plantuml/plantuml_sysroot_sysroot')"
fi
if [ -z "${PLANTUML_JAVA}" ]; then
  PLANTUML_JAVA="$(rlocation 'external/score_tooling+/third_party/plantuml/plantuml_java')"
fi
if [ -z "${SYSROOT}" ]; then
  SYSROOT="$(rlocation 'external/score_tooling+/third_party/plantuml/plantuml_sysroot_sysroot')"
fi

# Module names differ between bzlmod and local-main repositories. Resolve the
# exact data-dependency paths without depending on that repository-name spelling.
if [ -n "${RUNFILES_DIR:-}" ] && [ -d "${RUNFILES_DIR}" ]; then
  if [ -z "${PLANTUML_JAVA}" ]; then
    PLANTUML_JAVA="$(find "${RUNFILES_DIR}" -type f -path '*/third_party/plantuml/plantuml_java' -perm -u+x -print -quit 2>/dev/null)"
  fi
  if [ -z "${SYSROOT}" ]; then
    SYSROOT="$(find "${RUNFILES_DIR}" -type d -path '*/third_party/plantuml/plantuml_sysroot_sysroot' -print -quit 2>/dev/null)"
  fi
fi

# Since plantuml_java and plantuml_sysroot are data dependencies of this sh_binary,
# they should be in the same directory as the script in the runfiles layout.
if [ -z "${PLANTUML_JAVA}" ] && [ -x "${SCRIPT_DIR}/plantuml_java" ]; then
  PLANTUML_JAVA="${SCRIPT_DIR}/plantuml_java"
fi
if [ -z "${SYSROOT}" ] && [ -d "${SCRIPT_DIR}/plantuml_sysroot_sysroot" ]; then
  SYSROOT="${SCRIPT_DIR}/plantuml_sysroot_sysroot"
fi

# Search for plantuml_java in common Bazel locations if still not found
if [ -z "${PLANTUML_JAVA}" ] || [ ! -x "${PLANTUML_JAVA}" ]; then
  # Build a comprehensive search path list
  search_paths="${SCRIPT_DIR}"
  search_paths="${search_paths} $(pwd)"

  # Add RUNFILES_DIR-based paths if available
  if [ -n "${RUNFILES_DIR:-}" ]; then
    search_paths="${search_paths} ${RUNFILES_DIR}/score_tooling+/third_party/plantuml"
    search_paths="${search_paths} ${RUNFILES_DIR}/score_tooling/third_party/plantuml"
    search_paths="${search_paths} ${RUNFILES_DIR}/_main/third_party/plantuml"
  fi

  # Add relative paths
  search_paths="${search_paths} external/score_tooling+/third_party/plantuml"
  search_paths="${search_paths} external/score_tooling/third_party/plantuml"
  search_paths="${search_paths} score_tooling+/third_party/plantuml"
  search_paths="${search_paths} ."

  for search_dir in $search_paths; do
    if [ -x "${search_dir}/plantuml_java" ] 2>/dev/null; then
      PLANTUML_JAVA="${search_dir}/plantuml_java"
      break
    fi
  done
fi

# When a runfiles tree is available, never select a launcher outside it. The
# outer execroot launcher has no runfiles tree in a processwrapper sandbox.
if [ -n "${RUNFILES_DIR:-}" ] && [ -d "${RUNFILES_DIR}" ] && [ -n "${PLANTUML_JAVA}" ]; then
  case "${PLANTUML_JAVA}" in
    "${RUNFILES_DIR}"/*) ;;
    *) PLANTUML_JAVA="" ;;
  esac
fi

if [ -z "${PLANTUML_JAVA}" ] || [ ! -x "${PLANTUML_JAVA}" ]; then
  echo "ERROR: could not resolve PlantUML Java launcher" >&2
  echo "DEBUG: SCRIPT_DIR=${SCRIPT_DIR}" >&2
  echo "DEBUG: PLANTUML_BIN=${PLANTUML_BIN:-}" >&2
  echo "DEBUG: RUNFILES_DIR=${RUNFILES_DIR:-}" >&2
  echo "DEBUG: RUNFILES_MANIFEST_FILE=${RUNFILES_MANIFEST_FILE:-}" >&2
  echo "DEBUG: pwd=$(pwd)" >&2
  exit 1
fi

# Convert PLANTUML_JAVA to absolute path if it's relative
# This is critical for Java to correctly locate its runfiles in sandbox contexts
if [ "${PLANTUML_JAVA#/}" = "${PLANTUML_JAVA}" ]; then
  # PLANTUML_JAVA is relative; convert to absolute
  PLANTUML_JAVA="$(cd "$(dirname "${PLANTUML_JAVA}")" 2>/dev/null && pwd)/$(basename "${PLANTUML_JAVA}")" || PLANTUML_JAVA="${PLANTUML_JAVA}"
fi

# Search for sysroot in common Bazel locations if not found yet
if [ -z "${SYSROOT}" ] || [ ! -d "${SYSROOT}" ]; then
  search_paths="${SCRIPT_DIR}"
  search_paths="${search_paths} $(pwd)"

  if [ -n "${RUNFILES_DIR:-}" ]; then
    search_paths="${search_paths} ${RUNFILES_DIR}/score_tooling+/third_party/plantuml"
    search_paths="${search_paths} ${RUNFILES_DIR}/score_tooling/third_party/plantuml"
    search_paths="${search_paths} ${RUNFILES_DIR}/_main/third_party/plantuml"
  fi

  search_paths="${search_paths} external/score_tooling+/third_party/plantuml"
  search_paths="${search_paths} external/score_tooling/third_party/plantuml"
  search_paths="${search_paths} score_tooling+/third_party/plantuml"
  search_paths="${search_paths} ."

  for search_dir in $search_paths; do
    if [ -d "${search_dir}/plantuml_sysroot_sysroot" ] 2>/dev/null; then
      SYSROOT="${search_dir}/plantuml_sysroot_sysroot"
      break
    fi
  done
fi

if [ -z "${SYSROOT}" ] || [ ! -d "${SYSROOT}" ]; then
  echo "ERROR: could not resolve docs runtime sysroot" >&2
  echo "DEBUG: SCRIPT_DIR=${SCRIPT_DIR}" >&2
  echo "DEBUG: RUNFILES_DIR=${RUNFILES_DIR:-}" >&2
  echo "DEBUG: pwd=$(pwd)" >&2
  exit 1
fi

# The outer PlantUML launcher owns the runfiles tree containing plantuml_java.
# Invoke the Java launcher through that tree so Bazel's Java stub can derive
# its own runfiles directory even in a processwrapper sandbox.
PLANTUML_RUNFILES="${SCRIPT_DIR}/plantuml.runfiles"
if [ -d "${PLANTUML_RUNFILES}" ]; then
  for repository in score_tooling+ score_tooling; do
    candidate="${PLANTUML_RUNFILES}/${repository}/third_party/plantuml/plantuml_java"
    if [ -x "${candidate}" ]; then
      PLANTUML_JAVA="${candidate}"
      break
    fi
  done
  export JAVA_RUNFILES="${PLANTUML_RUNFILES}"
fi

# Fontconfig prefixes all absolute paths from fonts.conf with FONTCONFIG_SYSROOT.
# This gives the host JVM access to the packaged fonts without preloading
# fakechroot into Bazel's generated Java launcher and its runfiles machinery.
export FONTCONFIG_SYSROOT="${SYSROOT}"
export FONTCONFIG_FILE="/etc/fonts/fonts.conf"
export FONTCONFIG_PATH="/etc/fonts"

# The bundled JDK (a headless-trimmed distro build) ships no
# lib/fontconfig.properties. sun.awt.FontConfiguration therefore never finds a
# fallback config and throws "Fontconfig head is null" the first time AWT
# needs logical-font metrics -- this is independent of the native
# FONTCONFIG_* variables above, which only affect libfontconfig lookups.
# Generate a minimal properties file mapping the logical font families to the
# DejaVu TrueType files already present in the sysroot, and point the JVM at
# it via -Dsun.awt.fontconfig.
DEJAVU_DIR="${SYSROOT}/usr/share/fonts/truetype/dejavu"
FONTCONFIG_PROPERTIES="$(mktemp "${TMPDIR:-/tmp}/plantuml-fontconfigXXXXXX")"
cat > "${FONTCONFIG_PROPERTIES}" <<EOF
version=1

dialog.plain.latin-1=DejaVu Sans
dialog.bold.latin-1=DejaVu Sans Bold
dialog.italic.latin-1=DejaVu Sans Oblique
dialog.bolditalic.latin-1=DejaVu Sans Bold Oblique

sansserif.plain.latin-1=DejaVu Sans
sansserif.bold.latin-1=DejaVu Sans Bold
sansserif.italic.latin-1=DejaVu Sans Oblique
sansserif.bolditalic.latin-1=DejaVu Sans Bold Oblique

serif.plain.latin-1=DejaVu Serif
serif.bold.latin-1=DejaVu Serif Bold
serif.italic.latin-1=DejaVu Serif Italic
serif.bolditalic.latin-1=DejaVu Serif Bold Italic

monospaced.plain.latin-1=DejaVu Sans Mono
monospaced.bold.latin-1=DejaVu Sans Mono Bold
monospaced.italic.latin-1=DejaVu Sans Mono Oblique
monospaced.bolditalic.latin-1=DejaVu Sans Mono Bold Oblique

dialoginput.plain.latin-1=DejaVu Sans Mono
dialoginput.bold.latin-1=DejaVu Sans Mono Bold
dialoginput.italic.latin-1=DejaVu Sans Mono Oblique
dialoginput.bolditalic.latin-1=DejaVu Sans Mono Bold Oblique

sequence.allfonts=latin-1

filename.DejaVu_Sans=${DEJAVU_DIR}/DejaVuSans.ttf
filename.DejaVu_Sans_Bold=${DEJAVU_DIR}/DejaVuSans-Bold.ttf
filename.DejaVu_Sans_Oblique=${DEJAVU_DIR}/DejaVuSans-Oblique.ttf
filename.DejaVu_Sans_Bold_Oblique=${DEJAVU_DIR}/DejaVuSans-BoldOblique.ttf

filename.DejaVu_Sans_Mono=${DEJAVU_DIR}/DejaVuSansMono.ttf
filename.DejaVu_Sans_Mono_Bold=${DEJAVU_DIR}/DejaVuSansMono-Bold.ttf
filename.DejaVu_Sans_Mono_Oblique=${DEJAVU_DIR}/DejaVuSansMono-Oblique.ttf
filename.DejaVu_Sans_Mono_Bold_Oblique=${DEJAVU_DIR}/DejaVuSansMono-BoldOblique.ttf

filename.DejaVu_Serif=${DEJAVU_DIR}/DejaVuSerif.ttf
filename.DejaVu_Serif_Bold=${DEJAVU_DIR}/DejaVuSerif-Bold.ttf
filename.DejaVu_Serif_Italic=${DEJAVU_DIR}/DejaVuSerif-Italic.ttf
filename.DejaVu_Serif_Bold_Italic=${DEJAVU_DIR}/DejaVuSerif-BoldItalic.ttf
EOF

# Keep the explicit JDK font path aligned with the packaged DejaVu fonts.
export JAVA_TOOL_OPTIONS="${JAVA_TOOL_OPTIONS:-} -Djava.awt.headless=true -Dsun.java2d.fontpath=${DEJAVU_DIR} -Dsun.awt.fontconfig=${FONTCONFIG_PROPERTIES}"

# Under --nobuild_runfile_links (common on CI) or when PLANTUML_JAVA resolved
# to a shared exec-config copy with no runfiles tree of its own, Bazel's Java
# launcher aborts with "Cannot locate runfiles directory" before PlantUML even
# starts. The launcher resolves both its own JDK and PlantUML's classpath
# jars purely as paths under $JAVA_RUNFILES -- it never falls back to
# RUNFILES_MANIFEST_FILE for lookups the way our own rlocation() does -- so a
# symlink for just the JDK isn't enough (it still fails with
# ClassNotFoundException). Materialize a full synthetic runfiles tree from
# the manifest instead, one symlink per entry, matching what
# --nobuild_runfile_links leaves out. This runs once per diagram render
# within the same sandboxed action, so cache it (keyed by the manifest's
# checksum) under a stable path and reuse it across renders.
if [ -z "${JAVA_RUNFILES:-}" ]; then
  if [ -d "${PLANTUML_JAVA}.runfiles" ]; then
    JAVA_RUNFILES="${PLANTUML_JAVA}.runfiles"
  elif [ -n "${RUNFILES_DIR:-}" ] && [ -d "${RUNFILES_DIR}" ]; then
    JAVA_RUNFILES="${RUNFILES_DIR}"
  elif [ -f "${RUNFILES_MANIFEST_FILE:-}" ]; then
    manifest_key="$(cksum "${RUNFILES_MANIFEST_FILE}" | awk '{print $1 "-" $2}')"
    cache_dir="${TMPDIR:-/tmp}/plantuml-runfiles-cache-${manifest_key}"
    if [ ! -e "${cache_dir}/.materialized" ]; then
      rm -rf "${cache_dir}"
      mkdir -p "${cache_dir}"
      while IFS= read -r manifest_line; do
        rloc="${manifest_line%% *}"
        [ -z "${rloc}" ] && continue
        real="${manifest_line#* }"
        target="${cache_dir}/${rloc}"
        mkdir -p "$(dirname -- "${target}")"
        ln -sf "${real}" "${target}" 2>/dev/null || true
      done < "${RUNFILES_MANIFEST_FILE}"
      touch "${cache_dir}/.materialized"
    fi
    JAVA_RUNFILES="${cache_dir}"
  else
    JAVA_RUNFILES="$(mktemp -d "${TMPDIR:-/tmp}/plantuml-runfilesXXXXXX")"
  fi
  export JAVA_RUNFILES
fi

exec "${PLANTUML_JAVA}" "$@"
