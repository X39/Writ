function partition(arr, lo, hi) {
    const mid = lo + ((hi - lo) >> 1);
    if (arr[lo] > arr[mid]) { [arr[lo], arr[mid]] = [arr[mid], arr[lo]]; }
    if (arr[lo] > arr[hi]) { [arr[lo], arr[hi]] = [arr[hi], arr[lo]]; }
    if (arr[mid] > arr[hi]) { [arr[mid], arr[hi]] = [arr[hi], arr[mid]]; }
    [arr[mid], arr[hi]] = [arr[hi], arr[mid]];
    const pivot = arr[hi];
    let i = lo - 1;
    for (let j = lo; j < hi; j++) {
        if (arr[j] <= pivot) {
            i++;
            [arr[i], arr[j]] = [arr[j], arr[i]];
        }
    }
    [arr[i + 1], arr[hi]] = [arr[hi], arr[i + 1]];
    return i + 1;
}

function quicksort(arr, lo, hi) {
    if (lo < hi) {
        const p = partition(arr, lo, hi);
        quicksort(arr, lo, p - 1);
        quicksort(arr, p + 1, hi);
    }
}

const n = 100000;
const arr = new Array(n);
for (let i = 0; i < n; i++) arr[i] = n - i;
quicksort(arr, 0, n - 1);
console.log(arr[0] + " " + arr[n - 1]);
