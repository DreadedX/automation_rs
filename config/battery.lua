local ntfy = require("config.ntfy")

--- @class BatteryModule: Module
local module = {}

--- @type {[string]: number}
local low_battery = {}

--- @param device DeviceInterface
--- @param battery number
function module.callback(device, battery)
	local id = device:get_id()
	if battery < 15 then
		print("Device '" .. id .. "' has low battery: " .. tostring(battery))
		low_battery[id] = battery
	else
		low_battery[id] = nil
	end
end

local function notify_low_battery()
	-- Don't send notifications if there are now devices with low battery
	if next(low_battery) == nil then
		print("No devices with low battery")
		return
	end

	local lines = {}
	for name, battery in pairs(low_battery) do
		table.insert(lines, name .. ": " .. tostring(battery) .. "%")
	end
	local message = table.concat(lines, "\n")

	ntfy.send_notification({
		title = "Low battery",
		message = message,
		tags = { "battery" },
		priority = "default",
	})
end

--- @type Schedule
module.schedule = {
	["0 0 21 */1 * *"] = notify_low_battery,
}

return module
