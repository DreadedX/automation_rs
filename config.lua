local devices = require("automation:devices")
local device_manager = require("automation:device_manager")
local utils = require("automation:utils")
local secrets = require("automation:secrets")
local debug = require("automation:variables").debug and true or false

print(_VERSION)

local host = utils.get_hostname()
print("Running @" .. host)

--- @param topic string
--- @return string
local function mqtt_z2m(topic)
	return "zigbee2mqtt/" .. topic
end

--- @param topic string
--- @return string
local function mqtt_automation(topic)
	return "automation/" .. topic
end

local mqtt_client = require("automation:mqtt").new(device_manager, {
	host = ((host == "zeus" or host == "hephaestus") and "olympus.lan.huizinga.dev") or "mosquitto",
	port = 8883,
	client_name = "automation-" .. host,
	username = "mqtt",
	password = secrets.mqtt_password,
	tls = host == "zeus" or host == "hephaestus",
})

local ntfy_topic = secrets.ntfy_topic
if ntfy_topic == nil then
	error("Ntfy topic is not specified")
end
local ntfy = devices.Ntfy.new({
	topic = ntfy_topic,
})
device_manager:add(ntfy)

--- @type {[string]: number}
local low_battery = {}
--- @param device DeviceInterface
--- @param battery number
local function check_battery(device, battery)
	local id = device:get_id()
	if battery < 15 then
		print("Device '" .. id .. "' has low battery: " .. tostring(battery))
		low_battery[id] = battery
	else
		low_battery[id] = nil
	end
end
device_manager:schedule("0 0 21 */1 * *", function()
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

	ntfy:send_notification({
		title = "Low battery",
		message = message,
		tags = { "battery" },
		priority = "default",
	})
end)

