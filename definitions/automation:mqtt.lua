-- DO NOT MODIFY, FILE IS AUTOMATICALLY GENERATED
---@meta

local mqtt

---@class MqttConfig
---@field host string
---@field port integer
---@field client_name string
---@field username string
---@field password string
---@field tls boolean
local MqttConfig

---@class AsyncClient
local AsyncClient
---@async
---@param topic string
---@param message table?
function AsyncClient:send_message(topic, message) end
mqtt.AsyncClient = {}
---@param device_manager DeviceManager
---@param config MqttConfig
---@return AsyncClient
function mqtt.new(device_manager, config) end

return mqtt
