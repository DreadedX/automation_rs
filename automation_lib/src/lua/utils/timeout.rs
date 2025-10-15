use std::sync::Arc;
use std::time::Duration;

use lua_typed::Typed;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::debug;

use crate::action_callback::ActionCallback;

#[derive(Debug, Default)]
pub struct State {
    handle: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct Timeout {
    state: Arc<RwLock<State>>,
}

impl mlua::UserData for Timeout {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("new", |_lua, ()| {
            let device = Self {
                state: Default::default(),
            };

            Ok(device)
        });

        methods.add_async_method(
            "start",
            async |_lua, this, (timeout, callback): (f32, ActionCallback<()>)| {
                if let Some(handle) = this.state.write().await.handle.take() {
                    handle.abort();
                }

                debug!("Running timeout callback after {timeout}s");

                let timeout = Duration::from_secs_f32(timeout);

                this.state.write().await.handle = Some(tokio::spawn({
                    async move {
                        tokio::time::sleep(timeout).await;

                        callback.call(()).await;
                    }
                }));

                Ok(())
            },
        );

        methods.add_async_method("cancel", async |_lua, this, ()| {
            debug!("Canceling timeout callback");

            if let Some(handle) = this.state.write().await.handle.take() {
                handle.abort();
            }

            Ok(())
        });

        methods.add_async_method("is_waiting", async |_lua, this, ()| {
            debug!("Canceling timeout callback");

            if let Some(handle) = this.state.read().await.handle.as_ref() {
                debug!("Join handle: {}", handle.is_finished());
                return Ok(!handle.is_finished());
            }

            debug!("Join handle: None");

            Ok(false)
        });
    }
}

impl Typed for Timeout {
    fn type_name() -> String {
        "Timeout".into()
    }

    fn generate_header() -> Option<String> {
        let type_name = Self::type_name();
        Some(format!("---@class {type_name}\nlocal {type_name}\n"))
    }

    fn generate_members() -> Option<String> {
        let mut output = String::new();

        let type_name = Self::type_name();

        output += &format!(
            "---@async\n---@param timeout number\n---@param callback {}\nfunction {type_name}:start(timeout, callback) end\n",
            ActionCallback::<()>::type_name()
        );

        output += &format!("---@async\nfunction {type_name}:cancel() end\n",);

        output +=
            &format!("---@async\n---@return boolean\nfunction {type_name}:is_waiting() end\n",);

        Some(output)
    }

    fn generate_footer() -> Option<String> {
        let mut output = String::new();

        let type_name = Self::type_name();

        output += &format!("utils.{type_name} = {{}}\n");
        output += &format!("---@return {type_name}\n");
        output += &format!("function utils.{type_name}.new() end\n");

        Some(output)
    }
}
