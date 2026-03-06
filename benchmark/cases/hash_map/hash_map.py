m = {}
for i in range(100000):
    m[f"key_{i}"] = i
s = 0
for i in range(100000):
    s += m[f"key_{i}"]
print(s)
