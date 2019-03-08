use crate::event_priority;
use cortexm::events;
use cortexm4::{
    generic_isr, hard_fault_handler, set_privileged_thread, stash_process_state, svc_handler,
    systick_handler,
};

macro_rules! generic_isr {
    ($label:tt, $priority:expr) => {
        #[cfg(target_os = "none")]
        #[naked]
        unsafe extern "C" fn $label() {
            stash_process_state();
            events::set_event_flag_from_isr($priority as usize);
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
            events::set_event_flag_from_isr($priority);
            $isr();
            set_privileged_thread();
        }
    };
}

unsafe extern "C" fn unhandled_interrupt() {
    'loop0: loop {}
}

generic_isr!(uart0_nvic, event_priority::EVENT_PRIORITY::UART0);
generic_isr!(osc_isr, event_priority::EVENT_PRIORITY::OSC);

#[link_section = ".vectors"]
// used Ensures that the symbol is kept until the final binary
#[used]
pub static BASE_VECTORS: [unsafe extern "C" fn(); 54] = [
    cc26x2::crt1::_estack,
    cc26x2::crt1::reset_handler,
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
    generic_isr,         // GPIO Int handler
    generic_isr,         // I2C
    generic_isr,         // RF Core Command & Packet Engine 1
    generic_isr,         // AON SpiSplave Rx, Tx and CS
    generic_isr,         // AON RTC
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
    generic_isr, // TRNG event
    osc_isr,
    generic_isr,
    generic_isr, //uart1
    generic_isr,
];