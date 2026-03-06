function partition(arr, lo, hi) {
    local mid = lo + ((hi - lo) / 2).tointeger();
    if (arr[lo] > arr[mid]) { local t = arr[lo]; arr[lo] = arr[mid]; arr[mid] = t; }
    if (arr[lo] > arr[hi]) { local t = arr[lo]; arr[lo] = arr[hi]; arr[hi] = t; }
    if (arr[mid] > arr[hi]) { local t = arr[mid]; arr[mid] = arr[hi]; arr[hi] = t; }
    local t = arr[mid]; arr[mid] = arr[hi]; arr[hi] = t;
    local pivot = arr[hi];
    local i = lo - 1;
    for (local j = lo; j < hi; j += 1) {
        if (arr[j] <= pivot) {
            i += 1;
            local tmp = arr[i]; arr[i] = arr[j]; arr[j] = tmp;
        }
    }
    local tmp = arr[i + 1]; arr[i + 1] = arr[hi]; arr[hi] = tmp;
    return i + 1;
}

function quicksort(arr, lo, hi) {
    if (lo < hi) {
        local p = partition(arr, lo, hi);
        quicksort(arr, lo, p - 1);
        quicksort(arr, p + 1, hi);
    }
}

local n = 100000;
local arr = array(n);
for (local i = 0; i < n; i += 1) { arr[i] = n - i; }
quicksort(arr, 0, n - 1);
print(arr[0] + " " + arr[n - 1] + "\n");
