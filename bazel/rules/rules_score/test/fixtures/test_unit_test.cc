// Unit tests for mock libraries
#include <iostream>

// Declarations from mock libraries
extern int mock_function_1();
extern int mock_function_2();

int main() {
    // Test mock_function_1
    int result1 = mock_function_1();
    if (result1 != 42) {
        std::cerr << "Test failed: mock_function_1() returned " << result1 << ", expected 42" << std::endl;
        return 1;
    }

    // Test mock_function_2
    int result2 = mock_function_2();
    if (result2 != 84) {
        std::cerr << "Test failed: mock_function_2() returned " << result2 << ", expected 84" << std::endl;
        return 1;
    }

    std::cout << "All tests passed!" << std::endl;
    return 0;
}
