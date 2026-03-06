const n = 1000000;
const sieve = new Array(n + 1).fill(true);
sieve[0] = sieve[1] = false;
for (let i = 2; i * i <= n; i++) {
    if (sieve[i]) {
        for (let j = i * i; j <= n; j += i) {
            sieve[j] = false;
        }
    }
}
let count = 0;
for (let k = 2; k <= n; k++) {
    if (sieve[k]) count++;
}
console.log(count);
