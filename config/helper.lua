return {
	mqtt_z2m = function(topic)
		return "zigbee2mqtt/" .. topic
	end,

	mqtt_automation = function(topic)
		return "automation/" .. topic
	end,
}
