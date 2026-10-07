requirements "Feature Requirements" {
{FEAT_REQ_SOURCES}
}

{ROOT_CAUSE_BLOCK}

requirements "Component Requirements" {
{COMP_REQ_SOURCES}
}

implementation "Architecture" {
{ARCH_SOURCES}
  trace to: "Component Requirements";
}

activity "Unit Test" {
{UNIT_TEST_SOURCES}
}
