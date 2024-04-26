print("Hello from lua")

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

automation.device_manager:create(
	"debug_bridge",
	DebugBridge.new({
		topic = mqtt_automation("debug"),
		client = automation.mqtt_client,
	})
)

local hue_ip = "10.0.0.146"
local hue_token = automation.util.get_env("HUE_TOKEN")

automation.device_manager:create(
	"hue_bridge",
	HueBridge.new({
		ip = hue_ip,
		login = hue_token,
		flags = {
			presence = 41,
			darkness = 43,
		},
	})
)

automation.device_manager:create(
	"living_light_sensor",
	LightSensor.new({
		topic = mqtt_z2m("living/light"),
		min = 22000,
		max = 23500,
		event_channel = automation.event_channel,
	})
)

automation.device_manager:create(
	"living_zeus",
	WakeOnLAN.new({
		name = "Zeus",
		room = "Living Room",
		topic = mqtt_automation("appliance/living_room/zeus"),
		mac_address = "30:9c:23:60:9c:13",
		broadcast_ip = "10.0.0.255",
	})
)

local living_mixer = automation.device_manager:create("living_mixer", KasaOutlet.new({ ip = "10.0.0.49" }))
local living_speakers = automation.device_manager:create("living_speakers", KasaOutlet.new({ ip = "10.0.0.182" }))

automation.device_manager:create(
	"living_audio",
	AudioSetup.new({
		topic = mqtt_z2m("living/remote"),
		mixer = living_mixer,
		speakers = living_speakers,
	})
)

automation.device_manager:create(
	"kitchen_kettle",
	IkeaOutlet.new({
		outlet_type = "Kettle",
		name = "Kettle",
		room = "Kitchen",
		topic = mqtt_z2m("kitchen/kettle"),
		client = automation.mqtt_client,
		timeout = debug and 5 or 300,
		remotes = {
			{ topic = mqtt_z2m("bedroom/remote") },
			{ topic = mqtt_z2m("kitchen/remote") },
		},
	})
)

automation.device_manager:create(
	"batchroom_light",
	IkeaOutlet.new({
		outlet_type = "Light",
		name = "Light",
		room = "Bathroom",
		topic = mqtt_z2m("batchroom/light"),
		client = automation.mqtt_client,
		timeout = debug and 60 or 45 * 60,
	})
)

automation.device_manager:create(
	"bathroom_washer",
	Washer.new({
		topic = mqtt_z2m("batchroom/washer"),
		threshold = 1,
		event_channel = automation.event_channel,
	})
)

automation.device_manager:create(
	"workbench_charger",
	IkeaOutlet.new({
		outlet_type = "Charger",
		name = "Charger",
		room = "Workbench",
		topic = mqtt_z2m("workbench/charger"),
		client = automation.mqtt_client,
		timeout = debug and 5 or 20 * 3600,
	})
)

automation.device_manager:create(
	"workbench_outlet",
	IkeaOutlet.new({
		name = "Outlet",
		room = "Workbench",
		topic = mqtt_z2m("workbench/outlet"),
		client = automation.mqtt_client,
	})
)

local hallway_lights = automation.device_manager:create(
	"hallway_lights",
	HueGroup.new({
		ip = hue_ip,
		login = hue_token,
		group_id = 81,
		scene_id = "3qWKxGVadXFFG4o",
		timer_id = 1,
		remotes = {
			{ topic = mqtt_z2m("hallway/remote") },
		},
	})
)

automation.device_manager:create(
	"hallway_frontdoor",
	ContactSensor.new({
		topic = mqtt_z2m("hallway/frontdoor"),
		client = automation.mqtt_client,
		presence = {
			topic = mqtt_automation("presence/contact/frontdoor"),
			timeout = debug and 10 or 15 * 60,
		},
		trigger = {
			devices = { hallway_lights },
			timeout = debug and 10 or 2 * 60,
		},
	})
)

local bedroom_air_filter = automation.device_manager:create(
	"bedroom_air_filter",
	AirFilter.new({
		name = "Air Filter",
		room = "Bedroom",
		topic = "pico/filter/bedroom",
		client = automation.mqtt_client,
	})
)

-- TODO: Use the wrapped device bedroom_air_filter instead of the string
automation.device_manager:add_schedule({
	["0 0 19 * * *"] = {
		on = {
			"bedroom_air_filter",
		},
	},
	["0 0 20 * * *"] = {
		off = {
			"bedroom_air_filter",
		},
	},
})
