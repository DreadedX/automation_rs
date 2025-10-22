local battery = require("config.battery")
local devices = require("automation:devices")
local helper = require("config.helper")
local hue_bridge = require("config.hue_bridge")
local windows = require("config.windows")

--- @type Module
local module = {}

--- @type AirFilter?
local air_filter = nil

function module.setup(mqtt_client)
	local lights = devices.HueGroup.new({
		identifier = "bedroom_lights",
		ip = hue_bridge.ip,
		login = hue_bridge.token,
		group_id = 3,
		scene_id = "PvRs-lGD4VRytL9",
	})
	local lights_relax = devices.HueGroup.new({
		identifier = "bedroom_lights_relax",
		ip = hue_bridge.ip,
		login = hue_bridge.token,
		group_id = 3,
		scene_id = "60tfTyR168v2csz",
	})

	air_filter = devices.AirFilter.new({
		name = "Air Filter",
		room = "Bedroom",
		url = "http://10.0.0.103",
	})

	local switch = devices.HueSwitch.new({
		name = "Switch",
		room = "Bedroom",
		client = mqtt_client,
		topic = helper.mqtt_z2m("bedroom/switch"),
		left_callback = function()
			lights:set_on(not lights:on())
		end,
		left_hold_callback = function()
			lights_relax:set_on(true)
		end,
		battery_callback = battery.callback,
	})

	local window = devices.ContactSensor.new({
		name = "Window",
		room = "Bedroom",
		topic = helper.mqtt_z2m("bedroom/window"),
		client = mqtt_client,
		battery_callback = battery.callback,
	})
	windows.add(window)

	--- @type Module
	return {
		devices = {
			lights,
			lights_relax,
			air_filter,
			switch,
			window,
		},
		schedule = {
			["0 0 19 * * *"] = function()
				air_filter:set_on(true)
			end,
			["0 0 20 * * *"] = function()
				air_filter:set_on(false)
			end,
		},
	}
end

return module
