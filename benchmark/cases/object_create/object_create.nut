class Point {
    x = 0;
    y = 0;
    label = "";
    constructor(_x, _y, _label) {
        x = _x;
        y = _y;
        label = _label;
    }
}

local sum = 0;
for (local i = 0; i < 1000000; i += 1) {
    local p = Point(i, i, "item");
    sum += p.x;
}
print(sum + "\n");
