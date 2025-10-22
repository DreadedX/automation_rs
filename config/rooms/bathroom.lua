local debug = require("config.debug")
local devices = require("automation:devices")
local helper = require("config.helper")
local ntfy = require("config.ntfy")

--- @type Module
local module = {}

function module.setup(mqtt_client)
	local light = devices.LightOnOff.new({
		name = "Light",
		room = "Bathroom",
		topic = helper.mqtt_z2m("bathroom/light"),
		client = mqtt_client,
		callback = helper.off_timeout(debug.debug_mode and 60 or 45 * 60),
	})

	local washer = devices.Washer.new({
		identifier = "bathroom_washer",
		topic = helper.mqtt_z2m("bathroom/washer"),
		client = mqtt_client,
		threshold = 1,
		done_callback = function()
			ntfy.send_notification({
				title = "Laundy is done",
				message = "Don't forget to hang it!",
				tags = { "womans_clothes" },
				priority = "high",
			})
		end,
	})

	--- @type Module
	return {
		light,
		washer,
	}
end

return module
