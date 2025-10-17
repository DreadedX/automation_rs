-- DO NOT MODIFY, FILE IS AUTOMATICALLY GENERATED
---@meta

local devices

---@class Action
---@field action
---| "broadcast"
---@field extras table<string, string>?
---@field label string
---@field clear boolean?
local Action

---@class AirFilter: DeviceInterface, OnOffInterface
local AirFilter
devices.AirFilter = {}
---@param config AirFilterConfig
---@return AirFilter
function devices.AirFilter.new(config) end

---@class AirFilterConfig
---@field name string
---@field room string?
---@field url string
local AirFilterConfig

---@class ConfigLightLightStateBrightness
---@field name string
---@field room string?
---@field topic string
---@field callback fun(_: LightBrightness, _: LightStateBrightness) | fun(_: LightBrightness, _: LightStateBrightness)[]?
---@field client AsyncClient?
local ConfigLightLightStateBrightness

---@class ConfigLightLightStateColorTemperature
---@field name string
---@field room string?
---@field topic string
---@field callback fun(_: LightColorTemperature, _: LightStateColorTemperature) | fun(_: LightColorTemperature, _: LightStateColorTemperature)[]?
---@field client AsyncClient?
local ConfigLightLightStateColorTemperature

---@class ConfigLightLightStateOnOff
---@field name string
---@field room string?
---@field topic string
---@field callback fun(_: LightOnOff, _: LightStateOnOff) | fun(_: LightOnOff, _: LightStateOnOff)[]?
---@field client AsyncClient?
local ConfigLightLightStateOnOff

---@class ConfigOutletOutletStateOnOff
---@field name string
---@field room string?
---@field topic string
---@field outlet_type OutletType?
---@field callback fun(_: OutletOnOff, _: OutletStateOnOff) | fun(_: OutletOnOff, _: OutletStateOnOff)[]?
---@field client AsyncClient
local ConfigOutletOutletStateOnOff

---@class ConfigOutletOutletStatePower
---@field name string
---@field room string?
---@field topic string
---@field outlet_type OutletType?
---@field callback fun(_: OutletPower, _: OutletStatePower) | fun(_: OutletPower, _: OutletStatePower)[]?
---@field client AsyncClient
local ConfigOutletOutletStatePower

---@class ContactSensor: DeviceInterface, OpenCloseInterface
local ContactSensor
devices.ContactSensor = {}
---@param config ContactSensorConfig
---@return ContactSensor
function devices.ContactSensor.new(config) end

---@class ContactSensorConfig
---@field name string
---@field room string?
---@field topic string
---@field sensor_type SensorType?
---@field callback fun(_: ContactSensor, _: boolean) | fun(_: ContactSensor, _: boolean)[]?
---@field battery_callback fun(_: ContactSensor, _: number) | fun(_: ContactSensor, _: number)[]?
---@field client AsyncClient?
local ContactSensorConfig

---@alias Flag
---| "presence"
---| "darkness"

---@class FlagIDs
---@field presence integer
---@field darkness integer
local FlagIDs

---@class HueBridge: DeviceInterface
local HueBridge
devices.HueBridge = {}
---@param config HueBridgeConfig
---@return HueBridge
function devices.HueBridge.new(config) end
---@async
---@param flag Flag
---@param value boolean
function HueBridge:set_flag(flag, value) end

---@class HueBridgeConfig
---@field identifier string
---@field ip string
---@field login string
---@field flags FlagIDs
local HueBridgeConfig

---@class HueGroup: DeviceInterface, OnOffInterface
local HueGroup
devices.HueGroup = {}
---@param config HueGroupConfig
---@return HueGroup
function devices.HueGroup.new(config) end

---@class HueGroupConfig
---@field identifier string
---@field ip string
---@field login string
---@field group_id integer
---@field scene_id string
local HueGroupConfig

---@class HueSwitch: DeviceInterface
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

---@class IkeaRemote: DeviceInterface
local IkeaRemote
devices.IkeaRemote = {}
---@param config IkeaRemoteConfig
---@return IkeaRemote
function devices.IkeaRemote.new(config) end

