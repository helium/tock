//! Component for GPIO on the imix board.
//!
//! This provides one Component, GpioComponent, which implements
//! a userspace syscall interface to a subset of the SAM4L GPIO
//! pins provided on the board headers. It provides 5 pins:
//! 31 (P2), 30 (P3), 29 (P4), 28 (P5), 27  (P6), 26 (P7),
//! and 20 (P8).
//!
//! Usage
//! -----
//! ```rust
//! let gpio = GpioComponent::new().finalize();
//! ```

// Author: Philip Levis <pal@cs.stanford.edu>
// Last modified: 6/20/2018

#![allow(dead_code)] // Components are intended to be conditionally included

use capsules::gpio;
use kernel::component::Component;

pub struct GpioComponent {}

impl GpioComponent {
    pub fn new() -> GpioComponent {
        GpioComponent {}
    }
}

impl Component for GpioComponent {
    type Output = &'static gpio::GPIO<'static, sam4l::gpio::GPIOPin>;

    unsafe fn finalize(&mut self) -> Self::Output {
        let gpio_pins = static_init!(
            [&'static sam4l::gpio::GPIOPin; 7],
            [
                &sam4l::gpio::PC[31], // P2
                &sam4l::gpio::PC[30], // P3
                &sam4l::gpio::PC[29], // P4
                &sam4l::gpio::PC[28], // P5
                &sam4l::gpio::PC[27], // P6
                &sam4l::gpio::PC[26], // P7
                &sam4l::gpio::PA[20], // P8
            ]
        );

        let gpio = static_init!(
            gpio::GPIO<'static, sam4l::gpio::GPIOPin>,
            gpio::GPIO::new(gpio_pins)
        );
        for pin in gpio_pins.iter() {
            pin.set_client(gpio);
        }

        gpio
    }
}
