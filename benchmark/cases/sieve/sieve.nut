local n = 1000000;
local sieve = array(n + 1, true);
sieve[0] = false;
sieve[1] = false;
local i = 2;
while (i * i <= n) {
    if (sieve[i]) {
        local j = i * i;
        while (j <= n) {
            sieve[j] = false;
            j += i;
        }
    }
    i += 1;
}
local count = 0;
for (local k = 2; k <= n; k += 1) {
    if (sieve[k]) count += 1;
}
print(count + "\n");
