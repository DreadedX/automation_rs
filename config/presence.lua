local devices = require("automation:devices")
local helper = require("config.helper")
local ntfy = require("config.ntfy")

--- @class PresenceModule: Module
local module = {}

--- @class OnPresence
--- @field [integer] fun(presence: boolean)
local callbacks = {}

--- @param callback fun(presence: boolean)
function module.add_callback(callback)
	table.insert(callbacks, callback)
end

--- @param device OnOffInterface
function module.turn_off_when_away(device)
	module.add_callback(function(presence)
		if not presence then
			device:set_on(false)
		end
	end)
end

--- @param _ DeviceInterface
--- @param presence boolean
local function callback(_, presence)
	for _, f in ipairs(callbacks) do
		f(presence)
	end
end

--- @type Presence?
local presence = nil

--- @type SetupFunction
function module.setup(mqtt_client)
	presence = devices.Presence.new({
		topic = helper.mqtt_automation("presence/+/#"),
		client = mqtt_client,
		callback = callback,
	})

	module.add_callback(function(p)
		ntfy.send_notification({
			title = "Presence",
			message = p and "Home" or "Away",
			tags = { "house" },
			priority = "low",
			actions = {
				{
					action = "broadcast",
					extras = {
						cmd = "presence",
						state = p and "0" or "1",
					},
					label = p and "Set away" or "Set home",
					clear = true,
				},
			},
		})
	end)

	--- @type Module
	return {
		presence,
	}
end

function module.overall_presence()
	-- Default to no presence when the device has not been created yet
	if not presence then
		return false
	end

	return presence:overall_presence()
end

return module
