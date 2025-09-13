---@meta

local devices

---@class Action
---@field action "broadcast"
---@field extras table<string, string> | nil
---@field label string | nil
---@clear clear bool|nil

---@alias Priority
---| "min"
---| "low"
---| "default"
---| "high"
---| "max"

---@class Notification
---@field title string
---@field message string | nil
-- NOTE: It might be possible to specify this down to the actual possible values
---@field tags string[] | nil
---@field priority Priority | nil
---@field actions Action[] | nil

---@class Ntfy
local Ntfy
---@async
---@param notification Notification
function Ntfy:send_notification(notification) end

---@class NtfyConfig
---@field topic string

devices.Ntfy = {}
---@param config NtfyConfig
---@return Ntfy
function devices.Ntfy.new(config) end

return devices
