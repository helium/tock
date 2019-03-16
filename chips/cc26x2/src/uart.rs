//! UART driver, cc26x2 family
use crate::prcm;

use crate::peripheral_interrupts;
use core::cell::Cell;
use cortexm4::nvic;
use kernel::common::cells::MapCell;
use kernel::common::registers::{register_bitfields, ReadOnly, ReadWrite, WriteOnly};
use kernel::debug;

use kernel::hil;
use kernel::hil::uart;
use kernel::ReturnCode;

const MCU_CLOCK: u32 = 48_000_000;

static mut requested: bool = false;

#[repr(C)]
struct UartRegisters {
    dr: ReadWrite<u32>, //Data Section 21.7.1.1
    rsr_ecr: ReadWrite<u32>, //Error Clear Section 21.7.1.3
    _reserved0: [u32; 0x4], 
    fr: ReadOnly<u32, Flags::Register>, //Flag Section 21.7.1.4
    _reserved1: [u32; 0x2],
    ibrd: ReadWrite<u32, IntDivisor::Register>, //Integer Baud-Rate Divisor Section 21.7.1.5
    fbrd: ReadWrite<u32, FracDivisor::Register>, //Fractional Baud-Rate Divisor Section 21.7.1.6
    lcrh: ReadWrite<u32, LineControl::Register>, //Line Control Section 21.7.1.7
    ctl: ReadWrite<u32, Control::Register>, //Control Section 21.7.1.8
    ifls: ReadWrite<u32, FifoLevelSelect::Register>, //Interrupt FIFO Level Select Section 21.7.1.9
    imsc: ReadWrite<u32, Interrupts::Register>, //Interrupt Mask Set/Clear Section 21.7.1.10
    ris: ReadOnly<u32, Interrupts::Register>, // Raw Interrupt Status Section 21.7.1.11
    mis: ReadOnly<u32, Interrupts::Register>, //Masked Interrupt Status Section 21.7.1.12
    icr: WriteOnly<u32, Interrupts::Register>, //Interrupt Clear Section 21.7.1.13
    dmactl: ReadWrite<u32>, // DMA Control Section 2
}

register_bitfields![
    u32,
    Control [
        UART_ENABLE OFFSET(0) NUMBITS(1) [],
        LB_ENABLE OFFSET(7) NUMBITS(1) [],
        TX_ENABLE OFFSET(8) NUMBITS(1) [],
        RX_ENABLE OFFSET(9) NUMBITS(1) []
    ],
    LineControl [
        FIFO_ENABLE OFFSET(4) NUMBITS(1) [],
        WORD_LENGTH OFFSET(5) NUMBITS(2) [
            Len5 = 0x0,
            Len6 = 0x1,
            Len7 = 0x2,
            Len8 = 0x3
        ]
    ],
    FifoLevelSelect [
        RXSEL OFFSET(3) NUMBITS(3) [
            OneEighth = 0,
            OneQuarter = 1,
            Half = 2,
            ThreeQuarters = 3,
            SevenEights = 4
        ],
        TXSEL OFFSET(0) NUMBITS(3) [
            OneEighth = 0,
            OneQuarter = 1,
            Half = 2,
            ThreeQuarters = 3,
            SevenEights = 4
        ]
    ],
    IntDivisor [
        DIVISOR OFFSET(0) NUMBITS(16) []
    ],
    FracDivisor [
        DIVISOR OFFSET(0) NUMBITS(6) []
    ],
    Flags [
        CTS OFFSET(0) NUMBITS(1) [],
        BUSY OFFSET(3) NUMBITS(1) [],
        RX_FIFO_EMPTY OFFSET(4) NUMBITS(1) [],
        TX_FIFO_FULL OFFSET(5) NUMBITS(1) [],
        RX_FIFO_FULL OFFSET(6) NUMBITS(1) [],
        TX_FIFO_EMPTY OFFSET(7) NUMBITS(1) []
    ],
    Interrupts [
         ALL_INTERRUPTS OFFSET(0) NUMBITS(12) [
            // sets all interrupts without writing 1's to reg with undefined behavior
            Set =  0b111111110010,
            // you are allowed to write 0 to everyone
            Clear = 0x000000
        ],
        CTSIMM OFFSET(1) NUMBITS(1) [],              // clear to send interrupt mask
        RX OFFSET(4) NUMBITS(1) [],                  // receive interrupt mask
        TX OFFSET(5) NUMBITS(1) [],                  // transmit interrupt mask
        RX_TIMEOUT OFFSET(6) NUMBITS(1) [],          // receive timeout interrupt mask
        FE OFFSET(7) NUMBITS(1) [],                  // framing error interrupt mask
        PE OFFSET(8) NUMBITS(1) [],                  // parity error interrupt mask
        BE OFFSET(9) NUMBITS(1) [],                  // break error interrupt mask
        OE OFFSET(10) NUMBITS(1) [],                 // overrun error interrupt mask
        END_OF_TRANSMISSION OFFSET(11) NUMBITS(1) [] // end of transmission interrupt mask
    ]
];

