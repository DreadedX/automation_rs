use std::sync::Arc;
use std::time::Duration;

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
            |_lua, this, (timeout, callback): (f32, ActionCallback<mlua::Value, bool>)| async move {
                if let Some(handle) = this.state.write().await.handle.take() {
                    handle.abort();
                }

                debug!("Running timeout callback after {timeout}s");

                let timeout = Duration::from_secs_f32(timeout);

                this.state.write().await.handle = Some(tokio::spawn({
                    async move {
                        tokio::time::sleep(timeout).await;

                        callback.call(&mlua::Nil, &false).await;
                    }
                }));

                Ok(())
            },
        );

        methods.add_async_method("cancel", |_lua, this, ()| async move {
            debug!("Canceling timeout callback");

            if let Some(handle) = this.state.write().await.handle.take() {
                handle.abort();
            }

            Ok(())
        });

        methods.add_async_method("is_waiting", |_lua, this, ()| async move {
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
