use std::ops::Deref;

// TODO: Enable and disable functions based on query_only and command_only

pub trait OnOff {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M)
    where
        Self: Sized + google_home::traits::OnOff + 'static,
    {
        methods.add_async_method("set_on", |_lua, this, on: bool| async move {
            this.deref().set_on(on).await.unwrap();

            Ok(())
        });

        methods.add_async_method("on", |_lua, this, ()| async move {
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
        methods.add_async_method("set_brightness", |_lua, this, brightness: u8| async move {
            this.set_brightness(brightness).await.unwrap();

            Ok(())
        });

        methods.add_async_method("brightness", |_lua, this, _: ()| async move {
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
            |_lua, this, temperature: u32| async move {
                this.set_color(google_home::traits::Color { temperature })
                    .await
                    .unwrap();

                Ok(())
            },
        );

        methods.add_async_method("color_temperature", |_lua, this, ()| async move {
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
        methods.add_async_method(
            "set_open_percent",
            |_lua, this, open_percent: u8| async move {
                this.set_open_percent(open_percent).await.unwrap();

                Ok(())
            },
        );

        methods.add_async_method("open_percent", |_lua, this, _: ()| async move {
            Ok(this.open_percent().await.unwrap())
        });
    }
}
impl<T> OpenClose for T where T: google_home::traits::OpenClose {}

pub trait AddAdditionalMethods {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M)
    where
        Self: Sized + 'static;
}
