#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Generate Ferrocene Rust coverage reports from Bazel rust_test targets.

Usage:
  bazel run @score_tooling//coverage:ferrocene_report -- [options]

Options:
  --query <bazel-query>    Bazel query for rust_test targets.
                           Default: kind("rust_test", //...)
  --targets <labels>       Comma-separated list of test labels (overrides --query).
  --out-dir <path>         Output directory for reports.
                           Default: <workspace>/coverage/rust-tests
  --profraw-dir <path>     Directory containing .profraw files for all targets.
                           If set, used for every target.
  --profraw-root <path>    Root for bazel-testlogs. Default: bazel info bazel-testlogs
  --bazel-config <name>    Bazel config to use for build/cquery.
                           Repeat to pass multiple configs.
                           Default: ferrocene-coverage
  --min-line-coverage <p>  Minimum line coverage percentage (0-100).
                           If any target is below, exit non-zero.
  --target-triple <triple> Rust target triple. Default: x86_64-unknown-linux-gnu
  --ferrocene-repo <name>  Ferrocene repo name. Default: ferrocene_x86_64_unknown_linux_gnu
  --help                   Show this help.

Notes:
- Run tests first with: bazel test --config=ferrocene-coverage <targets>
- The config should set LLVM_PROFILE_FILE to TEST_UNDECLARED_OUTPUTS_DIR.
- For cross/target execution, copy .profraw files back and pass --profraw-dir.
USAGE
}

QUERY='kind("rust_test", //...)'
TARGETS_CSV=""
OUT_DIR=""
PROFRAW_DIR=""
PROFRAW_ROOT=""
BAZEL_CONFIGS=()
TARGET_TRIPLE="x86_64-unknown-linux-gnu"
FERROCENE_REPO="ferrocene_x86_64_unknown_linux_gnu"
SYMBOL_REPORT_LABEL=""
BLANKET_LABEL=""
MIN_LINE_COVERAGE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --query)
      QUERY="$2"; shift 2 ;;
    --targets)
      TARGETS_CSV="$2"; shift 2 ;;
    --out-dir)
      OUT_DIR="$2"; shift 2 ;;
    --profraw-dir)
      PROFRAW_DIR="$2"; shift 2 ;;
    --profraw-root)
      PROFRAW_ROOT="$2"; shift 2 ;;
    --bazel-config)
      BAZEL_CONFIGS+=("$2"); shift 2 ;;
    --min-line-coverage)
      MIN_LINE_COVERAGE="$2"; shift 2 ;;
    --target-triple)
      TARGET_TRIPLE="$2"; shift 2 ;;
    --ferrocene-repo)
      FERROCENE_REPO="$2"; shift 2 ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "Unknown arg: $1" >&2
      usage
      exit 2
      ;;
  esac
 done

workspace="${BUILD_WORKSPACE_DIRECTORY:-$(pwd)}"
cd "${workspace}"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
parse_line_coverage_py="${script_dir}/scripts/parse_line_coverage.py"
normalize_symbol_report_py="${script_dir}/scripts/normalize_symbol_report.py"
if [[ ! -f "${parse_line_coverage_py}" || ! -f "${normalize_symbol_report_py}" ]]; then
  echo "Coverage helper scripts not found under ${script_dir}/scripts" >&2
  exit 1
fi

if [[ -z "${OUT_DIR}" ]]; then
  # Keep reports under bazel-bin by default so they track the build output tree.
  OUT_DIR="$(bazel info bazel-bin)/coverage/rust-tests"
fi

