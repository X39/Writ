class Base {
    compute() { return 0; }
}

class TypeA extends Base {
    compute() { return 1; }
}

class TypeB extends Base {
    compute() { return 2; }
}

class TypeC extends Base {
    compute() { return 3; }
}

class TypeD extends Base {
    compute() { return 4; }
}

const classes = [TypeA, TypeB, TypeC, TypeD];
let sum = 0;
for (let i = 0; i < 100000; i++) {
    const obj = new classes[i % 4]();
    sum += obj.compute();
}
console.log(sum);
