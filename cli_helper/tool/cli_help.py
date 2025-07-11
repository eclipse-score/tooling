import os, re, subprocess, shutil, textwrap, xml.etree.ElementTree as ET

def bazel(*args: str) -> str:
    return subprocess.check_output(["bazel", *args], text=True)

# When invoked via `bazel run`, Bazel sets this env var
ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
if ws:
    os.chdir(ws)

# build a single query for all cli_help tags
ext_labels = bazel("query", "//external:*", "--output=label").splitlines()
ext_names  = {re.match(r"@([^/]+)//", lbl).group(1)
              for lbl in ext_labels if lbl.startswith("@")}
patterns   = ["//..."] + [f"@{n}//..." for n in ext_names]
expr       = " + ".join(f'attr(tags,"cli_help=.*",{p})' for p in patterns)

xml = bazel("query", expr, "--output=xml",
            "--ui_event_filters=-INFO,-progress", "--noshow_progress")
root = ET.fromstring(xml)

# format the output
rows, max_len = [], 0
for rule in root.iter("rule"):
    label = rule.get("name")
    # define here if you want to skip some targets
    if label.endswith(".find_main"):
        continue

    tags = [n.get("value") for n in rule.findall("list[@name='tags']/*")]
    try:
        desc = next(t for t in tags if t.startswith("cli_help=")).split("=", 1)[1]
    except StopIteration:
        continue

    rows.append((label, desc))
    max_len = max(max_len, len(label))

col_w       = max_len + 2
term_cols   = shutil.get_terminal_size(fallback=(80, 24)).columns
descr_width = max(20, term_cols - col_w - 1)

# pretty-print
CYAN= "\033[36m"
print(f"{CYAN}BAZEL TARGETS:\n")

for label, desc in rows:
    # keep user-inserted \n
    paragraphs = desc.splitlines()
    first_para, *rest_paras = paragraphs

    # wrap only the first paragraph
    wrapped_first = textwrap.wrap(
        first_para,
        width=descr_width,
        break_long_words=False,
        break_on_hyphens=False,
    )

    # print first paragraph
    print(f"{label.ljust(col_w)} {wrapped_first[0]}")
    for line in wrapped_first[1:]:
        print(" " * col_w + " " + line)

    # print remaining paragraphs exactly as written
    for para in rest_paras:
        print(" " * col_w + " " + para)
