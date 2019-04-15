use crate::enum_primitive::cast::FromPrimitive;
use crate::helium::{channels::RadioChannel, device, framer::PayloadType};
use core::cell::Cell;
use core::cmp::min;
use kernel::common::cells::{OptionalCell, TakeCell};
use kernel::hil::time::Frequency;
use kernel::{AppId, AppSlice, Callback, Driver, Grant, ReturnCode, Shared};

// Syscall number
pub const DRIVER_NUM: usize = 0xCC1352;

#[derive(Debug, Clone, Copy)]
pub enum PowerMode {
    Active,
    Sleep,
    DeepSleep,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeliumState {
    NotInitialized,
    Initialized,
    TransmitIdle,
    Transmit(RadioChannel),
    ReceiveIdle,
    Receive(RadioChannel),
}

#[derive(Copy, Clone)]
enum Expiration {
    Disabled,
    Abs(u32),
}

#[derive(Copy, Clone)]
struct AlarmData {
    t0: u32,
    expiration: Expiration,
}

impl AlarmData {
    fn new() -> AlarmData {
        AlarmData {
            t0: 0,
            expiration: Expiration::Disabled,
        }
    }
}

// #[derive(Default)]
#[allow(unused)]
pub struct App {
    tx_callback: Option<Callback>,
    rx_callback: Option<Callback>,
    app_cfg: Option<AppSlice<Shared, u8>>,
    app_write: Option<AppSlice<Shared, u8>>,
    app_read: Option<AppSlice<Shared, u8>>,
    pending_tx: Option<(u8, Option<PayloadType>)>, // Change u32 to keyid and fec mode later on during implementation
    tx_interval_ms: u32,

    process_status: Option<HeliumState>,
    alarm_data: AlarmData,

    // 400 ms is maximum per FCC
    random_nonce: u32, // Randomness to sending interval to reduce collissions
}

impl Default for App {
    fn default() -> App {
        App {
            tx_callback: None,
            rx_callback: None,
            app_cfg: None,
            app_write: None,
            app_read: None,
            pending_tx: None,
            tx_interval_ms: 400,
            process_status: Some(HeliumState::NotInitialized),
            alarm_data: AlarmData::new(),
            random_nonce: 0xdeadbeef,
        }
    }
}

impl App {
    fn random_nonce(&mut self) -> u32 {
        let mut next_nonce = ::core::num::Wrapping(self.random_nonce);
        next_nonce ^= next_nonce << 13;
        next_nonce ^= next_nonce >> 17;
        next_nonce ^= next_nonce << 5;
        self.random_nonce = next_nonce.0;
        self.random_nonce
    }

    // Set the next alarm for this app using the period and provided start time.
    fn set_next_alarm<F: Frequency>(&mut self, now: u32) {
        self.alarm_data.t0 = now;
        let nonce = self.random_nonce() % 10;

        let period_ms = (self.tx_interval_ms + nonce) * F::frequency() / 1000;
        self.alarm_data.expiration = Expiration::Abs(now.wrapping_add(period_ms));
    }
}

pub struct Helium<'a, A>
where
    A: kernel::hil::time::Alarm,
{
    app: Grant<App>,
    kernel_tx: TakeCell<'static, [u8]>,
    current_app: OptionalCell<AppId>,
    device: &'a device::Device<'a>,
    device_id: u32,
    alarm: &'a A,
    busy: Cell<bool>,
}

impl<A> Helium<'a, A>
where
    A: kernel::hil::time::Alarm,
{
    pub fn new(
        container: Grant<App>,
        tx_buf: &'static mut [u8],
        device: &'a device::Device<'a>,
        device_id: u32,
        alarm: &'a A,
    ) -> Helium<'a, A> {
        Helium {
            app: container,
            kernel_tx: TakeCell::new(tx_buf),
            current_app: OptionalCell::empty(),
            device,
            device_id,
            alarm,
            busy: Cell::new(false),
        }
    }

    fn reset_active_alarm(&self) {
        let now = self.alarm.now();
        let mut next_alarm = u32::max_value();
        let mut next_dist = u32::max_value();
        for app in self.app.iter() {
            app.enter(|app, _| match app.alarm_data.expiration {
                Expiration::Abs(exp) => {
                    let t_dist = exp.wrapping_sub(now);
                    if next_dist > t_dist {
                        next_alarm = exp;
                        next_dist = t_dist;
                    }
                }
                Expiration::Disabled => {}
            });
        }
        if next_alarm != u32::max_value() {
            self.alarm.set_alarm(next_alarm);
        }
    }

    /// Utility function to perform an action on an app in a system call.
    #[inline]
    fn do_with_app<F>(&self, appid: AppId, closure: F) -> ReturnCode
    where
        F: FnOnce(&mut App) -> ReturnCode,
    {
        self.app
            .enter(appid, |app, _| closure(app))
            .unwrap_or_else(|err| err.into())
    }