pub struct UART<'a> {
    nvic: nvic::Nvic,
    num: PeripheralNum,
    registers: &'a UartRegisters,
    tx: MapCell<&'a mut uart::TxRequest<'a>>,
    rx: MapCell<&'a mut uart::RxRequest<'a>>,
    receiving_word: Cell<bool>,
}

use enum_primitive::cast::{FromPrimitive, ToPrimitive};
use enum_primitive::enum_from_primitive;
enum_from_primitive! {
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PeripheralNum {
    _0,
    _1,
}
}

static mut GRANTED: [bool; 2] = [false, false];

use crate::memory_map::{UART0_BASE, UART1_BASE};

impl<'a> UART<'a> {
    pub fn new(num: PeripheralNum) -> UART<'a> {
        unsafe {
            if GRANTED[num as usize] == false {
                GRANTED[num as usize] = true;
                return Self::unsafe_new(num);
            } else {
                panic!(
                    "CC26x2: You have attempted to initialize UART {:?} more than once!",
                    num
                );
            }
        }
    }

    pub unsafe fn unsafe_new(num: PeripheralNum) -> UART<'a> {
        // a counter tracking if you've given these out would help make this safe
        let registers = match num {
            PeripheralNum::_0 => &*(UART0_BASE as *const UartRegisters),
            PeripheralNum::_1 => &*(UART1_BASE as *const UartRegisters),
        };

        let nvic = match num {
            PeripheralNum::_0 => nvic::Nvic::new(peripheral_interrupts::NVIC_IRQ::UART0 as u32),
            PeripheralNum::_1 => nvic::Nvic::new(peripheral_interrupts::NVIC_IRQ::UART1 as u32),
        };

        let ret = UART {
            nvic,
            num,
            registers,
            tx: MapCell::empty(),
            rx: MapCell::empty(),

            receiving_word: Cell::new(false),
        };

        // initialize power, clock and interrupts so it's usable
        ret.initialize();

        ret
    }

    /// Initialize the UART hardware.
    ///
    /// This function needs to be run before the UART module is used.
    pub fn initialize(&self) {
        self.power_and_clock();
        self.enable_interrupts();
    }

    fn power_and_clock(&self) {
        prcm::Power::enable_domain(prcm::PowerDomain::Serial);
        while !prcm::Power::is_enabled(prcm::PowerDomain::Serial) {}
        prcm::Clock::enable_uarts();
    }

    fn set_baud_rate(&self, baud_rate: u32) {
        // Fractional baud rate divider
        let div = (((MCU_CLOCK * 8) / baud_rate) + 1) / 2;
        // Set the baud rate
        self.registers.ibrd.write(IntDivisor::DIVISOR.val(div / 64));
        self.registers
            .fbrd
            .write(FracDivisor::DIVISOR.val(div % 64));
    }

    fn fifo_enable(&self) {
        self.registers.lcrh.modify(LineControl::FIFO_ENABLE::SET);
    }

    fn fifo_disable(&self) {
        self.registers.lcrh.modify(LineControl::FIFO_ENABLE::CLEAR);
    }

    fn disable(&self) {
        // disable interrupts
        self.registers.imsc.write(Interrupts::ALL_INTERRUPTS::CLEAR);
        self.fifo_disable();
        self.registers.ctl.modify(
            Control::UART_ENABLE::CLEAR + Control::TX_ENABLE::CLEAR + Control::RX_ENABLE::CLEAR,
        );
    }

    fn enable_interrupts(&self) {
        self.registers.ifls.modify(FifoLevelSelect::TXSEL::OneEighth);
        self.registers.ifls.modify(FifoLevelSelect::RXSEL::OneEighth);
    }

    pub fn write(&self, c: u32) {
        // Put byte in data register
        self.registers.dr.set(c);
    }

    // Pulls a byte out of the RX FIFO.
    #[inline]
    pub fn read(&self) -> u32 {
        self.registers.dr.get()
    }

    /// Checks if there is space in the transmit fifo queue.
    #[inline]
    pub fn rx_fifo_not_empty(&self) -> bool {
        !self.registers.fr.is_set(Flags::RX_FIFO_EMPTY)
    }

    /// Checks if there is space in the transmit fifo queue.
    #[inline]
    pub fn tx_fifo_not_full(&self) -> bool {
        !self.registers.fr.is_set(Flags::TX_FIFO_FULL)
    }
}

