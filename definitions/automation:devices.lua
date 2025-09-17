---@meta

local devices

---@class OutletOnOff
local OutletOnOff
---@async
---@param on boolean
function OutletOnOff:set_on(on) end
---@async
---@return boolean
function OutletOnOff:on() end
devices.OutletOnOff = {}
---@param config OutletConfig
---@return OutletOnOff
function devices.OutletOnOff.new(config) end

---@class OutletPower
local OutletPower
---@async
---@param on boolean
function OutletPower:set_on(on) end
---@async
---@return boolean
function OutletPower:on() end
devices.OutletPower = {}
---@param config OutletConfig
---@return OutletPower
function devices.OutletPower.new(config) end

---@class AirFilterConfig
---@field name string
---@field room string?
---@field url string
local AirFilterConfig

---@class AirFilter
local AirFilter
---@async
---@param on boolean
function AirFilter:set_on(on) end
---@async
---@return boolean
function AirFilter:on() end
devices.AirFilter = {}
---@param config AirFilterConfig
---@return AirFilter
function devices.AirFilter.new(config) end

---@class PresenceConfig
---@field topic string
---@field callback fun(_: Presence, _: boolean) | fun(_: Presence, _: boolean)[]?
---@field client AsyncClient
local PresenceConfig

---@class Presence
local Presence
devices.Presence = {}
---@param config PresenceConfig
---@return Presence
function devices.Presence.new(config) end

---@class WolConfig
---@field name string
---@field room string?
---@field topic string
---@field mac_address string
---@field broadcast_ip string?
---@field client AsyncClient
local WolConfig

---@class WakeOnLAN
local WakeOnLAN
devices.WakeOnLAN = {}
---@param config WolConfig
---@return WakeOnLAN
function devices.WakeOnLAN.new(config) end

---@class LightSensor
local LightSensor
devices.LightSensor = {}
---@param config LightSensorConfig
---@return LightSensor
function devices.LightSensor.new(config) end

---@class LightSensorConfig
---@field identifier string
---@field topic string
---@field min integer
---@field max integer
---@field callback fun(_: LightSensor, _: boolean) | fun(_: LightSensor, _: boolean)[]?
---@field client AsyncClient
local LightSensorConfig

---@class ContactSensor
local ContactSensor
---@async
---@param open_percent integer
function ContactSensor:set_open_percent(open_percent) end
---@async
---@return integer
function ContactSensor:open_percent() end
devices.ContactSensor = {}
---@param config ContactSensorConfig
---@return ContactSensor
function devices.ContactSensor.new(config) end

---@alias SensorType
---| "Door"
---| "Drawer"
---| "Window"

---@class ContactSensorConfig
---@field name string
---@field room string?
---@field topic string
---@field sensor_type SensorType?
---@field callback fun(_: ContactSensor, _: boolean) | fun(_: ContactSensor, _: boolean)[]?
---@field battery_callback fun(_: ContactSensor, _: number) | fun(_: ContactSensor, _: number)[]?
---@field client AsyncClient?
local ContactSensorConfig

---@class KasaOutlet
local KasaOutlet
---@async
---@param on boolean
function KasaOutlet:set_on(on) end
---@async
---@return boolean
function KasaOutlet:on() end
devices.KasaOutlet = {}
---@param config KasaOutletConfig
---@return KasaOutlet
function devices.KasaOutlet.new(config) end

---@class KasaOutletConfig
---@field identifier string
---@field ip string
local KasaOutletConfig

---@class LightOnOff
local LightOnOff
---@async
---@param on boolean
function LightOnOff:set_on(on) end
---@async
---@return boolean
function LightOnOff:on() end
devices.LightOnOff = {}
---@param config LightConfig
---@return LightOnOff
function devices.LightOnOff.new(config) end

