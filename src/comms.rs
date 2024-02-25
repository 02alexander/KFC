use embedded_hal::{
    serial::{Read, Write},
    timer::CountDown,
};
use rp_pico::hal::uart::{Enabled, UartDevice, UartPeripheral, ValidUartPinout};

pub struct ComLink<const N: usize, CD> {
    buffer: [u8; N],
    recvd: usize,
    is_waiting: bool,
    request_countdown: CD,
}

impl<const N: usize, CD> ComLink<N, CD>
where
    CD: CountDown,
{
    pub fn new(countdown: CD) -> Self {
        ComLink {
            buffer: [0; N],
            recvd: 0,
            is_waiting: false,
            request_countdown: countdown,
        }
    }

    pub fn poll<D, P>(&mut self, uart: &mut UartPeripheral<Enabled, D, P>) -> Option<&[u8; N]>
    where
        D: UartDevice,
        P: ValidUartPinout<D>,
    {
        if self.request_countdown.wait().is_ok() {
            if self.is_waiting {
                // Clear input of leftover input.
                let mut trash_bin = [0; 8];
                uart.read_raw(&mut trash_bin).ok()?;
                self.recvd = 0;
            }
            uart.write(0).ok()?;
            self.is_waiting = true;
        }

        if self.is_waiting {
            if uart.uart_is_readable() {
                let byte = uart.read().ok()?;
                self.buffer[self.recvd] = byte;
                self.recvd += 1;
            }

            if self.recvd >= self.buffer.len() {
                self.recvd = 0;
                self.is_waiting = false;
                return Some(&self.buffer);
            }
        }
        None
    }
}
