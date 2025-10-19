-- DO NOT MODIFY, FILE IS AUTOMATICALLY GENERATED
---@meta

---@class FulfillmentConfig
---@field openid_url string
---@field ip string?
---@field port integer?
local FulfillmentConfig

---@class Config
---@field fulfillment FulfillmentConfig
---@field devices Devices?
---@field mqtt MqttConfig
---@field schedule table<string, fun() | fun()[]>?
local Config

---@alias Devices (DeviceInterface | fun(client: AsyncClient): Devices)[]

---@class MqttConfig
---@field host string
---@field port integer
---@field client_name string
---@field username string
---@field password string
---@field tls boolean?
local MqttConfig

---@class AsyncClient
local AsyncClient
---@async
---@param topic string
---@param message table?
function AsyncClient:send_message(topic, message) end
