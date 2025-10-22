local devices = require("automation:devices")
local helper = require("config.helper")

--- @class LightModule: Module
local module = {}

--- @class OnPresence
--- @field [integer] fun(light: boolean)
local callbacks = {}

--- @param callback fun(light: boolean)
function module.add_callback(callback)
	table.insert(callbacks, callback)
end

--- @param _ DeviceInterface
--- @param light boolean
local function callback(_, light)
	for _, f in ipairs(callbacks) do
		f(light)
	end
end

--- @type LightSensor?
module.device = nil

--- @type SetupFunction
function module.setup(mqtt_client)
	module.device = devices.LightSensor.new({
		identifier = "living_light_sensor",
		topic = helper.mqtt_z2m("living/light"),
		client = mqtt_client,
		min = 22000,
		max = 23500,
		callback = callback,
	})

	--- @type Module
	return {
		module.device,
	}
end

return module
