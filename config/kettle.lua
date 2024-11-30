local h = require("helper")

return function(mqtt_client, debug)
	local kettle = IkeaOutlet.new({
		outlet_type = "Kettle",
		name = "Kettle",
		room = "Kitchen",
		topic = h.mqtt_z2m("kitchen/kettle"),
		client = mqtt_client,
		timeout = debug and 5 or 300,
	})
	automation.device_manager:add(kettle)

	local function set_kettle(on)
		kettle:set_on(on)
	end

	automation.device_manager:add(IkeaRemote.new({
		name = "Remote",
		room = "Bedroom",
		client = mqtt_client,
		topic = h.mqtt_z2m("bedroom/remote"),
		single_button = true,
		callback = set_kettle,
	}))

	automation.device_manager:add(IkeaRemote.new({
		name = "Remote",
		room = "Kitchen",
		client = mqtt_client,
		topic = h.mqtt_z2m("kitchen/remote"),
		single_button = true,
		callback = set_kettle,
	}))
end
