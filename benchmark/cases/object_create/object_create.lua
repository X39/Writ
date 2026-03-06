local sum = 0
for i = 0, 999999 do
    local p = {x = i, y = i, label = "item"}
    sum = sum + p.x
end
print(sum)
