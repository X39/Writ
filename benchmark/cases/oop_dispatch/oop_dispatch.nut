class Base {
    function compute() { return 0; }
}

class TypeA extends Base {
    function compute() { return 1; }
}

class TypeB extends Base {
    function compute() { return 2; }
}

class TypeC extends Base {
    function compute() { return 3; }
}

class TypeD extends Base {
    function compute() { return 4; }
}

local classes = [TypeA, TypeB, TypeC, TypeD];
local sum = 0;
for (local i = 0; i < 100000; i += 1) {
    local obj = classes[i % 4]();
    sum += obj.compute();
}
print(sum + "\n");
