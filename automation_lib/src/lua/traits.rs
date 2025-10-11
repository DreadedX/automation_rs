use std::ops::Deref;

// TODO: Enable and disable functions based on query_only and command_only

pub trait PartialUserData<T> {
    fn add_methods<M: mlua::UserDataMethods<T>>(methods: &mut M);
}

pub struct Device;

impl<T> PartialUserData<T> for Device
where
    T: crate::device::Device + 'static,
{
    fn add_methods<M: mlua::UserDataMethods<T>>(methods: &mut M) {
        methods.add_async_method("get_id", async |_lua, this, _: ()| Ok(this.get_id()));
    }
}

pub struct OnOff;

impl<T> PartialUserData<T> for OnOff
where
    T: google_home::traits::OnOff + 'static,
{
    fn add_methods<M: mlua::UserDataMethods<T>>(methods: &mut M) {
        methods.add_async_method("set_on", async |_lua, this, on: bool| {
            this.deref().set_on(on).await.unwrap();

            Ok(())
        });

        methods.add_async_method("on", async |_lua, this, ()| {
            Ok(this.deref().on().await.unwrap())
        });
    }
}

pub struct Brightness;

impl<T> PartialUserData<T> for Brightness
where
    T: google_home::traits::Brightness + 'static,
{
    fn add_methods<M: mlua::UserDataMethods<T>>(methods: &mut M) {
        methods.add_async_method("set_brightness", async |_lua, this, brightness: u8| {
            this.set_brightness(brightness).await.unwrap();

            Ok(())
        });

        methods.add_async_method("brightness", async |_lua, this, _: ()| {
            Ok(this.brightness().await.unwrap())
        });
    }
}

pub struct ColorSetting;

impl<T> PartialUserData<T> for ColorSetting
where
    T: google_home::traits::ColorSetting + 'static,
{
    fn add_methods<M: mlua::UserDataMethods<T>>(methods: &mut M) {
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
}

pub struct OpenClose;

impl<T> PartialUserData<T> for OpenClose
where
    T: google_home::traits::OpenClose + 'static,
{
    fn add_methods<M: mlua::UserDataMethods<T>>(methods: &mut M) {
        methods.add_async_method("set_open_percent", async |_lua, this, open_percent: u8| {
            this.set_open_percent(open_percent).await.unwrap();

            Ok(())
        });

        methods.add_async_method("open_percent", async |_lua, this, _: ()| {
            Ok(this.open_percent().await.unwrap())
        });
    }
}
