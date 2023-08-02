#![no_std]
#![no_main]

extern crate alloc;

mod hardware;

use fugit::RateExtU32;
use cortex_m::delay::Delay;
use embedded_hal::digital::v2::OutputPin;
use rp_pico::hal::clocks::init_clocks_and_plls;
use rp_pico::hal::gpio::{DynPin, FunctionUart};
use rp_pico::hal::uart::{UartPeripheral, UartConfig, DataBits, StopBits};
use rp_pico::hal::{Timer, Watchdog, Clock, Sio};
use rp_pico::Pins;
use rp_pico::pac::{CorePeripherals, Peripherals};

use crate::hardware::serial::{self, println};

fn start() -> ! {

    // Hardware setup.
    let mut pac = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    unsafe {
        serial::start(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            &mut pac.RESETS,
        );
    }

    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);

    let sio = Sio::new(pac.SIO);

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Get the LED pin.
    let mut led_pin = DynPin::from(pins.led);
    led_pin.into_push_pull_output();



    let pins = (
        pins.gpio12.into_mode::<FunctionUart>(),
        pins.gpio13.into_mode::<FunctionUart>(),
    );
    let uart = UartPeripheral::new(pac.UART0, pins, &mut pac.RESETS).enable(
        UartConfig::new(9600.Hz(), DataBits::Eight, None, StopBits::One),
        clocks.peripheral_clock.freq(),
    ).unwrap();

    let mut buf = [0 as u8; 1];
    let mut high = true;
    let mut time_last_blink = timer.get_counter().ticks();
    loop {

        if timer.get_counter().ticks() - time_last_blink > 500_000 {
            uart.write_full_blocking(&mut buf);
            if high {
                led_pin.set_high().unwrap();
            } else {
                led_pin.set_low().unwrap();
            }
            high = !high;
            time_last_blink = timer.get_counter().ticks();
        }
        

    }
}
