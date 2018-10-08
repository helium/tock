#![allow(unused_imports)]
use chip::SleepMode;
use core::cell::Cell;
use fixedvec::FixedVec;
use kernel::common::cells::{OptionalCell, TakeCell};
use kernel::hil::radio_client;
use kernel::ReturnCode;
use osc;
use radio::commands as cmd;
use radio::rfc;

static mut RFPARAMS: [u32; 18] = [
    // Synth: Use 48 MHz crystal as synth clock, enable extra PLL filtering
    0x02400403, // Synth: Set minimum RTRIM to 6
    0x00068793, // Synth: Configure extra PLL filtering
    0x001C8473, // Synth: Configure extra PLL filtering
    0x00088433, // Synth: Set Fref to 4 MHz
    0x000684A3,
    // Synth: Configure faster calibration
    // HW32_ARRAY_OVERRIDE(0x4004,1),
    // Synth: Configure faster calibration
    0x180C0618, // Synth: Configure faster calibration
    0xC00401A1, // Synth: Configure faster calibration
    0x00010101, // Synth: Configure faster calibration
    0xC0040141, 0x00214AD3, // Synth: Configure faster calibration
    0x02980243, // Synth: Decrease synth programming time-out by 90 us from default (0x0298 RAT ticks = 166 us)
    0x0A480583, // Synth: Set loop bandwidth after lock to 20 kHz
    0x7AB80603, // Synth: Set loop bandwidth after lock to 20 kHz
    0x00000623, // Synth: Set loop bandwidth after lock to 20 kHz
    0x00018883, // Rx: Set LNA bias current offset to adjust +1 (default: 0)
    0x000288A3, // Rx: Set RSSI offset to adjust reported RSSI by -2 dB (default: 0)
    0xFFFC08C3, // DC/DC regulator: In Tx with 14 dBm PA setting, use DCDCCTL5[3:0]=0xF (DITHER_EN=1 and IPEAK=7). In Rx, use DCDCCTL5[3:0]=0xC (DITHER_EN=1 and IPEAK=4).
    0xFFFFFFFF,
];

pub struct Radio {
    rfc: &'static rfc::RFCore,
    tx_radio_client: OptionalCell<&'static radio_client::TxClient>,
    rx_radio_client: OptionalCell<&'static radio_client::RxClient>,
    config_radio_client: OptionalCell<&'static radio_client::ConfigClient>,
    schedule_powerdown: Cell<bool>,
    tx_buf: TakeCell<'static, [u8]>,
}

impl Radio {
    pub const fn new(rfc: &'static rfc::RFCore) -> Radio {
        Radio {
            rfc,
            tx_radio_client: OptionalCell::empty(),
            rx_radio_client: OptionalCell::empty(),
            config_radio_client: OptionalCell::empty(),
            schedule_powerdown: Cell::new(false),
            tx_buf: TakeCell::empty(),
        }
    }

    pub fn test_power_up(&self) {
        // osc::OSC.switch_to_rc_osc();

        self.rfc.set_mode(rfc::RfcMode::Common);

        osc::OSC.request_switch_to_hf_xosc();

        self.rfc.enable();

        self.rfc.start_rat_test();

        osc::OSC.switch_to_hf_xosc();

        unsafe {
            let reg_overrides: u32 = RFPARAMS.as_mut_ptr() as u32;
            self.rfc.setup_test(reg_overrides, 0xFFFE)
        }
    }

    pub fn power_up(&self) -> ReturnCode {
        self.rfc.set_mode(rfc::RfcMode::Common);

        osc::OSC.request_switch_to_hf_xosc();

        self.rfc.enable();

        self.rfc.start_rat();

        osc::OSC.switch_to_hf_xosc();

        unsafe {
            let reg_overrides: u32 = RFPARAMS.as_mut_ptr() as u32;
            self.rfc.setup(reg_overrides, 0xFFFE) // No idea what power setting this is
        }

        if self.rfc.check_enabled() {
            ReturnCode::SUCCESS
        } else {
            ReturnCode::FAIL
        }
    }

    pub fn power_down(&self) {
        self.rfc.disable();
    }
}

