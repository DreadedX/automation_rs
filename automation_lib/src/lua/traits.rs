use std::ops::Deref;

// TODO: Enable and disable functions based on query_only and command_only

pub trait OnOff {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M)
    where
        Self: Sized + google_home::traits::OnOff + 'static,
    {
        methods.add_async_method("set_on", async |_lua, this, on: bool| {
            this.deref().set_on(on).await.unwrap();

            Ok(())
        });

        methods.add_async_method("on", async |_lua, this, ()| {
            Ok(this.deref().on().await.unwrap())
        });
    }
}
impl<T> OnOff for T where T: google_home::traits::OnOff {}

pub trait Brightness {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M)
    where
        Self: Sized + google_home::traits::Brightness + 'static,
    {
        methods.add_async_method("set_brightness", async |_lua, this, brightness: u8| {
            this.set_brightness(brightness).await.unwrap();

            Ok(())
        });

        methods.add_async_method("brightness", async |_lua, this, _: ()| {
            Ok(this.brightness().await.unwrap())
        });
    }
}
impl<T> Brightness for T where T: google_home::traits::Brightness {}

pub trait ColorSetting {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M)
    where
        Self: Sized + google_home::traits::ColorSetting + 'static,
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
}
impl<T> ColorSetting for T where T: google_home::traits::ColorSetting {}

pub trait OpenClose {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M)
    where
        Self: Sized + google_home::traits::OpenClose + 'static,
    {
        methods.add_async_method("set_open_percent", async |_lua, this, open_percent: u8| {
            this.set_open_percent(open_percent).await.unwrap();

            Ok(())
        });

        methods.add_async_method("open_percent", async |_lua, this, _: ()| {
            Ok(this.open_percent().await.unwrap())
        });
    }
}
impl<T> OpenClose for T where T: google_home::traits::OpenClose {}
