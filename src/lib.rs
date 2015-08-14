#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_upper_case_globals)]

#[macro_use]
pub mod gb_util;
pub mod gb_memory;
pub mod gb_cpu;
pub mod gb_lcd;
pub mod gb_joypad;

#[cfg(test)]
mod tests;