impl rfc::RFCoreClient for Radio {
    fn command_done(&self) {
        // Map standard callback to a command client.
    }

    fn tx_done(&self) {
        if self.schedule_powerdown.get() {
            self.power_down();
            osc::OSC.switch_to_hf_rcosc();

            self.schedule_powerdown.set(false);
        }

        let buf = self.tx_buf.take();
        self.tx_radio_client
            .take()
            .map(|client| client.transmit_event(buf.unwrap(), ReturnCode::SUCCESS));
    }

    fn rx_ok(&self) {}
}

impl radio_client::Radio for Radio {}

impl radio_client::RadioDriver for Radio {
    fn set_transmit_client(&self, tx_client: &'static radio_client::TxClient) {
        self.tx_radio_client.set(tx_client);
    }

    fn set_receive_client(
        &self,
        rx_client: &'static radio_client::RxClient,
        _rx_buf: &'static mut [u8],
    ) {
        self.rx_radio_client.set(rx_client);
    }

    fn set_receive_buffer(&self, _rx_buf: &'static mut [u8]) {
        // maybe make a rx buf only when needed?
    }

    fn set_config_client(&self, config_client: &'static radio_client::ConfigClient) {
        self.config_radio_client.set(config_client);
    }

    fn transmit(
        &self,
        tx_buf: &'static mut [u8],
        _frame_len: usize,
    ) -> (ReturnCode, Option<&'static mut [u8]>) {
        (ReturnCode::SUCCESS, Some(tx_buf))
    }
}

impl radio_client::RadioConfig for Radio {
    fn initialize(&self) -> ReturnCode {
        self.power_up()
    }

    fn reset(&self) -> ReturnCode {
        self.power_down();
        self.power_up()
    }

    fn stop(&self) -> ReturnCode {
        let cmd_stop = cmd::DirectCommand::new(0x0402, 0);
        let stopped = self.rfc.send_direct(&cmd_stop).is_ok();
        if stopped {
            ReturnCode::SUCCESS
        } else {
            ReturnCode::FAIL
        }
    }

    fn is_on(&self) -> bool {
        self.rfc.check_enabled()
    }

    fn busy(&self) -> bool {
        // Might be an obsolete command here in favor of get_command_status and some logic on the
        // user size to determine if the radio is busy. Not sure what is best to have here but
        // arguing best might be bikeshedding
        let status = self.rfc.status.get();
        match status {
            0x0001 => true,
            0x0002 => true,
            _ => false,
        }
    }

    fn config_commit(&self) {
        // TODO confirm set new config here
    }

    fn get_tx_power(&self) -> u32 {
        // TODO get tx power radio command
        0x00000000
    }

    fn get_radio_status(&self) -> u32 {
        // TODO get power status of radio
        0x00000000
    }

    fn get_command_status(&self) -> (ReturnCode, Option<u32>) {
        // TODO get command status specifics
        let status = self.rfc.status.get();
        match status & 0x0F00 {
            0 => (ReturnCode::SUCCESS, Some(status)),
            4 => (ReturnCode::SUCCESS, Some(status)),
            8 => (ReturnCode::FAIL, Some(status)),
            _ => (ReturnCode::EINVAL, Some(status)),
        }
    }

    fn set_tx_power(&self, power: u16) -> ReturnCode {
        // Send direct command for TX power change
        let command = cmd::DirectCommand::new(0x0010, power);
        if self.rfc.send_direct(&command).is_ok() {
            return ReturnCode::SUCCESS;
        } else {
            return ReturnCode::FAIL;
        }
    }

    fn send_stop_command(&self) -> ReturnCode {
        // Send "Gracefull" stop radio operation direct command
        let command = cmd::DirectCommand::new(0x0402, 0);
        if self.rfc.send_direct(&command).is_ok() {
            return ReturnCode::SUCCESS;
        } else {
            return ReturnCode::FAIL;
        }
    }

    fn send_kill_command(&self) -> ReturnCode {
        // Send immidiate command kill all radio operation commands
        let command = cmd::DirectCommand::new(0x0401, 0);
        if self.rfc.send_direct(&command).is_ok() {
            return ReturnCode::SUCCESS;
        } else {
            return ReturnCode::FAIL;
        }
    }
}
