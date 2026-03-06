class Base:
    def compute(self):
        return 0

class TypeA(Base):
    def compute(self):
        return 1

class TypeB(Base):
    def compute(self):
        return 2

class TypeC(Base):
    def compute(self):
        return 3

class TypeD(Base):
    def compute(self):
        return 4

classes = [TypeA, TypeB, TypeC, TypeD]
s = 0
for i in range(100000):
    obj = classes[i % 4]()
    s += obj.compute()
print(s)
