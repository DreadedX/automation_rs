use std::marker::PhantomData;
use std::ops::Deref;

use lua_typed::Typed;

// TODO: Enable and disable functions based on query_only and command_only

pub struct OnOff<T> {
    _phantom: PhantomData<T>,
}

impl<T: google_home::traits::OnOff + Typed> OnOff<T> {
    pub fn add_methods<M: mlua::UserDataMethods<T>>(methods: &mut M)
    where
        T: Sized + 'static,
    {
        methods.add_async_method("set_on", async |_lua, this, on: bool| {
            this.deref().set_on(on).await.unwrap();

            Ok(())
        });

        methods.add_async_method("on", async |_lua, this, ()| {
            Ok(this.deref().on().await.unwrap())
        });
    }

    pub fn generate_definitions() -> String {
        let type_name = T::type_name();
        let mut output = String::new();

        output +=
            &format!("---@async\n---@param on boolean\nfunction {type_name}:set_on(on) end\n");
        output += &format!("---@async\n---@return boolean\nfunction {type_name}:on() end\n");

        output
    }
}

pub struct Brightness<T> {
    _phantom: PhantomData<T>,
}

impl<T: google_home::traits::Brightness + Typed> Brightness<T> {
    pub fn add_methods<M: mlua::UserDataMethods<T>>(methods: &mut M)
    where
        T: Sized + 'static,
    {
        methods.add_async_method("set_brightness", async |_lua, this, brightness: u8| {
            this.set_brightness(brightness).await.unwrap();

            Ok(())
        });

        methods.add_async_method("brightness", async |_lua, this, _: ()| {
            Ok(this.brightness().await.unwrap())
        });
    }

    pub fn generate_definitions() -> String {
        let type_name = T::type_name();
        let mut output = String::new();

        output += &format!(
            "---@async\n---@param brightness integer\nfunction {type_name}:set_brightness(brightness) end\n"
        );
        output +=
            &format!("---@async\n---@return integer\nfunction {type_name}:brightness() end\n");

        output
    }
}

pub struct ColorSetting<T> {
    _phantom: PhantomData<T>,
}

impl<T: google_home::traits::ColorSetting + Typed> ColorSetting<T> {
    pub fn add_methods<M: mlua::UserDataMethods<T>>(methods: &mut M)
    where
        T: Sized + 'static,
    {
        methods.add_async_method(
            "set_color_temperature",
            async |_lua, this, temperature: u32| {
                this.set_color(google_home::traits::Color { temperature })
                    .await
                    .unwrap();

                Ok(())
            },
        );

        methods.add_async_method("color_temperature", async |_lua, this, ()| {
            Ok(this.color().await.temperature)
        });
    }

    pub fn generate_definitions() -> String {
        let type_name = T::type_name();
        let mut output = String::new();

        output += &format!(
            "---@async\n---@param temperature integer\nfunction {type_name}:set_color_temperature(temperature) end\n"
        );
        output += &format!(
            "---@async\n---@return integer\nfunction {type_name}:color_temperature() end\n"
        );

        output
    }
}

pub struct OpenClose<T> {
    _phantom: PhantomData<T>,
}

impl<T: google_home::traits::OpenClose + Typed> OpenClose<T> {
    pub fn add_methods<M: mlua::UserDataMethods<T>>(methods: &mut M)
    where
        T: Sized + 'static,
    {
        methods.add_async_method("set_open_percent", async |_lua, this, open_percent: u8| {
            this.set_open_percent(open_percent).await.unwrap();

            Ok(())
        });

        methods.add_async_method("open_percent", async |_lua, this, _: ()| {
            Ok(this.open_percent().await.unwrap())
        });
    }

    pub fn generate_definitions() -> String {
        let type_name = T::type_name();
        let mut output = String::new();

        output += &format!(
            "---@async\n---@param open_percent integer\nfunction {type_name}:set_open_percent(open_percent) end\n"
        );
        output +=
            &format!("---@async\n---@return integer\nfunction {type_name}:open_percent() end\n");

        output
    }
}
