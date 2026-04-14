// Unit tests for mock libraries
#include <gtest/gtest.h>

// Declarations from mock libraries
extern int mock_function_1();
extern int mock_function_2();

TEST(MockLibTest, MockFunction1ReturnsExpectedValue) {
  ::testing::Test::RecordProperty("lobster-tracing", "SeoocTest.COMP_001");
  EXPECT_EQ(mock_function_1(), 42);
}

TEST(MockLibTest, MockFunction2ReturnsExpectedValue) {
  ::testing::Test::RecordProperty("lobster-tracing", "SeoocTest.COMP_001");
  EXPECT_EQ(mock_function_2(), 84);
}