if [[ ${#BAZEL_CONFIGS[@]} -eq 0 ]]; then
  BAZEL_CONFIGS=("ferrocene-coverage")
fi

BAZEL_FLAGS=()
for cfg in "${BAZEL_CONFIGS[@]}"; do
  if [[ -n "${cfg}" ]]; then
    BAZEL_FLAGS+=("--config=${cfg}")
  fi
done

if [[ -z "${SYMBOL_REPORT_LABEL}" ]]; then
  SYMBOL_REPORT_LABEL="@score_toolchains_rust//toolchains/ferrocene:${FERROCENE_REPO}_symbol-report"
fi
if [[ -z "${BLANKET_LABEL}" ]]; then
  BLANKET_LABEL="@score_toolchains_rust//toolchains/ferrocene:${FERROCENE_REPO}_blanket"
fi

if [[ -z "${PROFRAW_ROOT}" ]]; then
  PROFRAW_ROOT="$(bazel info bazel-testlogs)"
fi

output_base="$(bazel info output_base)"
exec_root="$(bazel info execution_root)"

cquery_expr() {
  local label="$1"
  local expr="$2"
  bazel cquery "${BAZEL_FLAGS[@]}" --output=starlark --starlark:expr="${expr}" "${label}"
}

query_attr_build() {
  local label="$1"
  local attr="$2"
  local tmp
  tmp="$(mktemp)"
  bazel query --output=build "${label}" >"${tmp}"
  python3 - "$attr" "${tmp}" <<'PY'
import re
import sys

attr = sys.argv[1]
with open(sys.argv[2], "r", encoding="utf-8") as fh:
    data = fh.read()

# Match attr = <value> on a single line.
m = re.search(r'\\b%s\\s*=\\s*([^\\n]+)' % re.escape(attr), data)
if not m:
    sys.exit(0)
val = m.group(1).strip()

# Trim trailing commas.
if val.endswith(","):
    val = val[:-1].strip()

# If list spans multiple lines, capture the first [...] block.
if val.startswith("[") and "]" not in val:
    m = re.search(r'\\b%s\\s*=\\s*(\\[[^\\]]*\\])' % re.escape(attr), data, re.S)
    if m:
        val = m.group(1).strip()

print(val)
PY
  rm -f "${tmp}"
}

query_labels_attr() {
  local label="$1"
  local attr="$2"
  bazel query "labels(${attr}, ${label})" 2>/dev/null | head -n 1
}

aq_rustc_info() {
  local label="$1"
  local crate_root_rel="$2"
  local exec_root="$3"
  local tmp
  tmp="$(mktemp)"
  if ! bazel aquery "${BAZEL_FLAGS[@]}" --include_commandline --output=jsonproto "mnemonic(Rustc, ${label})" >"${tmp}" 2>/dev/null; then
    rm -f "${tmp}"
    return 0
  fi
  python3 - "${crate_root_rel}" "${exec_root}" "${tmp}" <<'PY'
import json
import sys
import shlex
from pathlib import Path

crate_root = sys.argv[1]
exec_root = sys.argv[2]
path = sys.argv[3]
try:
    data = json.load(open(path, "r", encoding="utf-8"))
except Exception:
    sys.exit(0)

actions = data.get("actions", [])
if not actions:
    sys.exit(0)

def match_action(act):
    args = act.get("arguments", [])
    if crate_root and crate_root in args:
        return True
    if crate_root:
        for arg in args:
            if arg.endswith(crate_root):
                return True
    return False

action = None
for act in actions:
    if match_action(act):
        action = act
        break
if action is None:
    action = actions[0]

args = list(action.get("arguments", []) or [])

def resolve_path(p: Path) -> Path:
    if p.is_absolute():
        return p
    if exec_root:
        return Path(exec_root) / p
    return p

def expand_params(argv):
    expanded = []
    for arg in argv:
        if arg.startswith("@") and len(arg) > 1:
            p = resolve_path(Path(arg[1:]))
            if p.is_file():
                content = p.read_text(encoding="utf-8", errors="ignore")
                try:
                    expanded.extend(shlex.split(content))
                except ValueError:
                    expanded.extend([line for line in content.splitlines() if line.strip()])
                continue
        expanded.append(arg)
    return expanded

args = expand_params(args)

if "--" in args:
    args = args[args.index("--") + 1:]

link_output = ""
for a in args:
    if a.startswith("--emit=link="):
        link_output = a.split("=", 2)[2]
        break

out = []
i = 0
while i < len(args):
    a = args[i]
    if a == "--extern" and i + 1 < len(args):
        out.extend(["--extern", args[i + 1]])
        i += 2
        continue
    if a.startswith("--extern="):
        out.append(a)
        i += 1
        continue
    if a == "-L" and i + 1 < len(args):
        out.extend(["-L", args[i + 1]])
        i += 2
        continue
    if a.startswith("-L"):
        out.append(a)
        i += 1
        continue
    if a == "--cfg" and i + 1 < len(args):
        out.extend(["--cfg", args[i + 1]])
        i += 2
        continue
    if a.startswith("--cfg="):
        out.append(a)
        i += 1
        continue
    if a == "--test":
        out.append(a)
        i += 1
        continue
    i += 1

lines = [link_output] + out
sys.stdout.write("\n".join(lines))
PY
  rm -f "${tmp}"
}

tool_cquery_expr() {
  local label="$1"
  local expr="$2"
  bazel cquery --output=starlark --starlark:expr="${expr}" "${label}"
}

# Build and locate the coverage tool wrappers (host tools; avoid target platform flags).
bazel build "${SYMBOL_REPORT_LABEL}" "${BLANKET_LABEL}" >/dev/null

symbol_report_rel="$(tool_cquery_expr "${SYMBOL_REPORT_LABEL}" 'target.files.to_list()[0].path')"
blanket_rel="$(tool_cquery_expr "${BLANKET_LABEL}" 'target.files.to_list()[0].path')"
if [[ -z "${symbol_report_rel}" || -z "${blanket_rel}" ]]; then
  echo "Failed to resolve coverage tool wrapper paths." >&2
  exit 1
fi

if [[ "${symbol_report_rel}" == /* ]]; then
  symbol_report_bin="${symbol_report_rel}"
else
  symbol_report_bin="${exec_root}/${symbol_report_rel}"
fi
if [[ "${blanket_rel}" == /* ]]; then
  blanket_bin="${blanket_rel}"
else
  blanket_bin="${exec_root}/${blanket_rel}"
fi

resolve_realpath() {
  local path="$1"
  if command -v realpath >/dev/null 2>&1; then
    realpath "${path}"
    return 0
  fi
  if command -v readlink >/dev/null 2>&1; then
    readlink -f "${path}"
    return 0
  fi
  echo "${path}"
}

strip_quotes() {
  local v="$1"
  v="${v%\"}"
  v="${v#\"}"
  echo "${v}"
}

label_to_path() {
  local label
  local pkg="${2:-}"
  label="$(strip_quotes "$1")"
  # External labels look like "@repo//pkg:target". Strip the repo prefix so
  # path conversion works for both workspace and external repos.
  if [[ "${label}" == @*//?* ]]; then
    label="//${label#*//}"
  fi
  # If the label still starts with "@", we do not know how to map it to a path.
  if [[ "${label}" == @* ]]; then
    echo ""
    return 0
  fi
  if [[ "${label}" == :* ]]; then
    if [[ -n "${pkg}" ]]; then
      echo "${pkg}/${label#:}"
    else
      echo "${label#:}"
    fi
    return 0
  fi
  if [[ "${label}" == //* ]]; then
    local rest="${label#//}"
    local pkg="${rest%%:*}"
    if [[ "${rest}" == *:* ]]; then
      local name="${rest#*:}"
      if [[ -n "${pkg}" ]]; then
        echo "${pkg}/${name}"
      else
        echo "${name}"
      fi
    else
      echo "${pkg}"
    fi
    return 0
  fi
  echo "${label}"
}

normalize_scalar() {
  local v
  v="$(strip_quotes "$1")"
  if [[ "${v}" =~ ^Label\\(\"(.*)\"\\)$ ]]; then
    v="${BASH_REMATCH[1]}"
  fi
  if [[ "${v}" == \[*\] ]]; then
    v="${v#[}"
    v="${v%]}"
    v="$(strip_quotes "${v%%,*}")"
  fi
  echo "${v}"
}

normalize_label() {
  local label
  local pkg="${2:-}"
  label="$(strip_quotes "$1")"
  if [[ "${label}" =~ ^Label\\(\"(.*)\"\\)$ ]]; then
    label="${BASH_REMATCH[1]}"
  fi
  if [[ "${label}" == :* ]]; then
    if [[ -n "${pkg}" ]]; then
      echo "//${pkg}:${label#:}"
    else
      echo "${label#:}"
    fi
    return 0
  fi
  echo "${label}"
}

label_pkg() {
  local label
  label="$(strip_quotes "$1")"
  if [[ "${label}" =~ ^Label\\(\"(.*)\"\\)$ ]]; then
    label="${BASH_REMATCH[1]}"
  fi
  # External labels include "@repo//". Strip the repo prefix to get the package.
  if [[ "${label}" == @*//?* ]]; then
    label="//${label#*//}"
  fi
  if [[ "${label}" == //* ]]; then
    local rest="${label#//}"
    echo "${rest%%:*}"
    return 0
  fi
  echo ""
}

# Resolve the "external/<repo>" prefix for an external label.
# We use bazel query --output=location to find a real file path, then extract
# the repo name from either "external/<repo>/..." or ".../external/<repo>/...".
workspace_root_for_label() {
  local label
  label="$(strip_quotes "$1")"
  if [[ "${label}" =~ ^Label\\(\"(.*)\"\\)$ ]]; then
    label="${BASH_REMATCH[1]}"
  fi
  # Non-external labels live in the workspace, so no external prefix is needed.
  if [[ "${label}" != @* ]]; then
    echo ""
    return 0
  fi
  # The location output may be absolute; handle both direct external paths
  # and absolute paths that contain "/external/<repo>/".
  local location
  location="$(bazel query --output=location "${label}" 2>/dev/null | head -n 1)"
  location="${location%%:*}"
  local rest=""
  if [[ "${location}" == external/* ]]; then
    rest="${location#external/}"
  elif [[ "${location}" == */external/* ]]; then
    rest="${location#*/external/}"
  fi
  local repo="${rest%%/*}"
  if [[ -n "${repo}" ]]; then
    echo "external/${repo}"
  fi
}

resolve_runfile() {
  local bin="$1"
  local name="$2"
  local runfiles_dir=""

  if [[ -d "${bin}.runfiles" ]]; then
    runfiles_dir="${bin}.runfiles"
  elif [[ -f "${bin}.runfiles_manifest" ]]; then
    local entry
    entry="$(grep -m1 "/${name}$" "${bin}.runfiles_manifest" || true)"
    if [[ -n "${entry}" ]]; then
      echo "${entry#* }"
      return 0
    fi
  fi

  if [[ -z "${runfiles_dir}" ]]; then
    return 1
  fi

  find "${runfiles_dir}" -type f -name "${name}" -print -quit 2>/dev/null
}

# Derive the Ferrocene sysroot from the wrapper location.
if [[ -z "${SYSROOT:-}" ]]; then
  symbol_report_real="$(resolve_realpath "${symbol_report_bin}" || true)"
  if [[ -n "${symbol_report_real}" && "${symbol_report_real}" == */symbol-report.sh ]]; then
    SYSROOT="$(cd "$(dirname "${symbol_report_real}")" && pwd)"
  else
    symbol_report_runfile="$(resolve_runfile "${symbol_report_bin}" "symbol-report.sh" || true)"
    if [[ -z "${symbol_report_runfile}" ]]; then
      echo "Failed to locate symbol-report.sh in runfiles for ${symbol_report_bin}" >&2
      exit 1
    fi
    SYSROOT="$(cd "$(dirname "${symbol_report_runfile}")" && pwd)"
  fi
fi

prefer_wrapper_script() {
  local bin="$1"
  local name="$2"
  if [[ "${bin}" == */${name} ]]; then
    local candidate
    candidate="$(dirname "${bin}")/${name}.sh"
    if [[ -f "${candidate}" ]]; then
      echo "${candidate}"
      return 0
    fi
  fi
  echo "${bin}"
}

symbol_report_bin="$(prefer_wrapper_script "${symbol_report_bin}" "symbol-report")"
blanket_bin="$(prefer_wrapper_script "${blanket_bin}" "blanket")"

raw_binary_path() {
  local bin="$1"
  if [[ "${bin}" == *.sh ]]; then
    local raw="${bin%.sh}"
    if [[ -x "${raw}" ]]; then
      echo "${raw}"
      return 0
    fi
  fi
  echo "${bin}"
}

extra_ld_for_missing_driver() {
  local bin="$1"
  local sysroot="$2"
  if [[ ! -x "${bin}" ]]; then
    echo ""
    return 0
  fi
  local missing
  missing="$(ldd "${bin}" 2>/dev/null | awk '/librustc_driver-.*not found/ {print $1; exit}')"
  if [[ -z "${missing}" ]]; then
    echo ""
    return 0
  fi
  local existing
  existing="$(ls "${sysroot}"/lib/librustc_driver-*.so 2>/dev/null | head -n1 || true)"
  if [[ -z "${existing}" ]]; then
    echo ""
    return 0
  fi
  local tmpdir
  tmpdir="$(mktemp -d)"
  ln -s "${existing}" "${tmpdir}/${missing}"
  echo "${tmpdir}"
}

symbol_report_cmd=("${symbol_report_bin}")
if [[ ! -x "${symbol_report_bin}" ]]; then
  if [[ "${symbol_report_bin}" == *.sh && -f "${symbol_report_bin}" ]]; then
    symbol_report_cmd=(bash "${symbol_report_bin}")
  else
    echo "symbol-report wrapper not executable at ${symbol_report_bin}" >&2
    exit 1
  fi
fi

blanket_cmd=("${blanket_bin}")
if [[ ! -x "${blanket_bin}" ]]; then
  if [[ "${blanket_bin}" == *.sh && -f "${blanket_bin}" ]]; then
    blanket_cmd=(bash "${blanket_bin}")
  else
    echo "blanket wrapper not executable at ${blanket_bin}" >&2
    exit 1
  fi
fi

symbol_report_raw="$(raw_binary_path "${symbol_report_bin}")"
blanket_raw="$(raw_binary_path "${blanket_bin}")"
symbol_report_extra_ld="$(extra_ld_for_missing_driver "${symbol_report_raw}" "${SYSROOT}")"
blanket_extra_ld="$(extra_ld_for_missing_driver "${blanket_raw}" "${SYSROOT}")"

mapfile -t targets < <(
  if [[ -n "${TARGETS_CSV}" ]]; then
    echo "${TARGETS_CSV}" | tr ',' '\n'
  else
    bazel query "${QUERY}"
  fi
)

if [[ ${#targets[@]} -eq 0 ]]; then
  echo "No targets found for query: ${QUERY}" >&2
  exit 1
fi

mkdir -p "${OUT_DIR}"

parse_line_coverage() {
  local html_path="$1"
  # Blanket reports line coverage in the HTML summary; parse it for gating.
  python3 "${parse_line_coverage_py}" "${html_path}"
}

failures=()
total_covered=0
total_lines=0
parsed_targets=0

for label in "${targets[@]}"; do
  if [[ -z "${label}" ]]; then
    continue
  fi

  pkg="${label#//}"
  pkg="${pkg%%:*}"
  name="${label##*:}"

  # Resolve the package path and repo root so test.outputs works for
  # workspace labels (//pkg:target) and external labels (@repo//pkg:target).
  label_pkg_path="$(label_pkg "${label}")"
  if [[ -z "${label_pkg_path}" ]]; then
    label_pkg_path="${pkg}"
  fi
  label_repo_root="$(workspace_root_for_label "${label}")"

  if [[ -n "${PROFRAW_DIR}" ]]; then
    test_out_dir="${PROFRAW_DIR}"
  elif [[ -n "${label_repo_root}" ]]; then
    test_out_dir="${PROFRAW_ROOT}/${label_repo_root}/${label_pkg_path}/${name}/test.outputs"
  else
    test_out_dir="${PROFRAW_ROOT}/${label_pkg_path}/${name}/test.outputs"
  fi

  shopt -s nullglob
  profraw_files=("${test_out_dir}"/*.profraw)
  shopt -u nullglob

  if [[ ${#profraw_files[@]} -eq 0 ]]; then
    echo "Skipping ${label}: no .profraw files in ${test_out_dir}" >&2
    continue
  fi

  # Build the test target with the chosen config to ensure the instrumented binary exists.
  bazel build "${BAZEL_FLAGS[@]}" "${label}" >/dev/null

  bin_rel="$(cquery_expr "${label}" 'target.files.to_list()[0].path')"
  if [[ -z "${bin_rel}" ]]; then
    echo "Skipping ${label}: could not resolve test binary path" >&2
    continue
  fi

  if [[ "${bin_rel}" == /* ]]; then
    bin_path="${bin_rel}"
  else
    bin_path="${exec_root}/${bin_rel}"
  fi

  if [[ ! -x "${bin_path}" ]]; then
    echo "Skipping ${label}: test binary not found at ${bin_path}" >&2
    continue
  fi

  # rust_test can reference a crate by label (preferred) or by name.
  # We try label first, then fall back to the raw attribute.
  crate_label_raw="$(query_labels_attr "${label}" "crate")"
  if [[ -z "${crate_label_raw}" ]]; then
    crate_label_raw="$(query_attr_build "${label}" "crate")"
  fi
  crate_label="$(normalize_scalar "${crate_label_raw}")"
  crate_type=""
  crate_target=""
  if [[ -n "${crate_label}" ]]; then
    crate_target="$(normalize_label "${crate_label}" "${pkg}")"
    crate_type="lib"
  else
    crate_target="${label}"
    crate_type="$(normalize_scalar "$(query_attr_build "${label}" "crate_type")")"
  fi
  if [[ -z "${crate_type}" ]]; then
    crate_type="bin"
  fi

  crate_pkg="$(label_pkg "${crate_target}")"
  if [[ -z "${crate_pkg}" ]]; then
    crate_pkg="${pkg}"
  fi
  repo_root="$(workspace_root_for_label "${crate_target}")"

  crate_root_raw="$(query_labels_attr "${crate_target}" "crate_root")"
  if [[ -z "${crate_root_raw}" ]]; then
    crate_root_raw="$(query_attr_build "${crate_target}" "crate_root")"
  fi
  crate_root="$(label_to_path "${crate_root_raw}" "${crate_pkg}")"
  # First, try conventional crate roots to avoid choosing a random source file.
  if [[ -z "${crate_root}" ]]; then
    for candidate in \
      "${crate_pkg}/src/lib.rs" \
      "${crate_pkg}/src/main.rs" \
      "${crate_pkg}/lib.rs" \
      "${crate_pkg}/main.rs"; do
      if [[ -n "${repo_root}" ]]; then
        if [[ -f "${exec_root}/${repo_root}/${candidate}" ]]; then
          crate_root="${candidate}"
          break
        fi
      elif [[ -f "${workspace}/${candidate}" ]]; then
        crate_root="${candidate}"
        break
      fi
    done
  fi
  # If there is no conventional root, fall back to the crate's declared srcs.
  if [[ -z "${crate_root}" ]]; then
    srcs_label="$(query_labels_attr "${crate_target}" "srcs")"
    if [[ -n "${srcs_label}" ]]; then
      srcs_path="$(label_to_path "${srcs_label}" "${crate_pkg}")"
      if [[ -n "${srcs_path}" && "${srcs_path}" == *.rs ]]; then
        crate_root="${srcs_path}"
      fi
    fi
  fi
  if [[ -z "${crate_root}" ]]; then
    # As a last resort, try rust_test srcs when the test target defines them.
    # This handles rust_test targets that directly list their sources.
    srcs_label="$(query_labels_attr "${label}" "srcs")"
    if [[ -n "${srcs_label}" ]]; then
      srcs_path="$(label_to_path "${srcs_label}" "${pkg}")"
      if [[ -n "${srcs_path}" && "${srcs_path}" == *.rs ]]; then
        crate_root="${srcs_path}"
      fi
    fi
  fi
  # Without a crate root, symbol-report cannot build the crate.
  if [[ -z "${crate_root}" ]]; then
    echo "Skipping ${label}: could not determine crate root for ${crate_target}" >&2
    continue
  fi
  # Convert the crate root into an absolute path. External repos live under
  # exec_root/external/<repo> (symlinked from output_base), while workspace
  # sources live under $workspace.
  if [[ "${crate_root}" != /* ]]; then
    if [[ -n "${repo_root}" && "${crate_root}" != "${repo_root}/"* ]]; then
      crate_root="${repo_root}/${crate_root}"
    fi
    if [[ "${crate_root}" == external/* ]]; then
      crate_root="${exec_root}/${crate_root}"
    else
      crate_root="${workspace}/${crate_root}"
    fi
  fi

  # Keep a workspace- or exec_root-relative path for reporting and mapping.
  crate_root_rel="${crate_root}"
  if [[ "${crate_root_rel}" == "${workspace}/"* ]]; then
    crate_root_rel="${crate_root_rel#${workspace}/}"
  elif [[ "${crate_root_rel}" == "${exec_root}/"* ]]; then
    crate_root_rel="${crate_root_rel#${exec_root}/}"
  fi

  crate_name="$(normalize_scalar "$(query_attr_build "${crate_target}" "crate_name")")"
  if [[ -z "${crate_name}" ]]; then
    crate_name="${crate_target##*:}"
    crate_name="${crate_name#//}"
  fi
  edition="$(normalize_scalar "$(query_attr_build "${crate_target}" "edition")")"
  if [[ -z "${edition}" ]]; then
    edition="2021"
  fi

  mapfile -t aquery_lines < <(aq_rustc_info "${crate_target}" "${crate_root_rel}" "${exec_root}")
  link_out="${aquery_lines[0]:-}"
  extra_rustc_flags=("${aquery_lines[@]:1}")
  test_link_out=""
  if [[ "${crate_target}" != "${label}" ]]; then
    mapfile -t test_aquery_lines < <(aq_rustc_info "${label}" "${crate_root_rel}" "${exec_root}")
    test_link_out="${test_aquery_lines[0]:-}"
  fi
  extra_rustc_args=()
  for flag in "${extra_rustc_flags[@]}"; do
    if [[ "${flag}" == --extern=* ]]; then
      extra_rustc_args+=("--extern" "${flag#--extern=}")
    elif [[ "${flag}" == --cfg=* ]]; then
      extra_rustc_args+=("--cfg" "${flag#--cfg=}")
    elif [[ "${flag}" == --test ]]; then
      extra_rustc_args+=("--test")
    else
      extra_rustc_args+=("${flag}")
    fi
  done
  if [[ "${FERROCENE_REPORT_DEBUG:-0}" == "1" ]]; then
    echo "Debug: ${label} crate_label_raw=${crate_label_raw}" >&2
    echo "Debug: ${label} crate_label=${crate_label}" >&2
    echo "Debug: ${label} crate_target=${crate_target} crate_type=${crate_type} crate_name=${crate_name} edition=${edition}" >&2
    echo "Debug: ${label} link_out=${link_out}" >&2
    if [[ -n "${test_link_out}" ]]; then
      echo "Debug: ${label} test_link_out=${test_link_out}" >&2
    fi
    echo "Debug: ${label} crate_root=${crate_root_rel} flags from aquery:" >&2
    printf '  %s\n' "${extra_rustc_flags[@]}" >&2
    echo "Debug: ${label} normalized rustc args:" >&2
    printf '  %q\n' "${extra_rustc_args[@]}" >&2
  fi

  if [[ -n "${link_out}" ]]; then
    if [[ "${link_out}" == /* ]]; then
      candidate_path="${link_out}"
    else
      candidate_path="${exec_root}/${link_out}"
    fi
    if [[ -x "${candidate_path}" ]]; then
      bin_rel="${link_out}"
      bin_path="${candidate_path}"
    fi
  fi
  if [[ -n "${test_link_out}" ]]; then
    if [[ "${test_link_out}" == /* ]]; then
      candidate_path="${test_link_out}"
    else
      candidate_path="${exec_root}/${test_link_out}"
    fi
    if [[ -x "${candidate_path}" ]]; then
      bin_rel="${test_link_out}"
      bin_path="${candidate_path}"
    fi
  fi

  safe_label="${label//\//_}"
  safe_label="${safe_label//:/_}"
  safe_label="${safe_label//@/_}"

  report_dir="${OUT_DIR}/${safe_label}"
  mkdir -p "${report_dir}"

  symbol_report_json="${report_dir}/symbol-report.json"

  sysroot_arg="${SYSROOT}"
  if [[ "${SYSROOT}" == "${exec_root}/"* ]]; then
    sysroot_arg="${SYSROOT#${exec_root}/}"
  fi

  # Remap execroot/workspace paths to keep symbol-report filenames stable.
  remap_args=()
  if [[ -n "${exec_root}" ]]; then
    remap_args+=("--remap-path-prefix=${exec_root}/=.")
  fi
  if [[ "${workspace}" != "${exec_root}" ]]; then
    remap_args+=("--remap-path-prefix=${workspace}/=.")
  fi

  # Pass the absolute crate root; relative external paths fail to canonicalize.
  (
    cd "${exec_root}"
    SYMBOL_REPORT_OUT="${symbol_report_json}" \
    LD_LIBRARY_PATH="${symbol_report_extra_ld}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
      "${symbol_report_cmd[@]}" \
      --crate-name "${crate_name}" \
      --edition "${edition}" \
      --crate-type "${crate_type}" \
      "${extra_rustc_args[@]}" \
      --target "${TARGET_TRIPLE}" \
      --sysroot "${sysroot_arg}" \
      -o /dev/null \
      "${remap_args[@]}" \
      "${crate_root}"
  )

  # Normalize symbol-report paths to be workspace-relative (like the demo),
  # so blanket can reliably locate sources.
  python3 "${normalize_symbol_report_py}" "${symbol_report_json}" "${workspace}" "${exec_root}"

  bin_arg="${bin_path}"
  if [[ "${bin_rel}" != /* ]]; then
    bin_arg="${bin_rel}"
  fi

  # Blanket resolves report filenames by joining them with --ferrocene-src.
  # Use a path-equivalence so source files map cleanly to report entries.
  # For external crates, profiler paths are absolute under output_base/external,
  # so we point --ferrocene-src there instead of the workspace.
  ferrocene_src="${workspace}"
  if [[ "${crate_root_rel}" == external/* ]]; then
    ferrocene_src="${output_base}"
  fi
  crate_root_dir_rel="$(dirname "${crate_root_rel}")"
  path_prefix="${crate_root_rel%%/*}"
  if [[ -n "${path_prefix}" && "${path_prefix}" != "${crate_root_rel}" && "${path_prefix}" != "." ]]; then
    # Broader remap to cover any file under the top-level directory (e.g. src/...).
    path_equiv_args=("--path-equivalence" "${path_prefix},${ferrocene_src}/${path_prefix}")
  elif [[ "${crate_root_dir_rel}" == "." ]]; then
    path_equiv_args=("--path-equivalence" ".,${ferrocene_src}")
  else
    path_equiv_args=("--path-equivalence" "${crate_root_dir_rel},${ferrocene_src}/${crate_root_dir_rel}")
  fi

  (
    cd "${workspace}"
    LD_LIBRARY_PATH="${blanket_extra_ld}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
      "${blanket_cmd[@]}" show \
      $(printf -- '--instr-profile=%s ' "${profraw_files[@]}") \
      --object "${bin_arg}" \
      --report "${symbol_report_json}" \
      --ferrocene-src "${ferrocene_src}" \
      "${path_equiv_args[@]}" \
      --html-out "${report_dir}/blanket/index.html"
  )

  line_cov=""
  line_cov_info=""
  if line_cov="$(parse_line_coverage "${report_dir}/blanket/index.html" 2>/dev/null)"; then
    read -r line_pct line_cov_lines line_total_lines <<<"${line_cov}"
    line_cov_info="${line_pct}% (${line_cov_lines}/${line_total_lines} lines)"
    echo "Line coverage for ${label}: ${line_cov_info}"
    if [[ -n "${line_cov_lines}" && -n "${line_total_lines}" ]]; then
      total_covered=$((total_covered + line_cov_lines))
      total_lines=$((total_lines + line_total_lines))
      parsed_targets=$((parsed_targets + 1))
    fi
    if [[ -n "${MIN_LINE_COVERAGE}" ]]; then
      if ! python3 - "${line_pct}" "${MIN_LINE_COVERAGE}" <<'PY'
import sys
try:
    val = float(sys.argv[1])
    minv = float(sys.argv[2])
except ValueError:
    sys.exit(0)
sys.exit(0 if val >= minv else 1)
PY
      then
        failures+=("${label} (${line_cov_info})")
      fi
    fi
  else
    echo "Warning: could not parse line coverage from ${report_dir}/blanket/index.html" >&2
    if [[ -n "${MIN_LINE_COVERAGE}" ]]; then
      failures+=("${label} (no line coverage parsed)")
    fi
  fi

  echo "Report for ${label}: ${report_dir}/blanket/index.html"
 done

if [[ ${total_lines} -gt 0 ]]; then
  # Aggregate line coverage across all targets that parsed successfully.
  overall_pct="$(python3 - "${total_covered}" "${total_lines}" <<'PY'
import sys
cov = int(sys.argv[1])
total = int(sys.argv[2])
if total == 0:
    print("0.00")
else:
    print(f"{(cov / total) * 100:.2f}")
PY
)"
  echo "---"
  echo "Overall line coverage: ${overall_pct}% (${total_covered}/${total_lines} lines across ${parsed_targets} targets)"
  echo "---"
else
  echo "---"
  echo "Overall line coverage: n/a (no per-target line coverage parsed)"
  echo "---"
fi

if [[ ${#failures[@]} -gt 0 ]]; then
  # Fail CI when any target is below the minimum line-coverage threshold.
  echo "Line coverage gate failed (min ${MIN_LINE_COVERAGE}%):" >&2
  printf '  %s\n' "${failures[@]}" >&2
  exit 3
fi
