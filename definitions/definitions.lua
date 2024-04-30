--- @meta

--- @class WrappedDevice
WrappedDevice = {}
--- @return string
function WrappedDevice:get_id() end

--- @class WrappedAsyncClient

--- @class EventChannel
--- @return EventChannel
function automation.device_manager:event_channel() end

automation = {}

automation.device_manager = {}
--- @param device WrappedDevice
function automation.device_manager:add(device) end

--- @param when string
--- @param func function
function automation.device_manager:schedule(when, func) end

automation.util = {}
--- @param env string
--- @return string
function automation.util.get_env(env) end

--- @class Fulfillment
--- @field openid_url string|nil
automation.fulfillment = {}

--- @class MqttConfig
--- @param config MqttConfig
--- @return WrappedAsyncClient
function automation.new_mqtt_client(config) end

--- TODO: Generate this automatically
--- @alias OutletType "Outlet"|"Kettle"|"Charger"|"Light"
--- @alias TriggerDevicesHelper WrappedDevice[]
