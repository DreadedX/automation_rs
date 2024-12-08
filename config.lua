print("Hello from lua")

local host = automation.util.get_hostname()
print("Running @" .. host)

local debug, value = pcall(automation.util.get_env, "DEBUG")
if debug and value ~= "true" then
	debug = false
end

local function mqtt_z2m(topic)
	return "zigbee2mqtt/" .. topic
end

local function mqtt_automation(topic)
	return "automation/" .. topic
end

automation.fulfillment = {
	openid_url = "https://login.huizinga.dev/api/oidc",
}

local mqtt_client = automation.new_mqtt_client({
	host = (host == "zeus" and "olympus.lan.huizinga.dev")
		or (host == "hephaestus" and "olympus.vpn.huizinga.dev")
		or "mosquitto",
	port = 8883,
	client_name = "automation-" .. host,
	username = "mqtt",
	password = automation.util.get_env("MQTT_PASSWORD"),
	tls = host == "zeus" or host == "hephaestus",
})

automation.device_manager:add(Ntfy.new({
	topic = automation.util.get_env("NTFY_TOPIC"),
	event_channel = automation.device_manager:event_channel(),
}))

automation.device_manager:add(Presence.new({
	topic = mqtt_automation("presence/+/#"),
	client = mqtt_client,
	event_channel = automation.device_manager:event_channel(),
}))

automation.device_manager:add(DebugBridge.new({
	identifier = "debug_bridge",
	topic = mqtt_automation("debug"),
	client = mqtt_client,
}))

local hue_ip = "10.0.0.136"
local hue_token = automation.util.get_env("HUE_TOKEN")

automation.device_manager:add(HueBridge.new({
	identifier = "hue_bridge",
	ip = hue_ip,
	login = hue_token,
	flags = {
		presence = 41,
		darkness = 43,
	},
}))

automation.device_manager:add(LightSensor.new({
	identifier = "living_light_sensor",
	topic = mqtt_z2m("living/light"),
	client = mqtt_client,
	min = 22000,
	max = 23500,
	event_channel = automation.device_manager:event_channel(),
}))

automation.device_manager:add(WakeOnLAN.new({
	name = "Zeus",
	room = "Living Room",
	topic = mqtt_automation("appliance/living_room/zeus"),
	client = mqtt_client,
	mac_address = "30:9c:23:60:9c:13",
	broadcast_ip = "10.0.0.255",
}))

local living_mixer = KasaOutlet.new({ identifier = "living_mixer", ip = "10.0.0.84" })
automation.device_manager:add(living_mixer)
local living_speakers = KasaOutlet.new({ identifier = "living_speakers", ip = "10.0.0.127" })
automation.device_manager:add(living_speakers)

automation.device_manager:add(IkeaRemote.new({
	name = "Remote",
	room = "Living",
	client = mqtt_client,
	topic = mqtt_z2m("living/remote"),
	single_button = true,
	callback = function(_, on)
		if on then
			if living_mixer:is_on() then
				living_mixer:set_on(false)
				living_speakers:set_on(false)
			else
				living_mixer:set_on(true)
				living_speakers:set_on(true)
			end
		else
			if not living_mixer:is_on() then
				living_mixer:set_on(true)
			else
				living_speakers:set_on(not living_speakers:is_on())
			end
		end
	end,
}))

local function off_timeout(duration)
	local timeout = Timeout.new()

	return function(self, on)
		if on then
			timeout:start(duration, function()
				self:set_on(false)
			end)
		else
			timeout:cancel()
		end
	end
end

local kettle = IkeaOutlet.new({
	outlet_type = "Kettle",
	name = "Kettle",
	room = "Kitchen",
	topic = mqtt_z2m("kitchen/kettle"),
	client = mqtt_client,
	callback = off_timeout(debug and 5 or 300),
})
automation.device_manager:add(kettle)

local function set_kettle(_, on)
	kettle:set_on(on)
end

automation.device_manager:add(IkeaRemote.new({
	name = "Remote",
	room = "Bedroom",
	client = mqtt_client,
	topic = mqtt_z2m("bedroom/remote"),
	single_button = true,
	callback = set_kettle,
}))

automation.device_manager:add(IkeaRemote.new({
	name = "Remote",
	room = "Kitchen",
	client = mqtt_client,
	topic = mqtt_z2m("kitchen/remote"),
	single_button = true,
	callback = set_kettle,
}))

automation.device_manager:add(IkeaOutlet.new({
	outlet_type = "Light",
	name = "Light",
	room = "Bathroom",
	topic = mqtt_z2m("bathroom/light"),
	client = mqtt_client,
	callback = off_timeout(debug and 60 or 45 * 60),
}))

