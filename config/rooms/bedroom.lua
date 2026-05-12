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
	local wardrobe_light = devices.HueGroup.new({
		identifier = "bedroom_lights_wardrobe",
		ip = hue_bridge.ip,
		login = hue_bridge.token,
		group_id = 3,
		scene_id = "1IDvpsN2YLZsDV95",
	})

	air_filter = devices.AirFilter.new({
		name = "Air Filter",
		room = "Bedroom",
		url = "http://10.0.0.103",
	})

	local wardrobe_door = devices.ContactSensor.new({
		name = "Wardrobe Door",
		room = "Bedroom",
		sensor_type = "Door",
		topic = helper.mqtt_z2m("bedroom/wardrobe_door"),
		client = mqtt_client,
		callback = function(_, open)
			-- Technically this has an edge case where if one of the spots is
			-- on, but that is not something I ever do
			if not lights:all_on() then
				wardrobe_light:set_on(open)
			end
		end,
		battery_callback = battery.callback,
	})

	local switch = devices.HueSwitch.new({
		name = "Switch",
		room = "Bedroom",
		client = mqtt_client,
		topic = helper.mqtt_z2m("bedroom/switch"),
		left_callback = function()
			local on = not lights:all_on()
			lights:set_on(on)
			-- This is a bit janky as the light will start to dim before turning
			-- back on, however this is really and edge case that probably won't
			-- happen often, so for now it's fine
			if not on and wardrobe_door:open_percent() == 100 then
				wardrobe_light:set_on(true)
			end
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
			wardrobe_door,
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
