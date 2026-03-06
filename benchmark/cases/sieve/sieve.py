n = 1000000
sieve = [True] * (n + 1)
sieve[0] = sieve[1] = False
i = 2
while i * i <= n:
    if sieve[i]:
        j = i * i
        while j <= n:
            sieve[j] = False
            j += i
    i += 1
count = sum(1 for x in sieve[2:] if x)
print(count)
