import os, re, subprocess, xml.etree.ElementTree as ET

def bazel(*args):
    return subprocess.check_output(["bazel", *args], text=True)

# When invoked via `bazel run`, Bazel sets this env var
ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
if ws:
    os.chdir(ws)

# discover external repos
ext_labels = bazel("query", "//external:*", "--output=label").splitlines()
ext_names  = {re.match(r"@([^/]+)//", l).group(1)
              for l in ext_labels if l.startswith("@")}
patterns   = ["//..."] + [f"@{n}//..." for n in ext_names]

expr = " + ".join(f'attr(tags,"cli_help=.*",{p})' for p in patterns)

# single XML query so that its not heavy on bazel runs
xml = bazel("query", expr, "--output=xml",
            "--ui_event_filters=-INFO,-progress", "--noshow_progress")

root = ET.fromstring(xml)
CYAN = "\033[36m"
print(f"{CYAN}BAZEL TARGETS:\n")

# print targets with cli_help tag
for rule in root.iter("rule"):
    lbl   = rule.get("name")
    tags  = [n.get("value") for n in rule.findall("list[@name='tags']/*")]
    for t in tags:
        if t.startswith("cli_help="):
            print(f"{lbl:40} {t.split('=',1)[1]}")
