implementation "Architecture" {
{ARCH_SOURCES}
}

requirements "Failure Modes" {
{FM_SOURCES}
  trace to: "Architecture";
}

activity "FTA Failure Modes" {
{FTA_FM_SOURCES}
  trace to: "Failure Modes";
}

activity "Root Causes" {
{RC_SOURCES}
}

requirements "Safety Measures" {
{SAFETYMEASURES_SOURCES}
}
