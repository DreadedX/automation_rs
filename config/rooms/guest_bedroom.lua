local battery = require("config.battery")
local devices = require("automation:devices")
local helper = require("config.helper")
local presence = require("config.presence")
local windows = require("config.windows")

--- @type Module
local module = {}

function module.setup(mqtt_client)
	local light = devices.LightOnOff.new({
		name = "Light",
		room = "Guest Room",
		topic = helper.mqtt_z2m("guest/light"),
		client = mqtt_client,
	})
	presence.turn_off_when_away(light)

	local window = devices.ContactSensor.new({
		name = "Window",
		room = "Guest Room",
		topic = helper.mqtt_z2m("guest/window"),
		client = mqtt_client,
		battery_callback = battery.callback,
	})
	windows.add(window)

	--- @type Module
	return {
		light,
		window,
	}
end

return module
