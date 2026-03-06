class Point:
    __slots__ = ('x', 'y', 'label')
    def __init__(self, x, y, label):
        self.x = x
        self.y = y
        self.label = label

s = 0
for i in range(1000000):
    p = Point(i, i, "item")
    s += p.x
print(s)
