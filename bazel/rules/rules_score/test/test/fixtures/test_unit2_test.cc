// Unit tests for test_unit2 fixture
#include <gtest/gtest.h>

// Declarations from mock libraries
extern int mock_function_1();
extern int mock_function_2();

TEST(Unit2Test, MockFunction1ReturnsExpectedValue) {
  ::testing::Test::RecordProperty("lobster-tracing", "SeoocTest.COMP_001");
  EXPECT_EQ(mock_function_1(), 42);
}

TEST(Unit2Test, MockFunction2ReturnsExpectedValue) {
  ::testing::Test::RecordProperty("lobster-tracing", "SeoocTest.COMP_001");
  EXPECT_EQ(mock_function_2(), 84);
}
