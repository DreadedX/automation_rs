---@meta

---@class DeviceManager
local DeviceManager
---@param device DeviceInterface
function DeviceManager:add(device) end

---@param cron string
---@param callback fun()
function DeviceManager:schedule(cron, callback) end

return DeviceManager