    /// Utility function to perform an action using an app's config buffer.
    #[inline]
    fn do_with_cfg<F>(&self, appid: AppId, len: usize, closure: F) -> ReturnCode
    where
        F: FnOnce(&[u8]) -> ReturnCode,
    {
        self.app
            .enter(appid, |app, _| {
                app.app_cfg
                    .take()
                    .as_ref()
                    .map_or(ReturnCode::EINVAL, |cfg| {
                        if cfg.len() != len {
                            return ReturnCode::EINVAL;
                        }
                        closure(cfg.as_ref())
                    })
            })
            .unwrap_or_else(|err| err.into())
    }

    /// If the driver is currently idle and there are pending transmissions,
    /// pick an app with a pending transmission and return its `AppId`.
    fn get_next_tx_if_idle(&self) -> Option<AppId> {
        if self.current_app.is_some() {
            return None;
        }
        let mut pending_app = None;
        for app in self.app.iter() {
            app.enter(|app, _| {
                if app.pending_tx.is_some() {
                    pending_app = Some(app.appid());
                }
            });
            if pending_app.is_some() {
                break;
            }
        }
        pending_app
    }

    /// Performs `appid`'s pending transmission asynchronously. If the
    /// transmission is not successful, the error is returned to the app via its
    /// `tx_callback`. Assumes that the driver is currently idle and the app has
    /// a pending transmission.
    #[inline]
    fn perform_tx_async(&self, appid: AppId) {
        let result = self.perform_tx_sync(appid);
        if result != ReturnCode::SUCCESS {
            let _ = self.app.enter(appid, |app, _| {
                app.tx_callback
                    .take()
                    .map(|mut cb| cb.schedule(result.into(), 0, 0));
            });
        }
    }

    /// Performs `appid`'s pending transmission synchronously. The result is
    /// returned immediately to the app. Assumes that the driver is currently
    /// idle and the app has a pending transmission.
    #[inline]
    fn perform_tx_sync(&self, appid: AppId) -> ReturnCode {
        self.do_with_app(appid, |app| {
            let (device_id, payload_type) = match app.pending_tx.take() {
                Some(pending_tx) => pending_tx,
                None => {
                    return ReturnCode::SUCCESS;
                }
            };

            let result = self.kernel_tx.take().map_or(ReturnCode::ENOMEM, |kbuf| {
                let seq: u8 = 0;
                let mut frame =
                    match self
                        .device
                        .prepare_data_frame(kbuf, seq, device_id, payload_type)
                    {
                        Ok(frame) => frame,
                        Err(kbuf) => {
                            self.kernel_tx.replace(kbuf);
                            return ReturnCode::FAIL;
                        }
                    };

                let result = app
                    .app_write
                    .take()
                    .as_ref()
                    .map(|payload| match frame.info.payload_type {
                        Some(PayloadType::None) => frame.frame_payload(payload.as_ref()),
                        Some(PayloadType::Packetizer) => frame.append_payload(payload.as_ref()),
                        Some(PayloadType::Cauterize) => frame.cauterize_payload(payload.as_ref()),
                        Some(PayloadType::LDPC) => {
                            //frame.frame_payload_ldpc(payload.as_ref()),
                            frame.frame_payload(payload.as_ref())
                        }
                        // Will never get to this
                        None => ReturnCode::EINVAL,
                    })
                    .unwrap_or(ReturnCode::EINVAL);

                if result != ReturnCode::SUCCESS {
                    return result;
                }
                // Finally, transmit the frame
                let (result, mbuf) = self.device.transmit(frame);
                if let Some(buf) = mbuf {
                    self.kernel_tx.replace(buf);
                }
                result
            });
            if result == ReturnCode::SUCCESS {
                self.current_app.set(appid);
            }
            result
        })
    }

    /// Schedule the next transmission if there is one pending. Performs the
    /// transmission asynchronously, returning any errors via callbacks.
    #[inline]
    fn do_next_tx_async(&self) {
        self.get_next_tx_if_idle()
            .map(|appid| self.perform_tx_async(appid));
    }

    /// Schedule the next transmission if there is one pending. If the next
    /// transmission happens to be the one that was just queued, then the
    /// transmission is synchronous. Hence, errors must be returned immediately.
    /// On the other hand, if it is some other app, then return any errors via
    /// callbacks.
    #[inline]
    fn do_next_tx_sync(&self, new_appid: AppId) -> ReturnCode {
        self.get_next_tx_if_idle()
            .map(|appid| {
                if appid == new_appid {
                    self.perform_tx_sync(appid)
                } else {
                    self.perform_tx_async(appid);
                    ReturnCode::SUCCESS
                }
            })
            .unwrap_or(ReturnCode::SUCCESS)
    }
}

