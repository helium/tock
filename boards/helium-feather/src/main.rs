#![no_std]
#![no_main]
#![feature(lang_items, asm, naked_functions)]

extern crate capsules;
extern crate cc26x2;
extern crate cortexm;
extern crate cortexm4;
extern crate enum_primitive;
extern crate fixedvec;

use cc26x2::aon;
use cc26x2::prcm;
use cortexm::events;

#[allow(unused_imports)]
use kernel::{create_capability, debug, debug_gpio, static_init};

use capsules::helium;
use capsules::helium::{device::Device, virtual_rfcore::RFCore};
use capsules::uart;
use cc26x2::adc;
use cc26x2::osc;
use cc26x2::radio;

use kernel::capabilities;
use kernel::common::cells::TakeCell;
use kernel::hil;
use kernel::hil::entropy::Entropy32;
use kernel::hil::gpio::InterruptMode;
use kernel::hil::gpio::Pin;
use kernel::hil::gpio::PinCtl;
use kernel::hil::i2c::I2CMaster;
use kernel::hil::rfcore::PaType;
use kernel::hil::rng::Rng;

#[macro_use]
pub mod io;

#[allow(dead_code)]
mod ccfg_test;
#[allow(dead_code)]
mod event_priority;
#[allow(dead_code)]
mod i2c_tests;
#[allow(unused_macros)]
mod interrupt_table;

// High frequency oscillator speed
pub const HFREQ: u32 = 48 * 1_000_000;

// How should the kernel respond when a process faults.
const FAULT_RESPONSE: kernel::procs::FaultResponse = kernel::procs::FaultResponse::Panic;

// Number of concurrent processes this platform supports.
const NUM_PROCS: usize = 3;
static mut PROCESSES: [Option<&'static kernel::procs::ProcessType>; NUM_PROCS] = [None, None, None];

#[link_section = ".app_memory"]
// Give half of RAM to be dedicated APP memory
static mut APP_MEMORY: [u8; 0x10000] = [0; 0x10000];

/// Dummy buffer that causes the linker to reserve enough space for the stack.
#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x1000] = [0; 0x1000];

pub struct FeatherPlatform<'a> {
    uart: &'a capsules::uart::UartDriver<'a>,
    debug_client: &'a debug::DebugClient<'a>,
    led: &'static capsules::led::LED<'static, cc26x2::gpio::GPIOPin>,
    button: &'static capsules::button::Button<'static, cc26x2::gpio::GPIOPin>,
    alarm: &'static capsules::alarm::AlarmDriver<
        'static,
        capsules::virtual_alarm::VirtualMuxAlarm<'static, cc26x2::rtc::Rtc>,
    >,
    rng: &'static capsules::rng::RngDriver<'static>,
    i2c_master: &'static capsules::i2c_master::I2CMasterDriver<cc26x2::i2c::I2CMaster<'static>>,
    helium: &'static capsules::helium::driver::Helium<'static>,
}

impl<'a> kernel::Platform for FeatherPlatform<'a> {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
    where
        F: FnOnce(Option<&kernel::Driver>) -> R,
    {
        match driver_num {
            capsules::uart::DRIVER_NUM => f(Some(self.uart)),
            capsules::led::DRIVER_NUM => f(Some(self.led)),
            capsules::button::DRIVER_NUM => f(Some(self.button)),
            capsules::alarm::DRIVER_NUM => f(Some(self.alarm)),
            capsules::rng::DRIVER_NUM => f(Some(self.rng)),
            capsules::i2c_master::DRIVER_NUM => f(Some(self.i2c_master)),
            capsules::helium::driver::DRIVER_NUM => f(Some(self.helium)),
            _ => f(None),
        }
    }

    fn has_pending_events(&mut self) -> bool {
        events::has_event()
    }

