#![feature(const_fn, untagged_unions, used, asm, naked_functions)]
#![no_std]
#![crate_name = "cc26x2"]
#![crate_type = "rlib"]
extern crate cortexm;
extern crate cortexm4;
#[allow(unused_imports)]
#[macro_use]
extern crate kernel;
#[macro_use]
extern crate enum_primitive;
#[macro_use]
extern crate bitfield;
extern crate fixedvec;

pub mod adc;
pub mod adi;
pub mod aon;
pub mod aux;
pub mod ccfg;
pub mod chip;
pub mod crt1;
pub mod event;
pub mod event_priority;
pub mod events;
pub mod fcfg1;
pub mod gpio;
pub mod gpt;
pub mod i2c;
pub mod ioc;
pub mod memory_map;
pub mod osc;
pub mod peripheral_interrupts;
pub mod prcm;
pub mod pwm;
pub mod radio;
pub mod rat;
pub mod rom;
pub mod rtc;
pub mod setup;
pub mod trng;
pub mod uart;
pub use crt1::init;
