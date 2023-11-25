/*
 * Copyright (c) 2015-2023 Lymia Kanokawa <lymia@lymia.moe>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

#![allow(non_snake_case)]

use crate::init::{MppatchCtx, MppatchFeature};
use anyhow::Result;
use libc::c_int;
use log::{debug, trace};
use mlua::{
    ffi::{
        luaL_checkstring, lua_getfenv, lua_gettable, lua_gettop, lua_insert, lua_isnil, lua_pop,
        lua_pushcfunction, lua_pushstring, lua_settable, lua_type, LUA_REGISTRYINDEX, LUA_TSTRING,
    },
    lua_State,
    prelude::{LuaFunction, LuaResult, LuaString},
    Lua, Table,
};
use std::{ffi::CStr, sync::Mutex};

#[cfg(unix)]
mod platform_impl {
    use super::*;
    use crate::{rt_patch::PatchedFunction, versions::VersionInfoLinux};
    use dlopen::raw::Library;
    use std::ffi::c_void;

    static GET_MEMORY_USAGE: Mutex<Option<PatchedFunction>> = Mutex::new(None);

    pub unsafe fn lGetMemoryUsage(lua: *mut lua_State) -> c_int {
        let func: unsafe extern "C" fn(*mut lua_State) -> c_int = std::mem::transmute(
            GET_MEMORY_USAGE
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .old_function(),
        );
        func(lua)
    }

    pub fn init(ctx: &MppatchCtx) -> Result<()> {
        log::info!("Applying lGetMemoryUsage patch...");

        let linux_info: VersionInfoLinux = ctx.info_linux()?;

        let sym = linux_info.sym_lGetMemoryUsage;
        let sym_size = linux_info.sym_lGetMemoryUsage_len;

        unsafe {
            let dylib_civ = Library::open_self()?;
            let patch = PatchedFunction::create(
                dylib_civ.symbol(sym)?,
                lGetMemoryUsageProxy as usize as *const c_void,
                sym_size,
                "lGetMemoryUsage",
            );
            *GET_MEMORY_USAGE.lock().unwrap() = Some(patch);
        }

        Ok(())
    }

    #[ctor::dtor]
    fn destroy_usage() {
        GET_MEMORY_USAGE.lock().unwrap().take();
    }

    #[cfg(target_os = "macos")]
    pub const PLATFORM: &str = "macos";

    #[cfg(target_os = "linux")]
    pub const PLATFORM: &str = "linux";
}

const LUA_REG_INDEX: &str = "2c11892f-7ad1-4ea1-bc4e-770a86c387e6";
const LUA_SENTINEL: &str = "216f0090-85dd-4061-8371-3d8ba2099a70";

const LUA_TABLE_INDEX: &str = "4f9ef697-7746-45d3-9c2d-f2121464a359";
const LUA_TABLE_INDEX_C: &CStr =
    match CStr::from_bytes_with_nul("4f9ef697-7746-45d3-9c2d-f2121464a359\0".as_bytes()) {
        Ok(x) => x,
        Err(_) => panic!("???"),
    };

const LUA_FUNC_GET_GLOBALS: &str = "7fe157f7-909f-4cbc-9257-8156d1d84a29";
const LUA_FUNC_GET_GLOBALS_C: &CStr =
    match CStr::from_bytes_with_nul("7fe157f7-909f-4cbc-9257-8156d1d84a29\0".as_bytes()) {
        Ok(x) => x,
        Err(_) => panic!("???"),
    };

static CTX: Mutex<Option<MppatchCtx>> = Mutex::new(None);

fn create_mppatch_table(lua_c: *mut lua_State, lua: &Lua) -> Result<()> {
    trace!("Building MPPatch function table...");

    let ctx = CTX.lock().unwrap();
    let ctx = ctx.as_ref().unwrap();

    let patch_table = lua.create_table()?;
    patch_table.set("__mppatch_marker", 1)?;
    patch_table.set("debugPrint", lua.create_function(l_debug_print)?)?;
    patch_table.set("getGlobals", lua.create_function(l_get_globals)?)?;
    patch_table.set("shared", lua.create_table()?)?;

    // version table
    {
        let version_table = lua.create_table()?;
        version_table.set("versionString", ctx.version())?;
        version_table.set("platform", platform_impl::PLATFORM)?;
        version_table.set("sha256", ctx.sha256())?;
        version_table.set("buildId", ctx.build_id())?;
        patch_table.set("version", version_table)?;
    }

    // config table
    {
        let config_table = lua.create_table()?;
        config_table.set("enableLogging", ctx.has_feature(MppatchFeature::Logging))?;
        config_table.set("enableDebug", ctx.has_feature(MppatchFeature::Debug))?;
        config_table.set("enableMultiplayerPatch", ctx.has_feature(MppatchFeature::Multiplayer))?;
        config_table.set("enableLuaJIT", ctx.has_feature(MppatchFeature::LuaJit))?;
        patch_table.set("config", config_table)?;
    }

    // find actual globals table
    let globals = {
        unsafe {
            lua_pushstring(lua_c, LUA_FUNC_GET_GLOBALS_C.as_ptr());
            lua_pushcfunction(lua_c, get_globals_table);
            lua_settable(lua_c, LUA_REGISTRYINDEX);
        }

        let get_globals: LuaFunction = lua.named_registry_value(LUA_FUNC_GET_GLOBALS)?;
        let table: Table = get_globals.call(())?;
        table
    };

    // globals table
    {
        let globals_table = lua.create_table()?;
        globals_table.set("rawget", globals.get::<_, LuaFunction>("rawget")?)?;
        globals_table.set("rawset", globals.get::<_, LuaFunction>("rawset")?)?;
        patch_table.set("globals", globals_table)?;
    }

    // NetPatch table
    {
        let net_patch_table = lua.create_table()?;
        // TODO: NetPatch.pushMod
        // TODO: NetPatch.overrideReloadMods
        // TODO: NetPatch.overrideModList
        // TODO: NetPatch.pushDLC
        // TODO: NetPatch.overrideReloadDLC
        // TODO: NetPatch.overrideDLCList
        // TODO: NetPatch.install
        // TODO: NetPatch.reset
        patch_table.set("NetPatch", net_patch_table)?;
    }

    lua.set_named_registry_value(LUA_TABLE_INDEX, patch_table)?;
    Ok(())
}
fn l_debug_print(_: &Lua, value: LuaString) -> LuaResult<()> {
    debug!("{}", value.to_string_lossy());
    Ok(())
}
fn l_get_globals(lua: &Lua, _: ()) -> LuaResult<Table> {
    Ok(lua.globals())
}

/// this can't be done entire in mlua, unfortunately
unsafe extern "C-unwind" fn get_globals_table(lua_c: *mut lua_State) -> c_int {
    lua_pushstring(lua_c, CStr::from_bytes_with_nul(b"\0").unwrap().as_ptr()); // S
    lua_pushstring(lua_c, CStr::from_bytes_with_nul(b"gsub\0").unwrap().as_ptr()); // S S
    lua_gettable(lua_c, -2);
    lua_getfenv(lua_c, -1);
    lua_insert(lua_c, -3);
    lua_pop(lua_c, 2);
    1
}

pub unsafe extern "C-unwind" fn lGetMemoryUsageProxy(lua_c: *mut lua_State) -> c_int {
    if lua_type(lua_c, 1) == LUA_TSTRING
        && CStr::from_ptr(luaL_checkstring(lua_c, 1)).to_string_lossy() == LUA_SENTINEL
    {
        trace!("Found sentinel value, returning MPPatch function table.");

        lua_pushstring(lua_c, LUA_TABLE_INDEX_C.as_ptr());
        lua_gettable(lua_c, LUA_REGISTRYINDEX);
        if lua_isnil(lua_c, lua_gettop(lua_c)) != 0 {
            lua_pop(lua_c, 1);

            let lua = Lua::init_from_ptr(lua_c);
            create_mppatch_table(lua_c, &lua).unwrap(); // TODO: Error handling
            drop(lua);
        } else {
            lua_pop(lua_c, 1);
        }

        lua_pushstring(lua_c, LUA_TABLE_INDEX_C.as_ptr());
        lua_gettable(lua_c, LUA_REGISTRYINDEX);
        1
    } else {
        platform_impl::lGetMemoryUsage(lua_c)
    }
}

pub fn init(ctx: &MppatchCtx) -> Result<()> {
    *CTX.lock().unwrap() = Some(ctx.clone());
    platform_impl::init(ctx)?;
    Ok(())
}
