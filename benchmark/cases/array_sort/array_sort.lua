local function partition(arr, lo, hi)
    local mid = lo + math.floor((hi - lo) / 2)
    if arr[lo] > arr[mid] then arr[lo], arr[mid] = arr[mid], arr[lo] end
    if arr[lo] > arr[hi] then arr[lo], arr[hi] = arr[hi], arr[lo] end
    if arr[mid] > arr[hi] then arr[mid], arr[hi] = arr[hi], arr[mid] end
    arr[mid], arr[hi] = arr[hi], arr[mid]
    local pivot = arr[hi]
    local i = lo - 1
    for j = lo, hi - 1 do
        if arr[j] <= pivot then
            i = i + 1
            arr[i], arr[j] = arr[j], arr[i]
        end
    end
    arr[i + 1], arr[hi] = arr[hi], arr[i + 1]
    return i + 1
end

local function quicksort(arr, lo, hi)
    if lo < hi then
        local p = partition(arr, lo, hi)
        quicksort(arr, lo, p - 1)
        quicksort(arr, p + 1, hi)
    end
end

local n = 100000
local arr = {}
for i = n, 1, -1 do arr[#arr + 1] = i end
quicksort(arr, 1, n)
print(arr[1] .. " " .. arr[n])
