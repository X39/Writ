local map = {};
for (local i = 0; i < 100000; i += 1) {
    map["key_" + i] <- i;
}
local sum = 0;
for (local i = 0; i < 100000; i += 1) {
    sum += map["key_" + i];
}
print(sum + "\n");