    fn service_pending_events(&mut self) {
        let pending_event: Option<event_priority::EVENT_PRIORITY> = events::next_pending();
        while let Some(event) = pending_event {
            events::clear_event_flag(event);
            match event {
                event_priority::EVENT_PRIORITY::GPIO => {} //unsafe {cc26x2::gpio::PORT.handle_events()},
                event_priority::EVENT_PRIORITY::AON_RTC => {} //unsafe {cc26x2::rtc::RTC.handle_events()},
                event_priority::EVENT_PRIORITY::I2C0 => {} //unsafe {cc26x2::i2c::I2C0.handle_events()},
                event_priority::EVENT_PRIORITY::UART0 => {
                    // pass data from static debug writer to the stack allocated debug uart client
                    unsafe {
                        self.debug_client
                            .with_buffer(|buf| debug::get_debug_writer().publish_str(buf));
                    }
                    let clients = [self.debug_client as &kernel::hil::uart::Client];
                    capsules::uart::handle_irq(0, self.uart, Some(&clients));
                }
                event_priority::EVENT_PRIORITY::UART1 => {
                    //capsules::uart::handle_irq(1, self.uart, None);
                }
                event_priority::EVENT_PRIORITY::RF_CMD_ACK => unsafe{ cc26x2::radio::RFC.handle_ack_event()},
                event_priority::EVENT_PRIORITY::RF_CORE_CPE0 => unsafe{ cc26x2::radio::RFC.handle_cpe0_event()},
                event_priority::EVENT_PRIORITY::RF_CORE_CPE1 => unsafe{ cc26x2::radio::RFC.handle_cpe1_event()},
                event_priority::EVENT_PRIORITY::RF_CORE_HW => panic!("Unhandled RFC interupt event!"),
                //event_priority::EVENT_PRIORITY::AUX_ADC => cc26x2::adc::ADC.handle_events(),
                //event_priority::EVENT_PRIORITY::OSC => cc26x2::prcm::handle_osc_interrupt(),
                event_priority::EVENT_PRIORITY::AON_PROG => (),
                _ => panic!("unhandled event {:?} ", event),
            }
        }
    }
}

static mut HELIUM_BUF: [u8; 240] = [0x00; 240];

mod cc1352p;

pub struct Pinmap {
    uart0_rx: usize,
    uart0_tx: usize,
    uart1_rx: usize,
    uart1_tx: usize,
    i2c0_scl: usize,
    i2c0_sda: usize,
    red_led: usize,
    green_led: usize,
    button1: usize,
    button2: usize,
    regulator_mode: usize,
    skyworks_csd: usize,
    skyworks_cps: usize,
    skyworks_ctx: usize,
    rf_2_4: Option<usize>,
    rf_subg: Option<usize>,
    rf_high_pa: Option<usize>,
}

unsafe fn configure_pins(pin: &Pinmap) {
    cc26x2::gpio::PORT[pin.uart0_rx].enable_uart0_rx();
    cc26x2::gpio::PORT[pin.uart0_tx].enable_uart0_tx();

    cc26x2::gpio::PORT[pin.uart1_rx].enable_uart1_rx();
    cc26x2::gpio::PORT[pin.uart1_tx].enable_uart1_tx();

    cc26x2::gpio::PORT[pin.i2c0_scl].enable_i2c_scl();
    cc26x2::gpio::PORT[pin.i2c0_sda].enable_i2c_sda();

    cc26x2::gpio::PORT[pin.red_led].enable_gpio();
    cc26x2::gpio::PORT[pin.green_led].enable_gpio();

    cc26x2::gpio::PORT[pin.button1].enable_gpio();
    cc26x2::gpio::PORT[pin.button2].enable_gpio();

    // cc26x2::gpio::PORT[pin.regulator_mode].enable_gpio();
    // cc26x2::gpio::PORT[pin.regulator_mode].make_output();
    // cc26x2::gpio::PORT[pin.regulator_mode].set();

    cc26x2::gpio::PORT[pin.skyworks_csd].enable_gpio();
    cc26x2::gpio::PORT[pin.skyworks_cps].enable_gpio();
    cc26x2::gpio::PORT[pin.skyworks_ctx].enable_gpio();

    if let Some(rf_2_4) = pin.rf_2_4 {
        cc26x2::gpio::PORT[rf_2_4].enable_24ghz_output();
    }
    if let Some(rf_high_pa) = pin.rf_high_pa {
        cc26x2::gpio::PORT[rf_high_pa].enable_pa_output();
    }
    if let Some(rf_subg) = pin.rf_subg {
        cc26x2::gpio::PORT[rf_subg].enable_subg_output();
    }
}

use kernel::hil::rf_frontend::SE2435L;


