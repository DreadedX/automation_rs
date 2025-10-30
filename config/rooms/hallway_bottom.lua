local battery = require("config.battery")
local debug = require("config.debug")
local devices = require("automation:devices")
local hallway_automation = require("config.hallway_automation")
local helper = require("config.helper")
local hue_bridge = require("config.hue_bridge")
local presence = require("config.presence")
local utils = require("automation:utils")
local windows = require("config.windows")

--- @type Module
local module = {}

function module.setup(mqtt_client)
	local main_light = devices.HueGroup.new({
		identifier = "hallway_main_light",
		ip = hue_bridge.ip,
		login = hue_bridge.token,
		group_id = 81,
		scene_id = "3qWKxGVadXFFG4o",
	})
	hallway_automation.add_callback(function(on)
		main_light:set_on(on)
	end)

	local storage_light = devices.LightBrightness.new({
		name = "Storage",
		room = "Hallway",
		topic = helper.mqtt_z2m("hallway/storage"),
		client = mqtt_client,
		callback = hallway_automation.light_callback,
	})
	presence.turn_off_when_away(storage_light)
	hallway_automation.add_callback(function(on)
		if on then
			storage_light:set_brightness(80)
		else
			storage_light:set_on(false)
		end
	end)

	local remote = devices.IkeaRemote.new({
		name = "Remote",
		room = "Hallway",
		client = mqtt_client,
		topic = helper.mqtt_z2m("hallway/remote"),
		callback = hallway_automation.switch_callback,
		battery_callback = battery.callback,
	})

	local trash = devices.ContactSensor.new({
		name = "Trash",
		room = "Hallway",
		sensor_type = "Drawer",
		topic = helper.mqtt_z2m("hallway/trash"),
		client = mqtt_client,
		callback = hallway_automation.trash_callback,
		battery_callback = battery.callback,
	})
	hallway_automation.set_trash(trash)

	local timeout = utils.Timeout.new()
	local function frontdoor_presence(_, open)
		if open then
			timeout:cancel()

			if not presence.overall_presence() then
				mqtt_client:send_message(helper.mqtt_automation("presence/contact/frontdoor"), {
					state = true,
					updated = utils.get_epoch(),
				})
			end
		else
			timeout:start(debug.debug_mode and 10 or 15 * 60, function()
				mqtt_client:send_message(helper.mqtt_automation("presence/contact/frontdoor"), nil)
			end)
		end
	end

	local frontdoor = devices.ContactSensor.new({
		name = "Frontdoor",
		room = "Hallway",
		sensor_type = "Door",
		topic = helper.mqtt_z2m("hallway/frontdoor"),
		client = mqtt_client,
		callback = {
			frontdoor_presence,
			hallway_automation.door_callback,
		},
		battery_callback = battery.callback,
	})
	windows.add(frontdoor)
	hallway_automation.set_door(frontdoor)

	--- @type Module
	return {
		main_light,
		storage_light,
		remote,
		trash,
		frontdoor,
	}
end

return module