// Timer alarm
impl<'a, A> kernel::hil::time::Client for Helium<'a, A>
where
    A: kernel::hil::time::Alarm,
{
    // When an alarm is fired, we find which apps have expired timers. Expired
    // timers indicate a desire to perform some operation (e.g. start an
    // advertising or scanning event). We know which operation based on the
    // current app's state.
    //
    // In case of collision---if there is already an event happening---we'll
    // just delay the operation for next time and hope for the best. Since some
    // randomness is added for each period in an app's timer, collisions should
    // be rare in practice.
    //
    // TODO: perhaps break ties more fairly by prioritizing apps that have least
    // recently performed an operation.
    fn fired(&self) {
        let now = self.alarm.now();

        self.app.each(|app| {
            if let Expiration::Abs(exp) = app.alarm_data.expiration {
                let expired =
                    now.wrapping_sub(app.alarm_data.t0) >= exp.wrapping_sub(app.alarm_data.t0);
                if expired {
                    if self.busy.get() {
                        // The radio is currently busy, so we won't be able to start the
                        // operation at the appropriate time. Instead, reschedule the
                        // operation for later. This is _kind_ of simulating actual
                        // on-air interference
                        debug!("HELIUM: operation delayed for app {:?}", app.appid());
                        app.set_next_alarm::<A::Frequency>(self.alarm.now());
                        return;
                    }

                    app.alarm_data.expiration = Expiration::Disabled;

                    match app.process_status {
                        Some(HeliumState::TransmitIdle) => {
                            self.busy.set(true);
                            app.process_status =
                                Some(HeliumState::Transmit(RadioChannel::Channel1));
                            // hop channel
                        }
                        Some(HeliumState::ReceiveIdle) => {
                            self.busy.set(true);
                            app.process_status = Some(HeliumState::Receive(RadioChannel::Channel1));
                            //self.receiving_app.set(app.appid());
                        }
                        _ => debug!(
                            "app: {:?} \t invalid state {:?}",
                            app.appid(),
                            app.process_status
                        ),
                    }
                }
            }
        });
        self.reset_active_alarm();
    }
}

