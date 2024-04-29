print("Hello from lua")

automation.fulfillment = {
	openid_url = "https://login.huizinga.dev/api/oidc",
}

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

local mqtt_client = automation.new_mqtt_client({
	host = debug and "olympus.lan.huizinga.dev" or "mosquitto",
	port = 8883,
	client_name = debug and "automation-debug" or "automation_rs",
	username = "mqtt",
	password = automation.util.get_env("MQTT_PASSWORD"),
	tls = debug and true or false,
})

automation.device_manager:add(Ntfy.new({
	topic = automation.util.get_env("NTFY_TOPIC"),
	event_channel = automation.device_manager:event_channel(),
}))

automation.device_manager:add(Presence.new({
	topic = "automation_dev/presence/+/#",
	client = mqtt_client,
	event_channel = automation.device_manager:event_channel(),
}))

automation.device_manager:add(DebugBridge.new({
	identifier = "debug_bridge",
	topic = mqtt_automation("debug"),
	client = mqtt_client,
}))

local hue_ip = "10.0.0.146"
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

local living_mixer = KasaOutlet.new({ identifier = "living_mixer", ip = "10.0.0.49" })
automation.device_manager:add(living_mixer)
local living_speakers = KasaOutlet.new({ identifier = "living_speakers", ip = "10.0.0.182" })
automation.device_manager:add(living_speakers)

automation.device_manager:add(AudioSetup.new({
	identifier = "living_audio",
	topic = mqtt_z2m("living/remote"),
	client = mqtt_client,
	mixer = living_mixer,
	speakers = living_speakers,
}))

automation.device_manager:add(IkeaOutlet.new({
	outlet_type = "Kettle",
	name = "Kettle",
	room = "Kitchen",
	topic = mqtt_z2m("kitchen/kettle"),
	client = mqtt_client,
	timeout = debug and 5 or 300,
	remotes = {
		{ topic = mqtt_z2m("bedroom/remote") },
		{ topic = mqtt_z2m("kitchen/remote") },
	},
}))

automation.device_manager:add(IkeaOutlet.new({
	outlet_type = "Light",
	name = "Light",
	room = "Bathroom",
	topic = mqtt_z2m("batchroom/light"),
	client = mqtt_client,
	timeout = debug and 60 or 45 * 60,
}))

automation.device_manager:add(Washer.new({
	identifier = "bathroom_washer",
	topic = mqtt_z2m("batchroom/washer"),
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
	timeout = debug and 5 or 20 * 3600,
}))

automation.device_manager:add(IkeaOutlet.new({
	name = "Outlet",
	room = "Workbench",
	topic = mqtt_z2m("workbench/outlet"),
	client = mqtt_client,
}))

local hallway_lights = automation.device_manager:add(HueGroup.new({
	identifier = "hallway_lights",
	ip = hue_ip,
	login = hue_token,
	group_id = 81,
	scene_id = "3qWKxGVadXFFG4o",
	timer_id = 1,
	remotes = {
		{ topic = mqtt_z2m("hallway/remote") },
	},
	client = mqtt_client,
}))

automation.device_manager:add(ContactSensor.new({
	identifier = "hallway_frontdoor",
	topic = mqtt_z2m("hallway/frontdoor"),
	client = mqtt_client,
	presence = {
		topic = mqtt_automation("presence/contact/frontdoor"),
		timeout = debug and 10 or 15 * 60,
	},
	trigger = {
		devices = { hallway_lights },
		timeout = debug and 10 or 2 * 60,
	},
}))

local bedroom_air_filter = AirFilter.new({
	name = "Air Filter",
	room = "Bedroom",
	topic = "pico/filter/bedroom",
	client = mqtt_client,
})
automation.device_manager:add(bedroom_air_filter)

automation.device_manager:schedule("0/1 * * * * *", function()
	print("Device: " .. bedroom_air_filter:get_id())
end)
