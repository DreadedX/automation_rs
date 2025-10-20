local battery = require("config.battery")
local devices = require("automation:devices")
local helper = require("config.helper")
local presence = require("config.presence")

local module = {}

--- @type SetupFunction
function module.setup(mqtt_client)
	local light = devices.LightBrightness.new({
		name = "Light",
		room = "Storage",
		topic = helper.mqtt_z2m("storage/light"),
		client = mqtt_client,
	})
	presence.turn_off_when_away(light)

	local door = devices.ContactSensor.new({
		name = "Door",
		room = "Storage",
		sensor_type = "Door",
		topic = helper.mqtt_z2m("storage/door"),
		client = mqtt_client,
		callback = function(_, open)
			if open then
				light:set_brightness(100)
			else
				light:set_on(false)
			end
		end,
		battery_callback = battery.callback,
	})

	return {
		light,
		door,
	}
end

return module
