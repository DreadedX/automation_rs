local devices = require("automation:devices")
local secrets = require("automation:secrets")

--- @class NtfyModule: Module
local module = {}

local ntfy_topic = secrets.ntfy_topic
if ntfy_topic == nil then
	error("Ntfy topic is not specified")
end

--- @type Ntfy?
local ntfy = nil

--- @param notification Notification
function module.send_notification(notification)
	if ntfy then
		ntfy:send_notification(notification)
	end
end

--- @type SetupFunction
function module.setup()
	ntfy = devices.Ntfy.new({
		topic = ntfy_topic,
	})

	--- @type Module
	return {
		ntfy,
	}
end

return module