#[no_mangle]
pub unsafe fn reset_handler() {
    cc26x2::init();

    // Create capabilities that the board needs to call certain protected kernel
    // functions.
    let process_management_capability =
        create_capability!(capabilities::ProcessManagementCapability);
    let main_loop_capability = create_capability!(capabilities::MainLoopCapability);
    let memory_allocation_capability = create_capability!(capabilities::MemoryAllocationCapability);

    // Setup AON event defaults
    aon::AON.setup();

    // Power on peripherals (eg. GPIO)
    prcm::Power::enable_domain(prcm::PowerDomain::Peripherals);

    // Wait for it to turn on until we continue
    while !prcm::Power::is_enabled(prcm::PowerDomain::Peripherals) {}

    // Power on Serial domain
    prcm::Power::enable_domain(prcm::PowerDomain::Serial);

    while !prcm::Power::is_enabled(prcm::PowerDomain::Serial) {}

    osc::OSC.request_switch_to_hf_xosc();
    osc::OSC.switch_to_hf_xosc();

    let board_kernel = static_init!(kernel::Kernel, kernel::Kernel::new(&PROCESSES));

    // Enable the GPIO clocks
    prcm::Clock::enable_gpio();

    let pinmap: &Pinmap;

    pinmap = &cc1352p::PINMAP;
    configure_pins(pinmap);

    // LEDs
    let led_pins = static_init!(
        [(
            &'static cc26x2::gpio::GPIOPin,
            capsules::led::ActivationMode
        ); 2],
        [
            (
                &cc26x2::gpio::PORT[pinmap.red_led],
                capsules::led::ActivationMode::ActiveHigh
            ), // Red
            (
                &cc26x2::gpio::PORT[pinmap.green_led],
                capsules::led::ActivationMode::ActiveHigh
            ), // Green
        ]
    );
    let led = static_init!(
        capsules::led::LED<'static, cc26x2::gpio::GPIOPin>,
        capsules::led::LED::new(led_pins)
    );

    // BUTTONS
    let button_pins = static_init!(
        [(&'static cc26x2::gpio::GPIOPin, capsules::button::GpioMode); 2],
        [
            (
                &cc26x2::gpio::PORT[pinmap.button1],
                capsules::button::GpioMode::LowWhenPressed
            ), // Button 1
            (
                &cc26x2::gpio::PORT[pinmap.button2],
                capsules::button::GpioMode::LowWhenPressed
            ), // Button 2
        ]
    );
    let button = static_init!(
        capsules::button::Button<'static, cc26x2::gpio::GPIOPin>,
        capsules::button::Button::new(
            button_pins,
            board_kernel.create_grant(&memory_allocation_capability)
        )
    );

    let mut count = 0;
    for &(btn, _) in button_pins.iter() {
        btn.set_input_mode(hil::gpio::InputMode::PullUp);
        btn.enable_interrupt(count, InterruptMode::FallingEdge);
        btn.set_client(button);
        count += 1;
    }

    // UART
    // setup static debug writer
    let debug_writer = static_init!(
        kernel::debug::DebugWriter,
        kernel::debug::DebugWriter::new(&mut kernel::debug::BUF)
    );
    kernel::debug::set_debug_writer(debug_writer);
    // setup uart client for debug on stack
    let mut debug_client_space = debug::DebugClient::space();
    let debug_client = debug::DebugClient::new_with_default_space(&mut debug_client_space);

    // UART
    let uart0_hil = cc26x2::uart::UART::new(cc26x2::uart::PeripheralNum::_0);
    let mut uart0_driver_app_space = uart::AppRequestsInProgress::space();

    // for each client for the driver, provide an empty TakeCell
    let uart0_clients: [TakeCell<hil::uart::RxRequest>; 3] =
        [TakeCell::empty(), TakeCell::empty(), TakeCell::empty()];

    let uart1_hil = cc26x2::uart::UART::new(cc26x2::uart::PeripheralNum::_1);
    let mut uart1_driver_app_space = uart::AppRequestsInProgress::space();

    let board_uarts = [
        &uart::Uart::new(
            &uart0_hil,
            Some(&uart0_clients),
            uart::AppRequestsInProgress::new_with_default_space(&mut uart0_driver_app_space),
            board_kernel.create_grant(&memory_allocation_capability),
        ),
        &uart::Uart::new(
            &uart1_hil,
            None,
            uart::AppRequestsInProgress::new_with_default_space(&mut uart1_driver_app_space),
            board_kernel.create_grant(&memory_allocation_capability),
        ),
    ];

    let uart_driver = uart::UartDriver::new(&board_uarts);

    cc26x2::i2c::I2C0.initialize();

    let i2c_master = static_init!(
        capsules::i2c_master::I2CMasterDriver<cc26x2::i2c::I2CMaster<'static>>,
        capsules::i2c_master::I2CMasterDriver::new(
            &cc26x2::i2c::I2C0,
            &mut capsules::i2c_master::BUF,
            board_kernel.create_grant(&memory_allocation_capability)
        )
    );

    cc26x2::i2c::I2C0.set_client(i2c_master);
    cc26x2::i2c::I2C0.enable();

    let rtc = &cc26x2::rtc::RTC;
    rtc.start();

    let mux_alarm = static_init!(
        capsules::virtual_alarm::MuxAlarm<'static, cc26x2::rtc::Rtc>,
        capsules::virtual_alarm::MuxAlarm::new(&cc26x2::rtc::RTC)
    );
    rtc.set_client(mux_alarm);

    let virtual_alarm1 = static_init!(
        capsules::virtual_alarm::VirtualMuxAlarm<'static, cc26x2::rtc::Rtc>,
        capsules::virtual_alarm::VirtualMuxAlarm::new(mux_alarm)
    );
    let alarm = static_init!(
        capsules::alarm::AlarmDriver<
            'static,
            capsules::virtual_alarm::VirtualMuxAlarm<'static, cc26x2::rtc::Rtc>,
        >,
        capsules::alarm::AlarmDriver::new(
            virtual_alarm1,
            board_kernel.create_grant(&memory_allocation_capability)
        )
    );
    virtual_alarm1.set_client(alarm);

    let entropy_to_random = static_init!(
        capsules::rng::Entropy32ToRandom<'static>,
        capsules::rng::Entropy32ToRandom::new(&cc26x2::trng::TRNG)
    );
    let rng = static_init!(
        capsules::rng::RngDriver<'static>,
        capsules::rng::RngDriver::new(
            entropy_to_random,
            board_kernel.create_grant(&memory_allocation_capability)
        )
    );
    cc26x2::trng::TRNG.set_client(entropy_to_random);
    entropy_to_random.set_client(rng);

    let sky = static_init!(
        capsules::skyworks_se2435l_r::Sky2435L<'static, cc26x2::gpio::GPIOPin>,
        capsules::skyworks_se2435l_r::Sky2435L::new(
            &cc26x2::gpio::PORT[pinmap.skyworks_csd],
            &cc26x2::gpio::PORT[pinmap.skyworks_cps],
            &cc26x2::gpio::PORT[pinmap.skyworks_ctx],
        )
    );

    // Set underlying radio client to the radio mode wrapper
    radio::RFC.set_client(&radio::MULTIMODE_RADIO);

    let radio = static_init!(
        helium::virtual_rfcore::VirtualRadio<'static, cc26x2::radio::multimode::Radio>,
        helium::virtual_rfcore::VirtualRadio::new(&cc26x2::radio::MULTIMODE_RADIO)
    );
    //Set PA option in radio based on board
    &cc26x2::radio::MULTIMODE_RADIO.pa_type.set(PaType::Skyworks);

    // Set mode client in hil
    kernel::hil::rfcore::RadioDriver::set_transmit_client(&radio::MULTIMODE_RADIO, radio);
    kernel::hil::rfcore::RadioDriver::set_receive_client(
        &radio::MULTIMODE_RADIO,
        radio,
        &mut HELIUM_BUF,
    );
    kernel::hil::rfcore::RadioDriver::set_power_client(&radio::MULTIMODE_RADIO, radio);
    kernel::hil::rfcore::RadioDriver::set_rf_frontend_client(&radio::MULTIMODE_RADIO, sky);

    // Virtual device that will respond to callbacks from the underlying radio and library
    // operations
    let virtual_device = static_init!(
        helium::framer::Framer<
            'static,
            helium::virtual_rfcore::VirtualRadio<'static, cc26x2::radio::multimode::Radio>,
        >,
        helium::framer::Framer::new(radio)
    );
    // Set client for underlying radio as virtual device
    radio.set_transmit_client(virtual_device);
    radio.set_receive_client(virtual_device);

    // Driver for user to interface with
    let radio_driver = static_init!(
        helium::driver::Helium<'static>,
        helium::driver::Helium::new(
            board_kernel.create_grant(&memory_allocation_capability),
            &mut HELIUM_BUF,
            virtual_device
        )
    );

    virtual_device.set_transmit_client(radio_driver);
    virtual_device.set_receive_client(radio_driver);

    let rfc = &cc26x2::radio::MULTIMODE_RADIO;
    //rfc.run_tests(0);

    let ipc = kernel::ipc::IPC::new(board_kernel, &memory_allocation_capability);

    let mut feather = FeatherPlatform {
        uart: &uart_driver,
        debug_client: &debug_client,
        led,
        button,
        alarm,
        rng,
        i2c_master,
        helium: radio_driver,
    };

    let chip = static_init!(cc26x2::chip::Cc26X2, cc26x2::chip::Cc26X2::new(HFREQ));

    extern "C" {
        /// Beginning of the ROM region containing app images.
        static _sapps: u8;
    }

    events::set_event_flag(event_priority::EVENT_PRIORITY::UART0);


    debug!("Loading processes");

    kernel::procs::load_processes(
        board_kernel,
        chip,
        &_sapps as *const u8,
        &mut APP_MEMORY,
        &mut PROCESSES,
        FAULT_RESPONSE,
        &process_management_capability,
    );

    board_kernel.kernel_loop(&mut feather, chip, Some(&ipc), &main_loop_capability);
}
