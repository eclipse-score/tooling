requirements "Feature Requirements" {
{FEAT_REQ_SOURCES}
}

requirements "Component Requirements" {
{COMP_REQ_SOURCES}
  trace to: "Feature Requirements";
}

activity "Unit Test" {
{UNIT_TEST_SOURCES}
  trace to: "Component Requirements";
}

implementation "Architecture" {
{ARCH_SOURCES}
  trace to: "Component Requirements";
}

implementation "Public API" {
{PUBLIC_API_SOURCES}
  trace to: "Component Requirements";
}

requirements "Failure Modes" {
{FM_SOURCES}
  trace to: "Public API";
}

requirements "Control Measures" {
{CM_SOURCES}
}

activity "Root Causes" {
{RC_SOURCES}
  trace to: "Failure Modes";
  trace to: "Control Measures";
}
