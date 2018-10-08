//! Interface for sending and receiving packets.
//!
//! Hardware independent interface for an arbitrary radio. Note that
//! configuration commands are asynchronous and must be committed with a call to
//! config_commit. For example, calling set_address will change the source
//! address of packets but does not change the address stored in hardware used
//! for address recognition. This must be committed to hardware with a call to
//! config_commit. Please see the relevant TRD for more details.

use returncode::ReturnCode;
pub trait ConfigClient {
    fn config_done(&self, result: ReturnCode);
}

pub trait TxClient {
    fn transmit_event(&self, buf: &'static mut [u8], result: ReturnCode);
}

pub trait RxClient {
    fn receive_event(
        &self,
        buf: &'static mut [u8],
        frame_len: usize,
        crc_valid: bool,
        result: ReturnCode,
    );
}

pub trait Radio: RadioConfig + RadioDriver {}

pub trait RadioConfig {
    fn initialize(&self) -> ReturnCode;
    fn reset(&self) -> ReturnCode;
    fn stop(&self) -> ReturnCode;
    fn is_on(&self) -> bool;
    fn busy(&self) -> bool;

    fn get_tx_power(&self) -> u32;
    fn get_radio_status(&self) -> u32;
    fn send_stop_command(&self) -> ReturnCode;
    fn send_kill_command(&self) -> ReturnCode;
    fn get_command_status(&self) -> (ReturnCode, Option<u32>);
    // fn get_rat_time(&self) -> u32;

    fn set_tx_power(&self, power: u16) -> ReturnCode;
    fn config_commit(&self);
}

pub trait RadioDriver {
    fn set_transmit_client(&self, &'static TxClient);
    fn set_receive_client(&self, &'static RxClient, receive_buffer: &'static mut [u8]);
    fn set_config_client(&self, &'static ConfigClient);
    fn set_receive_buffer(&self, receive_buffer: &'static mut [u8]);
    fn transmit(
        &self,
        tx_buf: &'static mut [u8],
        len: usize,
    ) -> (ReturnCode, Option<&'static mut [u8]>);
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum RadioOperation {
    Enable = 0,
    Tx = 1,
    Rx = 2,
    Configure = 3,
    SetPower = 4,
    StartTimer = 5,
    StopTimer = 6,
    Disable = 7,
    Abort = 8,
    Sleep = 9,
}

impl RadioOperation {
    pub fn get_operation_index(&self) -> u32 {
        match *self {
            RadioOperation::Enable => 0,
            RadioOperation::Tx => 1,
            RadioOperation::Rx => 2,
            RadioOperation::Configure => 3,
            RadioOperation::SetPower => 4,
            RadioOperation::StartTimer => 5,
            RadioOperation::StopTimer => 6,
            RadioOperation::Disable => 7,
            RadioOperation::Abort => 8,
            RadioOperation::Sleep => 9,
        }
    }
}

pub enum PowerMode {
    Active,
    Sleep,
    DeepSleep,
}

impl PowerMode {
    pub fn get_power_mode_index(&self) -> u32 {
        match *self {
            PowerMode::Active => 0,
            PowerMode::Sleep => 1,
            PowerMode::DeepSleep => 2,
        }
    }
}
