local utils = require("automation:utils")

local module = {}

--- @param topic string
--- @return string
function module.mqtt_z2m(topic)
	return "zigbee2mqtt/" .. topic
end

--- @param topic string
--- @return string
function module.mqtt_automation(topic)
	return "automation/" .. topic
end

--- @return fun(self: OnOffInterface, state: {state: boolean, power: number})
function module.auto_off()
	local timeout = utils.Timeout.new()

	return function(self, state)
		if state.state and state.power < 100 then
			timeout:start(3, function()
				self:set_on(false)
			end)
		else
			timeout:cancel()
		end
	end
end

--- @param duration number
--- @return fun(self: OnOffInterface, state: {state: boolean})
function module.off_timeout(duration)
	local timeout = utils.Timeout.new()

	return function(self, state)
		if state.state then
			timeout:start(duration, function()
				self:set_on(false)
			end)
		else
			timeout:cancel()
		end
	end
end

return module
