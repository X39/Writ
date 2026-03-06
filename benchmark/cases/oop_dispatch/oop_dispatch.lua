local Base = {}
Base.__index = Base
function Base:new() return setmetatable({}, self) end
function Base:compute() return 0 end

local TypeA = setmetatable({}, {__index = Base})
TypeA.__index = TypeA
function TypeA:compute() return 1 end

local TypeB = setmetatable({}, {__index = Base})
TypeB.__index = TypeB
function TypeB:compute() return 2 end

local TypeC = setmetatable({}, {__index = Base})
TypeC.__index = TypeC
function TypeC:compute() return 3 end

local TypeD = setmetatable({}, {__index = Base})
TypeD.__index = TypeD
function TypeD:compute() return 4 end

local classes = {TypeA, TypeB, TypeC, TypeD}
local sum = 0
for i = 0, 99999 do
    local cls = classes[(i % 4) + 1]
    local obj = cls:new()
    sum = sum + obj:compute()
end
print(sum)
