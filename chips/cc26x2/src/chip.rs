use adc;
use cortexm4f;
use event_priority::EVENT_PRIORITY;
use events;
use gpio;
use i2c;
use kernel;
use prcm;
use radio;
use rtc;
use uart;

#[repr(C)]
#[derive(Clone, Copy)]
pub enum SleepMode {
    DeepSleep = 0,
    Sleep = 1,
    Active = 2,
}

impl From<u32> for SleepMode {
    fn from(n: u32) -> Self {
        match n {
            0 => SleepMode::DeepSleep,
            1 => SleepMode::Sleep,
            2 => SleepMode::Active,
            _ => unimplemented!(),
        }
    }
}

pub struct Cc26X2 {
    mpu: cortexm4f::mpu::MPU,
    systick: cortexm4f::systick::SysTick,
}

impl Cc26X2 {
    pub unsafe fn new(hfreq: u32) -> Cc26X2 {
        Cc26X2 {
            mpu: cortexm4f::mpu::MPU::new(),
            // The systick clocks with 48MHz by default
            systick: cortexm4f::systick::SysTick::new_with_calibration(hfreq),
        }
    }
}

impl kernel::Chip for Cc26X2 {
    type MPU = cortexm4f::mpu::MPU;
    type SysTick = cortexm4f::systick::SysTick;

    fn mpu(&self) -> &Self::MPU {
        &self.mpu
    }

    fn systick(&self) -> &Self::SysTick {
        &self.systick
    }

    fn service_pending_interrupts(&self) {
        unsafe {
            while let Some(event) = events::next_pending() {
                events::clear_event_flag(event);
                match event {
                    EVENT_PRIORITY::GPIO => gpio::PORT.handle_events(),
                    EVENT_PRIORITY::AON_RTC => rtc::RTC.handle_events(),
                    EVENT_PRIORITY::I2C0 => i2c::I2C0.handle_events(),
                    EVENT_PRIORITY::UART0 => uart::UART0.handle_events(),
                    EVENT_PRIORITY::UART1 => uart::UART1.handle_events(),
                    EVENT_PRIORITY::RF_CMD_ACK => radio::RFC.handle_ack_event(),
                    EVENT_PRIORITY::RF_CORE_CPE0 => radio::RFC.handle_cpe0_event(),
                    EVENT_PRIORITY::RF_CORE_CPE1 => radio::RFC.handle_cpe1_event(),
                    EVENT_PRIORITY::RF_CORE_HW => panic!("Unhandled RFC interupt event!"),
                    EVENT_PRIORITY::AUX_ADC => adc::ADC.handle_events(),
                    EVENT_PRIORITY::OSC => prcm::handle_osc_interrupt(),
                    EVENT_PRIORITY::AON_PROG => (),
                    _ => panic!("unhandled event {:?} ", event),
                }
            }
        }
    }

    fn has_pending_interrupts(&self) -> bool {
        events::has_event()
    }

    fn sleep(&self) {
        unsafe {
            cortexm4f::support::wfi();
        }
    }

    unsafe fn atomic<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        cortexm4f::support::atomic(f)
    }
}
