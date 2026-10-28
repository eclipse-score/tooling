requirements "Public API" {
{PUBLIC_API_SOURCES}
}

requirements "Failure Modes" {
{FM_SOURCES}
  trace to: "Public API";
}

activity "Root Causes" {
{RC_SOURCES}
  trace to: "Failure Modes";
}
