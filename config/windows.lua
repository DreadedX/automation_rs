local ntfy = require("config.ntfy")
local presence = require("config.presence")

local module = {}

--- @class OnPresence
--- @field [integer] OpenCloseInterface
local sensors = {}

--- @param sensor OpenCloseInterface
function module.add(sensor)
	table.insert(sensors, sensor)
end

--- @type SetupFunction
function module.setup()
	presence.add_callback(function(p)
		if not p then
			local open = {}
			for _, sensor in ipairs(sensors) do
				if sensor:open_percent() > 0 then
					local id = sensor:get_id()
					print("Open window detected: " .. id)
					table.insert(open, id)
				end
			end

			if #open > 0 then
				local message = table.concat(open, "\n")

				ntfy.send_notification({
					title = "Windows are open",
					message = message,
					tags = { "window" },
					priority = "high",
				})
			end
		end
	end)
end

return module
