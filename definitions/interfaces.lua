--- @meta

---@class DeviceInterface
local DeviceInterface
---@return string
function DeviceInterface:get_id() end

---@class OnOffInterface: DeviceInterface
local OnOffInterface
---@async
---@param on boolean
function OnOffInterface:set_on(on) end
---@async
---@return boolean
function OnOffInterface:on() end

---@class BrightnessInterface: DeviceInterface
local BrightnessInterface
---@async
---@param brightness integer
function BrightnessInterface:set_brightness(brightness) end
---@async
---@return integer
function BrightnessInterface:brightness() end

---@class ColorSettingInterface: DeviceInterface
local ColorSettingInterface
---@async
---@param temperature integer
function ColorSettingInterface:set_color_temperature(temperature) end
---@async
---@return integer
function ColorSettingInterface:color_temperature() end

---@class OpenCloseInterface: DeviceInterface
local OpenCloseInterface
---@async
---@param open_percent integer
function OpenCloseInterface:set_open_percent(open_percent) end
---@async
---@return integer
function OpenCloseInterface:open_percent() end
