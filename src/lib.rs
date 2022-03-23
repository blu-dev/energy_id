#![feature(proc_macro_hygiene)]
#![feature(repr_simd)]
#![feature(simd_ffi)]
#![feature(asm)]
mod control;
mod energy;
mod motion;

use smash::{
    app::{
        *,
        lua_bind::*
    },
    lib::{
        *,
        lua_const::*
    },
    lua2cpp::*,
    phx::*
};

static mut SHOULD_RUN: bool = false;

#[cfg(not(feature = "dev-plugin"))]
#[no_mangle]
pub extern "Rust" fn set_should_run(should: bool) {
    unsafe {
        SHOULD_RUN = should;
    }
}

#[cfg(feature = "dev-plugin")]
#[smashline::installer]
pub fn install() {
    extern "Rust" {
        fn set_should_run(should: bool);
    }
    unsafe {
        set_should_run(true);
    }
}

#[cfg(feature = "dev-plugin")]
#[smashline::uninstaller]
pub fn uninstall() {
    extern "Rust" {
        fn set_should_run(should: bool);
    }
    unsafe {
        set_should_run(false);
    }
}

#[skyline::main(name = "energy_id")]
pub fn main() {
    #[cfg(not(feature = "dev-plugin"))]
    {
        control::install();
        motion::install();
    }
}
