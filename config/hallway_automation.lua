local debug = require("config.debug")
local utils = require("automation:utils")

local module = {}

local timeout = utils.Timeout.new()
local forced = false
--- @type OpenCloseInterface?
local trash = nil
--- @type OpenCloseInterface?
local door = nil

--- @type fun(on: boolean)[]
local callbacks = {}

--- @param on boolean
local function callback(on)
	for _, f in ipairs(callbacks) do
		f(on)
	end
end

---@type fun(device: DeviceInterface, on: boolean)
function module.switch_callback(_, on)
	timeout:cancel()
	callback(on)
	forced = on
end

---@type fun(device: DeviceInterface, open: boolean)
function module.door_callback(_, open)
	if open then
		timeout:cancel()

		callback(true)
	elseif not forced then
		timeout:start(debug.debug_mode and 10 or 2 * 60, function()
			if trash == nil or trash:open_percent() == 0 then
				callback(false)
			end
		end)
	end
end

---@type fun(device: DeviceInterface, open: boolean)
function module.trash_callback(_, open)
	if open then
		callback(true)
	else
		if not forced and not timeout:is_waiting() and (door == nil or door:open_percent() == 0) then
			callback(false)
		end
	end
end

---@type fun(device: DeviceInterface, state: { state: boolean })
function module.light_callback(_, state)
	print("LIGHT = " .. tostring(state.state))
	if state.state and (trash == nil or trash:open_percent()) == 0 and (door == nil or door:open_percent() == 0) then
		-- If the door and trash are not open, that means the light got turned on manually
		timeout:cancel()
		forced = true
	elseif not state.state then
		-- The light is never forced when it is off
		forced = false
	end
end

--- @param t OpenCloseInterface
function module.set_trash(t)
	trash = t
end

--- @param d OpenCloseInterface
function module.set_door(d)
	door = d
end

--- @param c fun(on: boolean)
function module.add_callback(c)
	table.insert(callbacks, c)
end

return module
