implementation "Architecture" {
{ARCH_SOURCES}
}

requirements "Failure Modes" {
{FM_SOURCES}
  trace to: "Architecture";
}

requirements "Control Measures" {
{CM_SOURCES}
}

activity "Root Causes" {
{RC_SOURCES}
  trace to: "Failure Modes";
  trace to: "Control Measures";
}
