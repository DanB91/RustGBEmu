#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_upper_case_globals)]


extern crate sdl2_sys;
extern crate sdl2;
extern crate libc;
extern crate errno;

mod sdl2_ttf;

#[macro_use]
pub mod gb_util;
pub mod gb_gameboy;
pub mod gb_memory;
pub mod gb_cpu;
pub mod gb_lcd;
pub mod gb_joypad;
pub mod gb_debug;

#[macro_use]
extern crate bitflags;



#[cfg(test)]
mod tests;

