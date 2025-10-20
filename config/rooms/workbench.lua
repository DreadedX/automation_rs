local battery = require("config.battery")
local debug = require("config.debug")
local devices = require("automation:devices")
local helper = require("config.helper")
local presence = require("config.presence")
local utils = require("automation:utils")

local module = {}

--- @type SetupFunction
function module.setup(mqtt_client)
	local charger = devices.OutletOnOff.new({
		name = "Charger",
		room = "Workbench",
		topic = helper.mqtt_z2m("workbench/charger"),
		client = mqtt_client,
		callback = helper.off_timeout(debug.debug_mode and 5 or 20 * 3600),
	})

	local outlets = devices.OutletOnOff.new({
		name = "Outlets",
		room = "Workbench",
		topic = helper.mqtt_z2m("workbench/outlet"),
		client = mqtt_client,
	})
	presence.turn_off_when_away(outlets)

	local light = devices.LightColorTemperature.new({
		name = "Light",
		room = "Workbench",
		topic = helper.mqtt_z2m("workbench/light"),
		client = mqtt_client,
	})
	presence.turn_off_when_away(light)

	local delay_color_temp = utils.Timeout.new()
	local remote = devices.IkeaRemote.new({
		name = "Remote",
		room = "Workbench",
		client = mqtt_client,
		topic = helper.mqtt_z2m("workbench/remote"),
		callback = function(_, on)
			delay_color_temp:cancel()
			if on then
				light:set_brightness(82)
				-- NOTE: This light does NOT support changing both the brightness and color
				-- temperature at the same time, so we first change the brightness and once
				-- that is complete we change the color temperature, as that is less likely
				-- to have to actually change.
				delay_color_temp:start(0.5, function()
					light:set_color_temperature(3333)
				end)
			else
				light:set_on(false)
			end
		end,
		battery_callback = battery.callback,
	})

	return {
		charger,
		outlets,
		light,
		remote,
	}
end

return module
