#pragma once

void notify();

class Base {
public:
    template <typename T>
    void update(T&& value) {
        notify();
    }
};
