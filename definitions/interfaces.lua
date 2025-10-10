--- @meta

---@class InterfaceDevice
local InterfaceDevice
---@return string
function InterfaceDevice:get_id() end

---@class InterfaceOnOff: InterfaceDevice
local InterfaceOnOff
---@async
---@param on boolean
function InterfaceOnOff:set_on(on) end
---@async
---@return boolean
function InterfaceOnOff:on() end

---@class InterfaceBrightness: InterfaceDevice
local InterfaceBrightness
---@async
---@param brightness integer
function InterfaceBrightness:set_brightness(brightness) end
---@async
---@return integer
function InterfaceBrightness:brightness() end

---@class InterfaceColorSetting: InterfaceDevice
local InterfaceColorSetting
---@async
---@param temperature integer
function InterfaceColorSetting:set_color_temperature(temperature) end
---@async
---@return integer
function InterfaceColorSetting:color_temperature() end

---@class InterfaceOpenClose: InterfaceDevice
local InterfaceOpenClose
---@async
---@param open_percent integer
function InterfaceOpenClose:set_open_percent(open_percent) end
---@async
---@return integer
function InterfaceOpenClose:open_percent() end
