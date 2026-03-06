const map = new Map();
for (let i = 0; i < 100000; i++) {
    map.set(`key_${i}`, i);
}
let sum = 0;
for (let i = 0; i < 100000; i++) {
    sum += map.get(`key_${i}`);
}
console.log(sum);
