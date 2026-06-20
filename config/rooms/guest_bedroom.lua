local battery = require("config.battery")
local devices = require("automation:devices")
local helper = require("config.helper")
local presence = require("config.presence")
local windows = require("config.windows")

local secrets = require("automation:secrets")

--- @type Module
local module = {}

function module.setup(mqtt_client)
	local light = nil

	local bambu = devices.Bambu.new({
		host = "10.0.0.108",
		device_id = secrets.printer_device_id,
		access_code = secrets.printer_access_code,
		callbacks = {
			connected = function(self)
				if light ~= nil then
					self:set_on(light:on())
				end
			end,
		},
	})

	light = devices.LightOnOff.new({
		name = "Light",
		room = "Guest Room",
		topic = helper.mqtt_z2m("guest/light"),
		client = mqtt_client,
		callback = function(_, state)
			bambu:set_on(state.state)
		end,
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

	local printer = devices.OutletOnOff.new({
		name = "3D Printer",
		room = "Guest Room",
		topic = helper.mqtt_z2m("guest/printer"),
		client = mqtt_client,
	})

	--- @type Module
	return {
		light,
		window,
		printer,
		bambu,
	}
end

return module
