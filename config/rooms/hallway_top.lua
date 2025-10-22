local battery = require("config.battery")
local devices = require("automation:devices")
local helper = require("config.helper")
local hue_bridge = require("config.hue_bridge")

--- @type Module
local module = {}

function module.setup(mqtt_client)
	local light = devices.HueGroup.new({
		identifier = "hallway_top_light",
		ip = hue_bridge.ip,
		login = hue_bridge.token,
		group_id = 83,
		scene_id = "QeufkFDICEHWeKJ7",
	})

	local top_switch = devices.HueSwitch.new({
		name = "SwitchTop",
		room = "Hallway",
		client = mqtt_client,
		topic = helper.mqtt_z2m("hallway/switchtop"),
		left_callback = function()
			light:set_on(not light:on())
		end,
		battery_callback = battery.callback,
	})

	local bottom_switch = devices.HueSwitch.new({
		name = "SwitchBottom",
		room = "Hallway",
		client = mqtt_client,
		topic = helper.mqtt_z2m("hallway/switchbottom"),
		left_callback = function()
			light:set_on(not light:on())
		end,
		battery_callback = battery.callback,
	})

	--- @type Module
	return {
		light,
		top_switch,
		bottom_switch,
	}
end

return module