impl<A> Driver for Helium<'a, A>
where
    A: kernel::hil::time::Alarm,
{
    /// Setup buffers to read/write from.
    ///
    ///  `allow_num`
    ///
    /// - `0`: Read buffer. Will contain the received frame.
    /// - `1`: Write buffer. Contains the frame payload to be transmitted.
    /// - `2`: Config buffer. Used to contain miscellaneous data associated with
    ///        some commands because the system call parameters / return codes are
    ///        not enough to convey the desired information.
    fn allow(
        &self,
        appid: AppId,
        allow_num: usize,
        slice: Option<AppSlice<Shared, u8>>,
    ) -> ReturnCode {
        match allow_num {
            0 | 1 | 2 => self.do_with_app(appid, |app| {
                match allow_num {
                    0 => app.app_read = slice,
                    1 => app.app_write = slice,
                    2 => app.app_cfg = slice,
                    _ => {}
                }
                ReturnCode::SUCCESS
            }),
            _ => ReturnCode::ENOSUPPORT,
        }
    }

    /// Setup callbacks.
    ///
    ///  `subscribe_num`
    /// - `0`: Setup callback for when frame is received.
    /// - `1`: Setup callback for when frame is transmitted.
    fn subscribe(
        &self,
        subscribe_num: usize,
        callback: Option<Callback>,
        app_id: AppId,
    ) -> ReturnCode {
        if let Some(subscribe) = HeliumCallback::from_usize(subscribe_num) {
            match subscribe {
                HeliumCallback::RxCallback => self.do_with_app(app_id, |app| {
                    app.rx_callback = callback;
                    ReturnCode::SUCCESS
                }),
                HeliumCallback::TxCallback => self.do_with_app(app_id, |app| {
                    app.tx_callback = callback;
                    ReturnCode::SUCCESS
                }),
            }
        } else {
            ReturnCode::ENOSUPPORT
        }
    }
    /// COMMANDS
    ///
    /// ### `command_num`
    ///
    /// - `0`: Driver check.
    /// - `1`: Initialize radio.
    /// - `2`: Get radio status.
    /// - `3`: Send stop radio command.
    /// - `4`: Send kill radio operation command.
    /// - `5`: Set device configuration.
    /// - `6`: Set next device transmission.
    /// -       a) payload_type field can be one of 3 types:
    ///             None (0x00)
    ///             Packetizer (0x01)
    ///             Cauterize (0x10)
    ///
    /// = `7`: Set device endpoint address.
    ///
    fn command(
        &self,
        command_num: usize,
        _addr: usize,
        payload_type: usize,
        appid: AppId,
    ) -> ReturnCode {
        if let Some(command) = HeliumCommand::from_usize(command_num) {
            match command {
                // Handle callback for CMDSTA after write to CMDR
                HeliumCommand::DriverCheck => ReturnCode::SUCCESS,
                HeliumCommand::Initialize => self.device.initialize(),
                HeliumCommand::GetRadioStatus => {
                    if self.device.is_on() {
                        ReturnCode::SUCCESS
                    } else {
                        ReturnCode::EOFF
                    }
                }
                HeliumCommand::SendStopCommand => self.device.send_stop_command(),
                HeliumCommand::SendKillCommand => self.device.send_kill_command(),
                HeliumCommand::SetDeviceConfig => self.device.set_device_config(),
                HeliumCommand::SetNextTx => {
                    self.do_with_app(appid, |app| {
                        if app.pending_tx.is_some() {
                            return ReturnCode::EBUSY;
                        }
                        //let device_id = addr as u16;
                        let device_id = (self.device_id & 0x000000FF) as u8;
                        let pl_type = match PayloadType::from_cmd(payload_type) {
                            Some(pl_type) => pl_type,
                            None => {
                                return ReturnCode::FAIL;
                            }
                        };
                        let next_tx = Some((device_id, Some(pl_type)));
                        if next_tx.is_none() {
                            return ReturnCode::EINVAL;
                        }
                        app.pending_tx = next_tx;
                        self.do_next_tx_sync(appid)
                        /*
                        let next_tx = app.app_cfg.as_ref().and_then(|cfg| {
                            if cfg.len() != 11 {
                                return None;
                            }
                            let caut = match PayloadType::from_slice(cfg.as_ref()[0]) {
                                // The first entry `[0]` should be the encoding type
                                Some(caut) => caut,
                                None => {
                                    return None;
                                }
                            };

                            if caut == PayloadType::None {
                                return Some((address, None));
                            }
                            Some((address, Some(caut)))
                        });

                        let next_tx = Some((device_id, Some(PayloadType::None)));
                        if next_tx.is_none() {
                            return ReturnCode::EINVAL;
                        }
                        app.pending_tx = next_tx;
                        self.do_next_tx_sync(appid)
                        */
                    })
                }
                HeliumCommand::SetAddress => self.do_with_cfg(appid, 10, |cfg| {
                    let mut addr_long = [0u8; 10];
                    addr_long.copy_from_slice(cfg);
                    self.device.set_address_long(addr_long);
                    ReturnCode::SUCCESS
                }),

                HeliumCommand::Invalid => ReturnCode::ENOSUPPORT,
            }
        } else {
            ReturnCode::ENOSUPPORT
        }
    }
}

impl<A> device::TxClient for Helium<'a, A>
where
    A: kernel::hil::time::Alarm,
{
    fn transmit_event(&self, buf: &'static mut [u8], result: ReturnCode) {
        self.kernel_tx.replace(buf);
        self.current_app.take().map(|appid| {
            let _ = self.app.enter(appid, |app, _| {
                app.tx_callback
                    .take()
                    .map(|mut cb| cb.schedule(result.into(), 0, 0));
            });
        });
        self.do_next_tx_async();
    }
}

impl<A> device::RxClient for Helium<'a, A>
where
    A: kernel::hil::time::Alarm,
{
    fn receive_event<'b>(&self, buf: &'b [u8], data_offset: usize, data_len: usize) {
        self.app.each(|app| {
            app.app_read.take().as_mut().map(|rbuf| {
                let rbuf = rbuf.as_mut();
                let len = min(rbuf.len(), data_offset + data_len);
                rbuf[..len].copy_from_slice(&buf[..len]);
                rbuf[0] = data_offset as u8;
                rbuf[1] = data_len as u8;

                app.rx_callback.take().map(|mut cb| cb.schedule(0, 0, 0));
            });
        })
    }
}
enum_from_primitive! {
#[derive(Debug, Clone, Copy)]
pub enum HeliumAllow {
    Read = 0,
    Write = 1,
    Config = 2,
}
}

enum_from_primitive! {
#[derive(Debug, Clone, Copy)]
pub enum HeliumCallback {
    RxCallback = 0,
    TxCallback = 1,
}
}

enum_from_primitive! {
#[derive(Debug, Clone, Copy)]
pub enum HeliumCommand {
    DriverCheck = 0,
    Initialize = 1,
    GetRadioStatus = 2,
    SendStopCommand = 3,
    SendKillCommand = 4,
    SetDeviceConfig = 5,
    SetNextTx = 6,
    SetAddress = 7,
    Invalid,
}
}
