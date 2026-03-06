fn partition(arr: &mut [i64], lo: usize, hi: usize) -> usize {
    let mid = lo + (hi - lo) / 2;
    if arr[lo] > arr[mid] { arr.swap(lo, mid); }
    if arr[lo] > arr[hi] { arr.swap(lo, hi); }
    if arr[mid] > arr[hi] { arr.swap(mid, hi); }
    arr.swap(mid, hi);
    let pivot = arr[hi];
    let mut i = lo;
    for j in lo..hi {
        if arr[j] <= pivot {
            arr.swap(i, j);
            i += 1;
        }
    }
    arr.swap(i, hi);
    i
}

fn quicksort(arr: &mut [i64], lo: usize, hi: usize) {
    if lo < hi {
        let p = partition(arr, lo, hi);
        if p > 0 { quicksort(arr, lo, p - 1); }
        quicksort(arr, p + 1, hi);
    }
}

fn main() {
    let n: usize = 100_000;
    let mut arr: Vec<i64> = (0..n).map(|i| (n - i) as i64).collect();
    quicksort(&mut arr, 0, n - 1);
    println!("{} {}", arr[0], arr[n - 1]);
}