automation.device_manager:add(Washer.new({
	identifier = "bathroom_washer",
	topic = mqtt_z2m("bathroom/washer"),
	client = mqtt_client,
	threshold = 1,
	event_channel = automation.device_manager:event_channel(),
}))

automation.device_manager:add(IkeaOutlet.new({
	outlet_type = "Charger",
	name = "Charger",
	room = "Workbench",
	topic = mqtt_z2m("workbench/charger"),
	client = mqtt_client,
	callback = off_timeout(debug and 5 or 20 * 3600),
}))

automation.device_manager:add(IkeaOutlet.new({
	name = "Outlet",
	room = "Workbench",
	topic = mqtt_z2m("workbench/outlet"),
	client = mqtt_client,
}))

local hallway_top_light = HueGroup.new({
	identifier = "hallway_top_light",
	ip = hue_ip,
	login = hue_token,
	group_id = 83,
	scene_id = "QeufkFDICEHWeKJ7",
	client = mqtt_client,
})
automation.device_manager:add(HueSwitch.new({
	name = "SwitchBottom",
	room = "Hallway",
	client = mqtt_client,
	topic = mqtt_z2m("hallway/switchbottom"),
	left_callback = function()
		hallway_top_light:set_on(not hallway_top_light:is_on())
	end,
}))
automation.device_manager:add(HueSwitch.new({
	name = "SwitchTop",
	room = "Hallway",
	client = mqtt_client,
	topic = mqtt_z2m("hallway/switchtop"),
	left_callback = function()
		hallway_top_light:set_on(not hallway_top_light:is_on())
	end,
}))

local hallway_bottom_lights = HueGroup.new({
	identifier = "hallway_bottom_lights",
	ip = hue_ip,
	login = hue_token,
	group_id = 81,
	scene_id = "3qWKxGVadXFFG4o",
	client = mqtt_client,
})
automation.device_manager:add(hallway_bottom_lights)

local hallway_light_automation = {
	group = hallway_bottom_lights,
	timeout = Timeout.new(),
	state = {
		door_open = false,
		trash_open = false,
		forced = false,
	},
	switch_callback = function(self, on)
		self.timeout:cancel()
		self.group:set_on(on)
		self.state.forced = on
	end,
	door_callback = function(self, open)
		self.state.door_open = open
		if open then
			self.timeout:cancel()

			self.group:set_on(true)
		elseif not self.state.forced then
			self.timeout:start(debug and 10 or 60, function()
				if not self.state.trash_open then
					self.group:set_on(false)
				end
			end)
		end
	end,
	trash_callback = function(self, open)
		self.state.trash_open = open
		if open then
			self.group:set_on(true)
		else
			if not self.timeout:is_waiting() and not self.state.door_open and not self.state.forced then
				self.group:set_on(false)
			end
		end
	end,
}

automation.device_manager:add(IkeaRemote.new({
	name = "Remote",
	room = "Hallway",
	client = mqtt_client,
	topic = mqtt_z2m("hallway/remote"),
	callback = function(_, on)
		hallway_light_automation:switch_callback(on)
	end,
}))
automation.device_manager:add(ContactSensor.new({
	identifier = "hallway_frontdoor",
	topic = mqtt_z2m("hallway/frontdoor"),
	client = mqtt_client,
	presence = {
		topic = mqtt_automation("presence/contact/frontdoor"),
		timeout = debug and 10 or 15 * 60,
	},
	callback = function(_, open)
		hallway_light_automation:door_callback(open)
	end,
}))
automation.device_manager:add(ContactSensor.new({
	identifier = "hallway_trash",
	topic = mqtt_z2m("hallway/trash"),
	client = mqtt_client,
	callback = function(_, open)
		hallway_light_automation:trash_callback(open)
	end,
}))

automation.device_manager:add(IkeaOutlet.new({
	outlet_type = "Light",
	name = "Light",
	room = "Guest",
	topic = mqtt_z2m("guest/light"),
	client = mqtt_client,
}))

local bedroom_air_filter = AirFilter.new({
	name = "Air Filter",
	room = "Bedroom",
	topic = "pico/filter/bedroom",
	client = mqtt_client,
})
automation.device_manager:add(bedroom_air_filter)

automation.device_manager:schedule("0 0 19 * * *", function()
	bedroom_air_filter:set_on(true)
end)
automation.device_manager:schedule("0 0 20 * * *", function()
	bedroom_air_filter:set_on(false)
end)
