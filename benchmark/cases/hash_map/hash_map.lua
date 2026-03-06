local map = {}
for i = 0, 99999 do
    map["key_" .. i] = i
end
local sum = 0
for i = 0, 99999 do
    sum = sum + map["key_" .. i]
end
print(sum)