--- @class OnPresence
--- @field [integer] fun(presence: boolean)
local on_presence = {}
--- @param f fun(presence: boolean)
function on_presence:add(f)
	self[#self + 1] = f
end

local presence_system = devices.Presence.new({
	topic = mqtt_automation("presence/+/#"),
	client = mqtt_client,
	callback = function(_, presence)
		for _, f in ipairs(on_presence) do
			if type(f) == "function" then
				f(presence)
			end
		end
	end,
})
device_manager:add(presence_system)
on_presence:add(function(presence)
	ntfy:send_notification({
		title = "Presence",
		message = presence and "Home" or "Away",
		tags = { "house" },
		priority = "low",
		actions = {
			{
				action = "broadcast",
				extras = {
					cmd = "presence",
					state = presence and "0" or "1",
				},
				label = presence and "Set away" or "Set home",
				clear = true,
			},
		},
	})
end)
on_presence:add(function(presence)
	mqtt_client:send_message(mqtt_automation("debug") .. "/presence", {
		state = presence,
		updated = utils.get_epoch(),
	})
end)

--- @class WindowSensor
--- @field [integer] OpenCloseInterface
local window_sensors = {}
--- @param sensor OpenCloseInterface
function window_sensors:add(sensor)
	self[#self + 1] = sensor
end
on_presence:add(function(presence)
	if not presence then
		local open = {}
		for _, sensor in ipairs(window_sensors) do
			if sensor:open_percent() > 0 then
				local id = sensor:get_id()
				print("Open window detected: " .. id)
				table.insert(open, id)
			end
		end

		if #open > 0 then
			local message = table.concat(open, "\n")

			ntfy:send_notification({
				title = "Windows are open",
				message = message,
				tags = { "window" },
				priority = "high",
			})
		end
	end
end)

--- @param device OnOffInterface
local function turn_off_when_away(device)
	on_presence:add(function(presence)
		if not presence then
			device:set_on(false)
		end
	end)
end

--- @class OnLight
--- @field [integer] fun(light: boolean)
local on_light = {}
--- @param f fun(light: boolean)
function on_light:add(f)
	self[#self + 1] = f
end
device_manager:add(devices.LightSensor.new({
	identifier = "living_light_sensor",
	topic = mqtt_z2m("living/light"),
	client = mqtt_client,
	min = 22000,
	max = 23500,
	callback = function(_, light)
		for _, f in ipairs(on_light) do
			if type(f) == "function" then
				f(light)
			end
		end
	end,
}))
on_light:add(function(light)
	mqtt_client:send_message(mqtt_automation("debug") .. "/darkness", {
		state = not light,
		updated = utils.get_epoch(),
	})
end)

local hue_ip = "10.0.0.102"
local hue_token = secrets.hue_token
if hue_token == nil then
	error("Hue token is not specified")
end

local hue_bridge = devices.HueBridge.new({
	identifier = "hue_bridge",
	ip = hue_ip,
	login = hue_token,
	flags = {
		presence = 41,
		darkness = 43,
	},
})
device_manager:add(hue_bridge)
on_light:add(function(light)
	hue_bridge:set_flag("darkness", not light)
end)
on_presence:add(function(presence)
	hue_bridge:set_flag("presence", presence)
end)

local kitchen_lights = devices.HueGroup.new({
	identifier = "kitchen_lights",
	ip = hue_ip,
	login = hue_token,
	group_id = 7,
	scene_id = "7MJLG27RzeRAEVJ",
})
device_manager:add(kitchen_lights)
local living_lights = devices.HueGroup.new({
	identifier = "living_lights",
	ip = hue_ip,
	login = hue_token,
	group_id = 1,
	scene_id = "SNZw7jUhQ3cXSjkj",
})
device_manager:add(living_lights)
local living_lights_relax = devices.HueGroup.new({
	identifier = "living_lights",
	ip = hue_ip,
	login = hue_token,
	group_id = 1,
	scene_id = "eRJ3fvGHCcb6yNw",
})
device_manager:add(living_lights_relax)

device_manager:add(devices.HueSwitch.new({
	name = "Switch",
	room = "Living",
	client = mqtt_client,
	topic = mqtt_z2m("living/switch"),
	left_callback = function()
		kitchen_lights:set_on(not kitchen_lights:on())
	end,
	right_callback = function()
		living_lights:set_on(not living_lights:on())
	end,
	right_hold_callback = function()
		living_lights_relax:set_on(true)
	end,
	battery_callback = check_battery,
}))

device_manager:add(devices.WakeOnLAN.new({
	name = "Zeus",
	room = "Living Room",
	topic = mqtt_automation("appliance/living_room/zeus"),
	client = mqtt_client,
	mac_address = "30:9c:23:60:9c:13",
	broadcast_ip = "10.0.3.255",
}))

local living_mixer = devices.OutletOnOff.new({
	name = "Mixer",
	room = "Living Room",
	topic = mqtt_z2m("living/mixer"),
	client = mqtt_client,
})
turn_off_when_away(living_mixer)
device_manager:add(living_mixer)
local living_speakers = devices.OutletOnOff.new({
	name = "Speakers",
	room = "Living Room",
	topic = mqtt_z2m("living/speakers"),
	client = mqtt_client,
})
turn_off_when_away(living_speakers)
device_manager:add(living_speakers)

device_manager:add(devices.IkeaRemote.new({
	name = "Remote",
	room = "Living Room",
	client = mqtt_client,
	topic = mqtt_z2m("living/remote"),
	single_button = true,
	callback = function(_, on)
		if on then
			if living_mixer:on() then
				living_mixer:set_on(false)
				living_speakers:set_on(false)
			else
				living_mixer:set_on(true)
				living_speakers:set_on(true)
			end
		else
			if not living_mixer:on() then
				living_mixer:set_on(true)
			else
				living_speakers:set_on(not living_speakers:on())
			end
		end
	end,
	battery_callback = check_battery,
}))

--- @return fun(self: OnOffInterface, state: {state: boolean, power: number})
local function kettle_timeout()
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

--- @type OutletPower
local kettle = devices.OutletPower.new({
	outlet_type = "Kettle",
	name = "Kettle",
	room = "Kitchen",
	topic = mqtt_z2m("kitchen/kettle"),
	client = mqtt_client,
	callback = kettle_timeout(),
})
turn_off_when_away(kettle)
device_manager:add(kettle)

--- @param on boolean
local function set_kettle(_, on)
	kettle:set_on(on)
end

device_manager:add(devices.IkeaRemote.new({
	name = "Remote",
	room = "Bedroom",
	client = mqtt_client,
	topic = mqtt_z2m("bedroom/remote"),
	single_button = true,
	callback = set_kettle,
	battery_callback = check_battery,
}))

device_manager:add(devices.IkeaRemote.new({
	name = "Remote",
	room = "Kitchen",
	client = mqtt_client,
	topic = mqtt_z2m("kitchen/remote"),
	single_button = true,
	callback = set_kettle,
	battery_callback = check_battery,
}))

--- @param duration number
--- @return fun(self: OnOffInterface, state: {state: boolean})
local function off_timeout(duration)
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

local bathroom_light = devices.LightOnOff.new({
	name = "Light",
	room = "Bathroom",
	topic = mqtt_z2m("bathroom/light"),
	client = mqtt_client,
	callback = off_timeout(debug and 60 or 45 * 60),
})
device_manager:add(bathroom_light)

device_manager:add(devices.Washer.new({
	identifier = "bathroom_washer",
	topic = mqtt_z2m("bathroom/washer"),
	client = mqtt_client,
	threshold = 1,
	done_callback = function()
		ntfy:send_notification({
			title = "Laundy is done",
			message = "Don't forget to hang it!",
			tags = { "womans_clothes" },
			priority = "high",
		})
	end,
}))

device_manager:add(devices.OutletOnOff.new({
	name = "Charger",
	room = "Workbench",
	topic = mqtt_z2m("workbench/charger"),
	client = mqtt_client,
	callback = off_timeout(debug and 5 or 20 * 3600),
}))

local workbench_outlet = devices.OutletOnOff.new({
	name = "Outlet",
	room = "Workbench",
	topic = mqtt_z2m("workbench/outlet"),
	client = mqtt_client,
})
turn_off_when_away(workbench_outlet)
device_manager:add(workbench_outlet)

local workbench_light = devices.LightColorTemperature.new({
	name = "Light",
	room = "Workbench",
	topic = mqtt_z2m("workbench/light"),
	client = mqtt_client,
})
turn_off_when_away(workbench_light)
device_manager:add(workbench_light)

local delay_color_temp = utils.Timeout.new()
device_manager:add(devices.IkeaRemote.new({
	name = "Remote",
	room = "Workbench",
	client = mqtt_client,
	topic = mqtt_z2m("workbench/remote"),
	callback = function(_, on)
		delay_color_temp:cancel()
		if on then
			workbench_light:set_brightness(82)
			-- NOTE: This light does NOT support changing both the brightness and color
			-- temperature at the same time, so we first change the brightness and once
			-- that is complete we change the color temperature, as that is less likely
			-- to have to actually change.
			delay_color_temp:start(0.5, function()
				workbench_light:set_color_temperature(3333)
			end)
		else
			workbench_light:set_on(false)
		end
	end,
	battery_callback = check_battery,
}))

local hallway_top_light = devices.HueGroup.new({
	identifier = "hallway_top_light",
	ip = hue_ip,
	login = hue_token,
	group_id = 83,
	scene_id = "QeufkFDICEHWeKJ7",
})
device_manager:add(devices.HueSwitch.new({
	name = "SwitchBottom",
	room = "Hallway",
	client = mqtt_client,
	topic = mqtt_z2m("hallway/switchbottom"),
	left_callback = function()
		hallway_top_light:set_on(not hallway_top_light:on())
	end,
	battery_callback = check_battery,
}))
device_manager:add(devices.HueSwitch.new({
	name = "SwitchTop",
	room = "Hallway",
	client = mqtt_client,
	topic = mqtt_z2m("hallway/switchtop"),
	left_callback = function()
		hallway_top_light:set_on(not hallway_top_light:on())
	end,
	battery_callback = check_battery,
}))

local hallway_light_automation = {
	timeout = utils.Timeout.new(),
	forced = false,
	trash = nil,
	door = nil,
}
---@return fun(_, on: boolean)
function hallway_light_automation:switch_callback()
	return function(_, on)
		self.timeout:cancel()
		self.group.set_on(on)
		self.forced = on
	end
end
---@return fun(_, open: boolean)
function hallway_light_automation:door_callback()
	return function(_, open)
		if open then
			self.timeout:cancel()

			self.group.set_on(true)
		elseif not self.forced then
			self.timeout:start(debug and 10 or 2 * 60, function()
				if self.trash == nil or self.trash:open_percent() == 0 then
					self.group.set_on(false)
				end
			end)
		end
	end
end
---@return fun(_, open: boolean)
function hallway_light_automation:trash_callback()
	return function(_, open)
		if open then
			self.group.set_on(true)
		else
			if
				not self.timeout:is_waiting()
				and (self.door == nil or self.door:open_percent() == 0)
				and not self.forced
			then
				self.group.set_on(false)
			end
		end
	end
end
---@return fun(_, state: { on: boolean })
function hallway_light_automation:light_callback()
	return function(_, state)
		if
			state.on
			and (self.trash == nil or self.trash:open_percent()) == 0
			and (self.door == nil or self.door:open_percent() == 0)
		then
			-- If the door and trash are not open, that means the light got turned on manually
			self.timeout:cancel()
			self.forced = true
		elseif not state.on then
			-- The light is never forced when it is off
			self.forced = false
		end
	end
end

local hallway_storage = devices.LightBrightness.new({
	name = "Storage",
	room = "Hallway",
	topic = mqtt_z2m("hallway/storage"),
	client = mqtt_client,
	callback = hallway_light_automation:light_callback(),
})
turn_off_when_away(hallway_storage)
device_manager:add(hallway_storage)

local hallway_bottom_lights = devices.HueGroup.new({
	identifier = "hallway_bottom_lights",
	ip = hue_ip,
	login = hue_token,
	group_id = 81,
	scene_id = "3qWKxGVadXFFG4o",
})
device_manager:add(hallway_bottom_lights)

hallway_light_automation.group = {
	set_on = function(on)
		if on then
			hallway_storage:set_brightness(80)
		else
			hallway_storage:set_on(false)
		end
		hallway_bottom_lights:set_on(on)
	end,
}

---@param duration number
---@return fun(_, open: boolean)
local function presence(duration)
	local timeout = utils.Timeout.new()

	return function(_, open)
		if open then
			timeout:cancel()

			if not presence_system:overall_presence() then
				mqtt_client:send_message(mqtt_automation("presence/contact/frontdoor"), {
					state = true,
					updated = utils.get_epoch(),
				})
			end
		else
			timeout:start(duration, function()
				mqtt_client:send_message(mqtt_automation("presence/contact/frontdoor"), nil)
			end)
		end
	end
end

device_manager:add(devices.IkeaRemote.new({
	name = "Remote",
	room = "Hallway",
	client = mqtt_client,
	topic = mqtt_z2m("hallway/remote"),
	callback = hallway_light_automation:switch_callback(),
	battery_callback = check_battery,
}))
local hallway_frontdoor = devices.ContactSensor.new({
	name = "Frontdoor",
	room = "Hallway",
	sensor_type = "Door",
	topic = mqtt_z2m("hallway/frontdoor"),
	client = mqtt_client,
	callback = {
		presence(debug and 10 or 15 * 60),
		hallway_light_automation:door_callback(),
	},
	battery_callback = check_battery,
})
device_manager:add(hallway_frontdoor)
window_sensors:add(hallway_frontdoor)
hallway_light_automation.door = hallway_frontdoor

local hallway_trash = devices.ContactSensor.new({
	name = "Trash",
	room = "Hallway",
	sensor_type = "Drawer",
	topic = mqtt_z2m("hallway/trash"),
	client = mqtt_client,
	callback = hallway_light_automation:trash_callback(),
	battery_callback = check_battery,
})
device_manager:add(hallway_trash)
hallway_light_automation.trash = hallway_trash

local guest_light = devices.LightOnOff.new({
	name = "Light",
	room = "Guest Room",
	topic = mqtt_z2m("guest/light"),
	client = mqtt_client,
})
turn_off_when_away(guest_light)
device_manager:add(guest_light)

local bedroom_air_filter = devices.AirFilter.new({
	name = "Air Filter",
	room = "Bedroom",
	url = "http://10.0.0.103",
})
device_manager:add(bedroom_air_filter)

local bedroom_lights = devices.HueGroup.new({
	identifier = "bedroom_lights",
	ip = hue_ip,
	login = hue_token,
	group_id = 3,
	scene_id = "PvRs-lGD4VRytL9",
})
device_manager:add(bedroom_lights)
local bedroom_lights_relax = devices.HueGroup.new({
	identifier = "bedroom_lights",
	ip = hue_ip,
	login = hue_token,
	group_id = 3,
	scene_id = "60tfTyR168v2csz",
})
device_manager:add(bedroom_lights_relax)

device_manager:add(devices.HueSwitch.new({
	name = "Switch",
	room = "Bedroom",
	client = mqtt_client,
	topic = mqtt_z2m("bedroom/switch"),
	left_callback = function()
		bedroom_lights:set_on(not bedroom_lights:on())
	end,
	left_hold_callback = function()
		bedroom_lights_relax:set_on(true)
	end,
	battery_callback = check_battery,
}))

local balcony = devices.ContactSensor.new({
	name = "Balcony",
	room = "Living Room",
	sensor_type = "Door",
	topic = mqtt_z2m("living/balcony"),
	client = mqtt_client,
	battery_callback = check_battery,
})
device_manager:add(balcony)
window_sensors:add(balcony)
local living_window = devices.ContactSensor.new({
	name = "Window",
	room = "Living Room",
	topic = mqtt_z2m("living/window"),
	client = mqtt_client,
	battery_callback = check_battery,
})
device_manager:add(living_window)
window_sensors:add(living_window)
local bedroom_window = devices.ContactSensor.new({
	name = "Window",
	room = "Bedroom",
	topic = mqtt_z2m("bedroom/window"),
	client = mqtt_client,
	battery_callback = check_battery,
})
device_manager:add(bedroom_window)
window_sensors:add(bedroom_window)
local guest_window = devices.ContactSensor.new({
	name = "Window",
	room = "Guest Room",
	topic = mqtt_z2m("guest/window"),
	client = mqtt_client,
	battery_callback = check_battery,
})
device_manager:add(guest_window)
window_sensors:add(guest_window)

local storage_light = devices.LightBrightness.new({
	name = "Light",
	room = "Storage",
	topic = mqtt_z2m("storage/light"),
	client = mqtt_client,
})
turn_off_when_away(storage_light)
device_manager:add(storage_light)

device_manager:add(devices.ContactSensor.new({
	name = "Door",
	room = "Storage",
	sensor_type = "Door",
	topic = mqtt_z2m("storage/door"),
	client = mqtt_client,
	callback = function(_, open)
		if open then
			storage_light:set_brightness(100)
		else
			storage_light:set_on(false)
		end
	end,
	battery_callback = check_battery,
}))

device_manager:schedule("0 0 19 * * *", function()
	bedroom_air_filter:set_on(true)
end)
device_manager:schedule("0 0 20 * * *", function()
	bedroom_air_filter:set_on(false)
end)

---@type Config
return {
	fulfillment = {
		openid_url = "https://login.huizinga.dev/api/oidc",
	},
}
