use cortexm4::{
    disable_specific_nvic, generic_isr, hard_fault_handler, nvic, set_privileged_thread,
    stash_process_state, svc_handler, systick_handler,
};

use crate::events::set_event_flag_from_isr;
use tock_rt0;

extern "C" {
    // Symbols defined in the linker file
    static mut _erelocate: u32;
    static mut _etext: u32;
    static mut _ezero: u32;
    static mut _srelocate: u32;
    static mut _szero: u32;
    fn reset_handler();

    // _estack is not really a function, but it makes the types work
    // You should never actually invoke it!!
    fn _estack();
}

use crate::event_priority;
macro_rules! generic_isr {
    ($label:tt, $priority:expr) => {
        #[cfg(target_os = "none")]
        #[naked]
        unsafe extern "C" fn $label() {
            stash_process_state();
            set_event_flag_from_isr($priority);
            disable_specific_nvic();
            set_privileged_thread();
        }
    };
}

macro_rules! custom_isr {
    ($label:tt, $priority:expr, $isr:ident) => {
        #[cfg(target_os = "none")]
        #[naked]
        unsafe extern "C" fn $label() {
            stash_process_state();
            set_event_flag_from_isr($priority);
            $isr();
            set_privileged_thread();
        }
    };
}

generic_isr!(gpio_nvic, event_priority::EVENT_PRIORITY::GPIO);
generic_isr!(i2c0_nvic, event_priority::EVENT_PRIORITY::I2C0);
generic_isr!(aon_rtc_nvic, event_priority::EVENT_PRIORITY::AON_RTC);

use crate::uart::{uart0_isr, uart1_isr};
custom_isr!(uart0_nvic, event_priority::EVENT_PRIORITY::UART0, uart0_isr);
custom_isr!(uart1_nvic, event_priority::EVENT_PRIORITY::UART1, uart1_isr);

unsafe extern "C" fn unhandled_interrupt() {
    'loop0: loop {}
}

#[link_section = ".vectors"]
// used Ensures that the symbol is kept until the final binary
#[used]
pub static BASE_VECTORS: [unsafe extern "C" fn(); 54] = [
    _estack,
    reset_handler,
    unhandled_interrupt, // NMI
    hard_fault_handler,  // Hard Fault
    unhandled_interrupt, // MPU fault
    unhandled_interrupt, // Bus fault
    unhandled_interrupt, // Usage fault
    unhandled_interrupt, // Reserved
    unhandled_interrupt, // Reserved
    unhandled_interrupt, // Reserved
    unhandled_interrupt, // Reserved
    svc_handler,         // SVC
    unhandled_interrupt, // Debug monitor,
    unhandled_interrupt, // Reserved
    unhandled_interrupt, // PendSV
    systick_handler,     // Systick
    gpio_nvic,           // GPIO Int handler
    i2c0_nvic,           // I2C0
    generic_isr,         // RF Core Command & Packet Engine 1
    generic_isr,         // AON SpiSplave Rx, Tx and CS
    aon_rtc_nvic,        // AON RTC
    uart0_nvic,          // UART0 Rx and Tx
    generic_isr,         // AUX software event 0
    generic_isr,         // SSI0 Rx and Tx
    generic_isr,         // SSI1 Rx and Tx
    generic_isr,         // RF Core Command & Packet Engine 0
    generic_isr,         // RF Core Hardware
    generic_isr,         // RF Core Command Acknowledge
    generic_isr,         // I2S
    generic_isr,         // AUX software event 1
    generic_isr,         // Watchdog timer
    generic_isr,         // Timer 0 subtimer A
    generic_isr,         // Timer 0 subtimer B
    generic_isr,         // Timer 1 subtimer A
    generic_isr,         // Timer 1 subtimer B
    generic_isr,         // Timer 2 subtimer A
    generic_isr,         // Timer 2 subtimer B
    generic_isr,         // Timer 3 subtimer A
    generic_isr,         // Timer 3 subtimer B
    generic_isr,         // Crypto Core Result available
    generic_isr,         // uDMA Software
    generic_isr,         // uDMA Error
    generic_isr,         // Flash controller
    generic_isr,         // Software Event 0
    generic_isr,         // AUX combined event
    generic_isr,         // AON programmable 0
    generic_isr,         // Dynamic Programmable interrupt
    // source (Default: PRCM)
    generic_isr, // AUX Comparator A
    generic_isr, // AUX ADC new sample or ADC DMA
    // done, ADC underflow, ADC overflow
    generic_isr, // TRNG event (hw_ints.h 49)
    generic_isr,
    generic_isr,
    uart1_nvic, //uart1_generic_isr,//uart::uart1_isr, // 52 allegedly UART1 (http://e2e.ti.com/support/wireless_connectivity/proprietary_sub_1_ghz_simpliciti/f/156/t/662981?CC1312R-UART1-can-t-work-correctly-in-sensor-oad-cc1312lp-example-on-both-cc1312-launchpad-and-cc1352-launchpad)
    generic_isr,
];

#[no_mangle]
pub unsafe extern "C" fn init() {
    tock_rt0::init_data(&mut _etext, &mut _srelocate, &mut _erelocate);
    tock_rt0::zero_bss(&mut _szero, &mut _ezero);
    nvic::enable_all();
}
