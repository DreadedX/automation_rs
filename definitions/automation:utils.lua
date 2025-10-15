-- DO NOT MODIFY, FILE IS AUTOMATICALLY GENERATED
---@meta

local utils

---@class Timeout
local Timeout
---@async
---@param timeout number
---@param callback fun() | fun()[]
function Timeout:start(timeout, callback) end
---@async
function Timeout:cancel() end
---@async
---@return boolean
function Timeout:is_waiting() end
utils.Timeout = {}
---@return Timeout
function utils.Timeout.new() end

---@return string
function utils.get_hostname() end

---@return integer
function utils.get_epoch() end

return utils
