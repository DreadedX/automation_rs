local devices = require("automation:devices")
local light = require("config.light")
local presence = require("config.presence")
local secrets = require("automation:secrets")

--- @class HueBridgeModule: Module
local module = {}

module.ip = "10.0.0.102"
module.token = secrets.hue_token

if module.token == nil then
	error("Hue token is not specified")
end

--- @type SetupFunction
function module.setup()
	local bridge = devices.HueBridge.new({
		identifier = "hue_bridge",
		ip = module.ip,
		login = module.token,
		flags = {
			presence = 41,
			darkness = 43,
		},
	})

	light.add_callback(function(l)
		bridge:set_flag("darkness", not l)
	end)

	presence.add_callback(function(p)
		bridge:set_flag("presence", p)
	end)

	return {
		bridge,
	}
end

return module
