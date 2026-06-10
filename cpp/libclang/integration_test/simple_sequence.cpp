/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/
// C++ implementation mirroring simple_sequence.puml
//
// Sequence diagram flow:
//   ComponentA -> ComponentB : Call Method1()
//   alt Condition1
//       ComponentB --> ComponentA : Return Result
//   else
//       ComponentB -> ComponentC : Call Method2()
//       ComponentC --> ComponentB : Return Result
//       alt Condition2
//           ComponentB -> ComponentC : Call Method2()
//           ComponentC --> ComponentB : Return Result
//       end
//       ComponentB --> ComponentA : Return Result
//   end

class ComponentC
{
  public:
    int method2(int input)
    {
        return input * 2;
    }
};

class ComponentB
{
  public:
    int method1(bool condition1, bool condition2, ComponentC& c)
    {
        if (condition1)
        {
            return 0;  // Return Result directly to ComponentA
        }
        else
        {
            int result = c.method2(1);  // Call Method2() on ComponentC
            for (int i = 0; i < 3; ++i)
            {
                result += c.method2(i);  // Accumulate results in a loop
            }
            int count = 2;
            while (count > 0)
            {
                result += c.method2(count);  // Accumulate via while loop
                --count;
            }
            if (condition2)
            {
                result = c.method2(result);  // Call Method2() again
            }
            return result;  // Return Result to ComponentA
        }
    }
};

class ComponentA
{
  public:
    int callMethod1(bool condition1, bool condition2)
    {
        ComponentC c;
        ComponentB b;
        return b.method1(condition1, condition2, c);  // Call Method1() on ComponentB
    }
};

int main()
{
    ComponentA a;
    return a.callMethod1(false, true);
}