impl<'a> uart::Uart<'a> for UART<'a> {}
impl<'a> uart::UartPeripheral<'a> for UART<'a> {}

impl<'a> uart::InterruptHandler<'a> for UART<'a> {
    /// this particular implementation can use hardware to determine state
    fn handle_interrupt(
        &self,
        _state: hil::uart::PeripheralState,
    ) -> (
        Option<&mut hil::uart::TxRequest<'a>>,
        Option<&mut hil::uart::RxRequest<'a>>,
    ) {
        // clear and enable interrupts in the beginning as actions might trigger more
        self.nvic.clear_pending();
        self.nvic.enable();
        
        let (mut tx_complete, mut rx_complete) = (None, None);

        // // Hardware RX FIFO is not empty
        //while self.rx_fifo_not_empty() {

            // buffer read request was made
            if self.rx.is_some() {
                self.rx.take().map(|rx| {
                    // read in a byte
                    // Hardware RX FIFO is not empty
                    while self.rx_fifo_not_empty() {
                        let byte = self.read() as u8;
        
                        if byte == b'\n' {
                            rx.new_line_return = true;
                        }
                        rx.req.push(byte);


                        if rx.req.request_completed() || rx.new_line_return {
                            if rx.new_line_return {
                                rx.req.push(b'\0');
                            }
                            break;   
                        }
                    }

                    if rx.req.request_completed() || rx.new_line_return {
                        self.registers.imsc.modify(
                            Interrupts::RX::CLEAR
                                + Interrupts::RX_TIMEOUT::CLEAR
                        );
                        rx_complete = Some(rx);
                    } else {
                        self.rx.put(rx);
                    }
                });
            }
            // no current read request
            // else {
            //     // read bytes into the void to avoid hardware RX buffer overflow
            //     self.read();
            // }
       //}

        //if we have a request, handle it
        self.tx.take().map(|tx| {
            // send out one byte at a time, IRQ when TX FIFO empty will bring us back
            if self.tx_fifo_not_full() && !tx.request_completed() {
                if let Some(item) = tx.pop() {
                    self.write(item as u32);
                    if self.num == PeripheralNum::_1 {
                        //debug!("{}", item as char);
                    }
                }
            }

            if tx.request_completed() {
                self.registers.imsc.modify(
                    Interrupts::TX::CLEAR +
                    Interrupts::END_OF_TRANSMISSION::CLEAR,
                );
                tx_complete = Some(tx);
            } else {
                self.tx.put(tx);
            }
        });

        // Clear interrupts
        self.registers.icr.write(Interrupts::ALL_INTERRUPTS::SET);

        (tx_complete, rx_complete)
    }
}

