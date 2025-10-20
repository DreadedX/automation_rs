local utils = require("automation:utils")
local secrets = require("automation:secrets")

local host = utils.get_hostname()
print("Lua " .. _VERSION .. " running on " .. utils.get_hostname())

---@type Config
return {
	fulfillment = {
		openid_url = "https://login.huizinga.dev/api/oidc",
	},
	mqtt = {
		host = ((host == "zeus" or host == "hephaestus") and "olympus.lan.huizinga.dev") or "mosquitto",
		port = 8883,
		client_name = "automation-" .. host,
		username = "mqtt",
		password = secrets.mqtt_password,
		tls = host == "zeus" or host == "hephaestus",
	},
	modules = {
		require("config.battery"),
		require("config.debug"),
		require("config.hallway_automation"),
		require("config.helper"),
		require("config.hue_bridge"),
		require("config.light"),
		require("config.ntfy"),
		require("config.presence"),
		require("config.rooms"),
		require("config.windows"),
	},
	-- TODO: Make this also part of the modules
	schedule = {
		["0 0 19 * * *"] = function()
			require("config.rooms.bedroom").set_airfilter_on(true)
		end,
		["0 0 20 * * *"] = function()
			require("config.rooms.bedroom").set_airfilter_on(false)
		end,
		["0 0 21 */1 * *"] = require("config.battery").notify_low_battery,
	},
}
