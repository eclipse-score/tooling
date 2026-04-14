// Unit tests for test_binary_unit fixture
#include <gtest/gtest.h>

// Declarations from mock libraries
extern int mock_function_1();
extern int mock_function_2();

TEST(BinaryUnitTest, MockFunction1ReturnsExpectedValue) {
  ::testing::Test::RecordProperty("lobster-tracing", "SeoocTest.COMP_001");
  EXPECT_EQ(mock_function_1(), 42);
}

TEST(BinaryUnitTest, MockFunction2ReturnsExpectedValue) {
  ::testing::Test::RecordProperty("lobster-tracing", "SeoocTest.COMP_001");
  EXPECT_EQ(mock_function_2(), 84);
}