---@class IkeaRemoteConfig
---@field name string
---@field room string?
---@field single_button boolean?
---@field topic string
---@field client AsyncClient
---@field callback fun(_: IkeaRemote, _: boolean) | fun(_: IkeaRemote, _: boolean)[]?
---@field battery_callback fun(_: IkeaRemote, _: number) | fun(_: IkeaRemote, _: number)[]?
local IkeaRemoteConfig

---@class KasaOutlet: DeviceInterface, OnOffInterface
local KasaOutlet
devices.KasaOutlet = {}
---@param config KasaOutletConfig
---@return KasaOutlet
function devices.KasaOutlet.new(config) end

---@class KasaOutletConfig
---@field identifier string
---@field ip string
local KasaOutletConfig

---@class LightBrightness: DeviceInterface, OnOffInterface, BrightnessInterface
local LightBrightness
devices.LightBrightness = {}
---@param config ConfigLightLightStateBrightness
---@return LightBrightness
function devices.LightBrightness.new(config) end

---@class LightColorTemperature: DeviceInterface, OnOffInterface, BrightnessInterface, ColorSettingInterface
local LightColorTemperature
devices.LightColorTemperature = {}
---@param config ConfigLightLightStateColorTemperature
---@return LightColorTemperature
function devices.LightColorTemperature.new(config) end

---@class LightOnOff: DeviceInterface, OnOffInterface
local LightOnOff
devices.LightOnOff = {}
---@param config ConfigLightLightStateOnOff
---@return LightOnOff
function devices.LightOnOff.new(config) end

---@class LightSensor: DeviceInterface
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

---@class LightStateBrightness
---@field state boolean
---@field brightness number
local LightStateBrightness

---@class LightStateColorTemperature
---@field state boolean
---@field brightness number
---@field color_temp integer
local LightStateColorTemperature

---@class LightStateOnOff
---@field state boolean
local LightStateOnOff

---@class Notification
---@field title string
---@field message string?
---@field tags string[]?
---@field priority Priority?
---@field actions Action[]?
local Notification

---@class Ntfy: DeviceInterface
local Ntfy
devices.Ntfy = {}
---@param config NtfyConfig
---@return Ntfy
function devices.Ntfy.new(config) end
---@async
---@param notification Notification
function Ntfy:send_notification(notification) end

---@class NtfyConfig
---@field url string?
---@field topic string
local NtfyConfig

---@class OutletOnOff: DeviceInterface, OnOffInterface
local OutletOnOff
devices.OutletOnOff = {}
---@param config ConfigOutletOutletStateOnOff
---@return OutletOnOff
function devices.OutletOnOff.new(config) end

---@class OutletPower: DeviceInterface, OnOffInterface
local OutletPower
devices.OutletPower = {}
---@param config ConfigOutletOutletStatePower
---@return OutletPower
function devices.OutletPower.new(config) end

---@class OutletStateOnOff
---@field state boolean
local OutletStateOnOff

---@class OutletStatePower
---@field state boolean
---@field power number
local OutletStatePower

---@alias OutletType
---| "Outlet"
---| "Kettle"

---@class Presence: DeviceInterface
local Presence
devices.Presence = {}
---@param config PresenceConfig
---@return Presence
function devices.Presence.new(config) end
---@async
---@return boolean
function Presence:overall_presence() end

---@class PresenceConfig
---@field topic string
---@field callback fun(_: Presence, _: boolean) | fun(_: Presence, _: boolean)[]?
---@field client AsyncClient
local PresenceConfig

---@alias Priority
---| "min"
---| "low"
---| "default"
---| "high"
---| "max"

---@alias SensorType
---| "Door"
---| "Drawer"
---| "Window"

---@class WakeOnLAN: DeviceInterface
local WakeOnLAN
devices.WakeOnLAN = {}
---@param config WolConfig
---@return WakeOnLAN
function devices.WakeOnLAN.new(config) end

---@class Washer: DeviceInterface
local Washer
devices.Washer = {}
---@param config WasherConfig
---@return Washer
function devices.Washer.new(config) end

---@class WasherConfig
---@field identifier string
---@field topic string
---@field threshold number
---@field done_callback fun(_: Washer) | fun(_: Washer)[]?
---@field client AsyncClient
local WasherConfig

---@class WolConfig
---@field name string
---@field room string?
---@field topic string
---@field mac_address string
---@field broadcast_ip string?
---@field client AsyncClient
local WolConfig

return devices
