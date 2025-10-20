local battery = require("config.battery")
local devices = require("automation:devices")
local helper = require("config.helper")
local hue_bridge = require("config.hue_bridge")
local presence = require("config.presence")

local module = {}

--- @type HueGroup?
local lights = nil

--- @type SetupFunction
function module.setup(mqtt_client)
	lights = devices.HueGroup.new({
		identifier = "kitchen_lights",
		ip = hue_bridge.ip,
		login = hue_bridge.token,
		group_id = 7,
		scene_id = "7MJLG27RzeRAEVJ",
	})

	local kettle = devices.OutletPower.new({
		outlet_type = "Kettle",
		name = "Kettle",
		room = "Kitchen",
		topic = helper.mqtt_z2m("kitchen/kettle"),
		client = mqtt_client,
		callback = helper.auto_off(),
	})
	presence.turn_off_when_away(kettle)

	local kettle_remote = devices.IkeaRemote.new({
		name = "Remote",
		room = "Kitchen",
		client = mqtt_client,
		topic = helper.mqtt_z2m("kitchen/remote"),
		single_button = true,
		callback = function(_, on)
			kettle:set_on(on)
		end,
		battery_callback = battery.callback,
	})

	local kettle_remote_bedroom = devices.IkeaRemote.new({
		name = "Remote",
		room = "Bedroom",
		client = mqtt_client,
		topic = helper.mqtt_z2m("bedroom/remote"),
		single_button = true,
		callback = function(_, on)
			kettle:set_on(on)
		end,
		battery_callback = battery.callback,
	})

	return {
		lights,
		kettle,
		kettle_remote,
		kettle_remote_bedroom,
	}
end

function module.toggle_lights()
	if lights then
		lights:set_on(not lights:on())
	end
end

return module
