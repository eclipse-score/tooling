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

#include <map>
#include <vector>

class MethodParameterTypeSample {
  public:
    void setId(int id);
    void setName(const char* name);
    void setIdRef(const int& id);
    void setIdRValueRef(const int&& id);

    void setMutableBuffer(int* buffer);
    void setPinnedBuffer(int* const buffer);
    void setConstBuffer(const int* const buffer);

    void setHistory(std::vector<int> history);
    void setNameList(const std::vector<const char*>& names);
    void setHistoryMatrix(std::vector<std::vector<int>> matrix);
    void setNameMatrix(std::vector<std::vector<const char*>> matrix);

    void setNameIndex(std::map<int, std::vector<const char*>> index);
    void setHistoryLayerViews(std::vector<const std::vector<int>*> layers);
    void setConstPointerMatrix(std::vector<std::vector<const int*>> matrix);
};
