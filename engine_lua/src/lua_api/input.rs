//! User input API for Lua scripting.

use crate::helpers::lua_error_msg;
use crate::input::InputProvider;
use mlua::{Lua, Result as LuaResult, Table};
use std::sync::{Arc, Mutex};

/// Registers the input API.
pub fn register_input_api(
    lua: &Lua,
    globals: &Table,
    input_provider: Arc<Mutex<Box<dyn InputProvider + Send + Sync>>>,
) -> LuaResult<()> {
    // get_user_input(prompt)
    let input_provider_clone = input_provider.clone();
    let get_user_input = lua.create_function(move |lua, prompt: String| {
        let mut provider = input_provider_clone
            .lock()
            .map_err(|_| lua_error_msg(lua, "Input provider lock poisoned"))?;
        provider
            .read_line(&prompt)
            .map_err(|e| lua_error_msg(lua, &format!("Input error: {e}")))
    })?;
    globals.set("get_user_input", get_user_input)?;

    Ok(())
}
