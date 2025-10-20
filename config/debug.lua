local helper = require("config.helper")
local light = require("config.light")
local presence = require("config.presence")
local utils = require("automation:utils")
local variables = require("automation:variables")

local module = {}

if variables.debug == "true" then
	module.debug_mode = true
elseif not variables.debug or variables.debug == "false" then
	module.debug_mode = false
else
	error("Variable debug has invalid value '" .. variables.debug .. "', expected 'true' or 'false'")
end

--- @type SetupFunction
function module.setup(mqtt_client)
	presence.add_callback(function(p)
		mqtt_client:send_message(helper.mqtt_automation("debug") .. "/presence", {
			state = p,
			updated = utils.get_epoch(),
		})
	end)

	light.add_callback(function(l)
		mqtt_client:send_message(helper.mqtt_automation("debug") .. "/darkness", {
			state = not l,
			updated = utils.get_epoch(),
		})
	end)
end

return module
