local battery = require("config.battery")
local devices = require("automation:devices")
local helper = require("config.helper")
local hue_bridge = require("config.hue_bridge")
local presence = require("config.presence")
local windows = require("config.windows")

local module = {}

--- @type SetupFunction
function module.setup(mqtt_client)
	local lights = devices.HueGroup.new({
		identifier = "living_lights",
		ip = hue_bridge.ip,
		login = hue_bridge.token,
		group_id = 1,
		scene_id = "SNZw7jUhQ3cXSjkj",
	})

	local lights_relax = devices.HueGroup.new({
		identifier = "living_lights_relax",
		ip = hue_bridge.ip,
		login = hue_bridge.token,
		group_id = 1,
		scene_id = "eRJ3fvGHCcb6yNw",
	})

	local switch = devices.HueSwitch.new({
		name = "Switch",
		room = "Living",
		client = mqtt_client,
		topic = helper.mqtt_z2m("living/switch"),
		left_callback = require("config.rooms.kitchen").toggle_lights,
		right_callback = function()
			lights:set_on(not lights:on())
		end,
		right_hold_callback = function()
			lights_relax:set_on(true)
		end,
		battery_callback = battery.callback,
	})

	local pc = devices.WakeOnLAN.new({
		name = "Zeus",
		room = "Living Room",
		topic = helper.mqtt_automation("appliance/living_room/zeus"),
		client = mqtt_client,
		mac_address = "30:9c:23:60:9c:13",
		broadcast_ip = "10.0.3.255",
	})

	local mixer = devices.OutletOnOff.new({
		name = "Mixer",
		room = "Living Room",
		topic = helper.mqtt_z2m("living/mixer"),
		client = mqtt_client,
	})
	presence.turn_off_when_away(mixer)

	local speakers = devices.OutletOnOff.new({
		name = "Speakers",
		room = "Living Room",
		topic = helper.mqtt_z2m("living/speakers"),
		client = mqtt_client,
	})
	presence.turn_off_when_away(speakers)

	local audio_remote = devices.IkeaRemote.new({
		name = "Remote",
		room = "Living Room",
		client = mqtt_client,
		topic = helper.mqtt_z2m("living/remote"),
		single_button = true,
		callback = function(_, on)
			if on then
				if mixer:on() then
					mixer:set_on(false)
					speakers:set_on(false)
				else
					mixer:set_on(true)
					speakers:set_on(true)
				end
			else
				if not mixer:on() then
					mixer:set_on(true)
				else
					speakers:set_on(not speakers:on())
				end
			end
		end,
		battery_callback = battery.callback,
	})

	local balcony = devices.ContactSensor.new({
		name = "Balcony",
		room = "Living Room",
		sensor_type = "Door",
		topic = helper.mqtt_z2m("living/balcony"),
		client = mqtt_client,
		battery_callback = battery.callback,
	})
	windows.add(balcony)
	local window = devices.ContactSensor.new({
		name = "Window",
		room = "Living Room",
		topic = helper.mqtt_z2m("living/window"),
		client = mqtt_client,
		battery_callback = battery.callback,
	})
	windows.add(window)

	return {
		lights,
		lights_relax,
		switch,
		pc,
		mixer,
		speakers,
		audio_remote,
		balcony,
		window,
	}
end

return module
