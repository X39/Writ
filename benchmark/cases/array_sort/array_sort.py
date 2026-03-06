import sys
sys.setrecursionlimit(200000)

def partition(arr, lo, hi):
    mid = lo + (hi - lo) // 2
    if arr[lo] > arr[mid]: arr[lo], arr[mid] = arr[mid], arr[lo]
    if arr[lo] > arr[hi]: arr[lo], arr[hi] = arr[hi], arr[lo]
    if arr[mid] > arr[hi]: arr[mid], arr[hi] = arr[hi], arr[mid]
    arr[mid], arr[hi] = arr[hi], arr[mid]
    pivot = arr[hi]
    i = lo - 1
    for j in range(lo, hi):
        if arr[j] <= pivot:
            i += 1
            arr[i], arr[j] = arr[j], arr[i]
    arr[i + 1], arr[hi] = arr[hi], arr[i + 1]
    return i + 1

def quicksort(arr, lo, hi):
    if lo < hi:
        p = partition(arr, lo, hi)
        quicksort(arr, lo, p - 1)
        quicksort(arr, p + 1, hi)

n = 100000
arr = list(range(n, 0, -1))
quicksort(arr, 0, n - 1)
print(f"{arr[0]} {arr[n - 1]}")
