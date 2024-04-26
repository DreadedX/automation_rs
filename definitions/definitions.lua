--- @meta

automation = {}

--- @class Device
--- @class Config

--- @class DeviceManager
automation.device_manager = {}

--- @class MqttClient
automation.mqtt_client = {}

--- @param identifier string
--- @param config Config
--- @return Device
function automation.device_manager:create(identifier, config) end

--- @class DebugBridge
DebugBridge = {}

--- @class DebugBridgeConfig
--- @field topic string

--- @param config DebugBridgeConfig
--- @return Config
function DebugBridge.new(config) end
