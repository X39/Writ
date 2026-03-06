class Point {
    constructor(x, y, label) {
        this.x = x;
        this.y = y;
        this.label = label;
    }
}

let sum = 0;
for (let i = 0; i < 1000000; i++) {
    const p = new Point(i, i, "item");
    sum += p.x;
}
console.log(sum);
