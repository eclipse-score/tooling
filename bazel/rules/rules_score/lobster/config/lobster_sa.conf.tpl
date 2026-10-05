implementation "Architecture" {
{ARCH_SOURCES}
}

requirements "Failure Modes" {
{FM_SOURCES}
  trace to: "Architecture";
}

activity "Root Causes" {
{RC_SOURCES}
  trace to: "Failure Modes";
}