impl<'a> uart::Configure for UART<'a> {
    fn configure(&self, params: uart::Parameters) -> ReturnCode {
        // These could probably be implemented, but are currently ignored, so
        // throw an error.
        if params.stop_bits != uart::StopBits::One {
            return ReturnCode::ENOSUPPORT;
        }
        if params.parity != uart::Parity::None {
            return ReturnCode::ENOSUPPORT;
        }
        if params.hw_flow_control != false {
            return ReturnCode::ENOSUPPORT;
        }

        // Disable the UART before configuring
        self.disable();

        self.set_baud_rate(params.baud_rate);

        // Set word length
        let word_width;
        match params.width {
            uart::Width::Six => word_width = LineControl::WORD_LENGTH::Len6,
            uart::Width::Seven => word_width = LineControl::WORD_LENGTH::Len7,
            uart::Width::Eight => word_width = LineControl::WORD_LENGTH::Len8,
        }
        self.registers.lcrh.write(word_width);

        self.fifo_enable();

        self.enable_interrupts();

        // Enable UART, RX and TX
        self.registers
            .ctl
            .write(Control::UART_ENABLE::SET + Control::RX_ENABLE::SET + Control::TX_ENABLE::SET);

        ReturnCode::SUCCESS
    }
}

impl<'a> uart::Transmit<'a> for UART<'a> {
    fn transmit_buffer(&self, tx: &'a mut uart::TxRequest<'a>) -> ReturnCode {
        self.registers.imsc.modify(
            Interrupts::TX::SET + 
            Interrupts::END_OF_TRANSMISSION::SET
        );

        if self.num == PeripheralNum::_1 {
            //debug!("transmit_buffer begin");
        }
        let mut count = 0;
        // we will send one byte, causing EOT interrupt
        if self.tx_fifo_not_full() && !tx.request_completed(){
            if let Some(item) = tx.pop() {
                self.write(item as u32);

                if self.num == PeripheralNum::_1 {
                    //debug!("{}", item as char);
                }
            }
            

        }

        if self.num == PeripheralNum::_1 {
            //debug!("transmit_buffer end");
        }

        
        self.tx.put(tx);
        
        ReturnCode::SUCCESS
    }

    fn transmit_word(&self, word: u32) -> ReturnCode {
        // if there's room in outgoing FIFO and no buffer Request
        if self.tx_fifo_not_full() && self.tx.is_none() {
            self.write(word);
            return ReturnCode::SUCCESS;
        }
        ReturnCode::FAIL
    }

    fn transmit_abort(&self) -> Option<&'a mut uart::TxRequest<'a>> {
        self.tx.take()
    }
}

impl<'a> uart::Receive<'a> for UART<'a> {
    fn receive_buffer(&self, request: &'a mut uart::RxRequest<'a>) -> ReturnCode {
        if self.rx.is_some() || self.receiving_word.get() {

            ReturnCode::EBUSY
        } else {
            self.registers.imsc.modify(
            Interrupts::RX::SET
                + Interrupts::RX_TIMEOUT::SET
            );
            self.rx.put(request);

            ReturnCode::SUCCESS
        }
    }

    fn receive_word(&self) -> ReturnCode {
        if self.rx.is_some() || self.receiving_word.get() {
            ReturnCode::EBUSY
        } else {
            self.receiving_word.set(true);
            ReturnCode::SUCCESS
        }
    }

    fn receive_abort(&self) -> Option<&'a mut uart::RxRequest<'a>> {
        self.rx.take()
    }
}