---@class LightColorTemperature
local LightColorTemperature
---@async
---@param on boolean
function LightColorTemperature:set_on(on) end
---@async
---@return boolean
function LightColorTemperature:on() end
---@async
---@param brightness integer
function LightColorTemperature:set_brightness(brightness) end
---@async
---@return integer
function LightColorTemperature:brightness() end
---@async
---@param temperature integer
function LightColorTemperature:set_color_temperature(temperature) end
---@async
---@return integer
function LightColorTemperature:color_temperature() end
devices.LightColorTemperature = {}
---@param config LightConfig
---@return LightColorTemperature
function devices.LightColorTemperature.new(config) end

---@class LightBrightness
local LightBrightness
---@async
---@param on boolean
function LightBrightness:set_on(on) end
---@async
---@return boolean
function LightBrightness:on() end
---@async
---@param brightness integer
function LightBrightness:set_brightness(brightness) end
---@async
---@return integer
function LightBrightness:brightness() end
devices.LightBrightness = {}
---@param config LightConfig
---@return LightBrightness
function devices.LightBrightness.new(config) end

---@class HueSwitch
local HueSwitch
devices.HueSwitch = {}
---@param config HueSwitchConfig
---@return HueSwitch
function devices.HueSwitch.new(config) end

---@class HueSwitchConfig
---@field name string
---@field room string?
---@field topic string
---@field client AsyncClient
---@field left_callback fun(_: HueSwitch) | fun(_: HueSwitch)[]?
---@field right_callback fun(_: HueSwitch) | fun(_: HueSwitch)[]?
---@field left_hold_callback fun(_: HueSwitch) | fun(_: HueSwitch)[]?
---@field right_hold_callback fun(_: HueSwitch) | fun(_: HueSwitch)[]?
---@field battery_callback fun(_: HueSwitch, _: number) | fun(_: HueSwitch, _: number)[]?
local HueSwitchConfig

---@class HueBridge
local HueBridge
devices.HueBridge = {}
---@param config HueBridgeConfig
---@return HueBridge
function devices.HueBridge.new(config) end

---@class HueBridgeConfig
---@field identifier string
---@field ip string
---@field login string
---@field flags FlagIDs
local HueBridgeConfig

---@class FlagIDs
---@field presence integer
---@field darkness integer
local FlagIDs

---@class IkeaRemoteConfig
---@field name string
---@field room string?
---@field single_button boolean?
---@field topic string
---@field client AsyncClient
---@field callback fun(_: IkeaRemote, _: boolean) | fun(_: IkeaRemote, _: boolean)[]?
---@field battery_callback fun(_: IkeaRemote, _: number) | fun(_: IkeaRemote, _: number)[]?
local IkeaRemoteConfig

---@class IkeaRemote
local IkeaRemote
devices.IkeaRemote = {}
---@param config IkeaRemoteConfig
---@return IkeaRemote
function devices.IkeaRemote.new(config) end

---@alias Priority
---| "min"
---| "low"
---| "default"
---| "high"
---| "max"

---@class Ntfy
local Ntfy
devices.Ntfy = {}
---@param config NtfyConfig
---@return Ntfy
function devices.Ntfy.new(config) end

---@class NtfyConfig
---@field url string?
---@field topic string
local NtfyConfig

---@class Action
---@field action
---| "broadcast"
---@field extras table<string, string>?
---@field label string
---@field clear boolean?
local Action

---@class Notification
---@field title string
---@field message string?
---@field tags string[]?
---@field priority Priority?
---@field actions Action[]?
local Notification

---@class WasherConfig
---@field identifier string
---@field topic string
---@field threshold number
---@field done_callback fun(_: Washer) | fun(_: Washer)[]?
---@field client AsyncClient
local WasherConfig

---@class Washer
local Washer
devices.Washer = {}
---@param config WasherConfig
---@return Washer
function devices.Washer.new(config) end

---@class HueGroupConfig
---@field identifier string
---@field ip string
---@field login string
---@field group_id integer
---@field scene_id string
local HueGroupConfig

---@class HueGroup
local HueGroup
---@async
---@param on boolean
function HueGroup:set_on(on) end
---@async
---@return boolean
function HueGroup:on() end
devices.HueGroup = {}
---@param config HueGroupConfig
---@return HueGroup
function devices.HueGroup.new(config) end

return devices
