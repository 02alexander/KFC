use cortex_m::delay::Delay;
use embedded_hal::{digital::v2::OutputPin, serial::Read, timer::CountDown};
use fugit::{ExtU32, RateExtU32};

use rp_pico as bsp;

use bsp::{
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::DynPin,
        rom_data::reset_to_usb_boot,
        sio::Sio,
        uart::{DataBits, StopBits, UartConfig},
        watchdog::Watchdog,
        Timer,
    },
    pac::CorePeripherals,
    pac::Peripherals,
    Pins,
};

use crate::{
    buttonmatrix::ButtonMatrix,
    encoding::encode,
    hardware::{self, serial::println},
};

#[allow(unused)]
pub fn run() -> ! {
    let mut pac = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);

    let sio = Sio::new(pac.SIO);

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            unsafe {
                hardware::serial::start(pac.USBCTRL_REGS, pac.USBCTRL_DPRAM, clocks.usb_clock, &mut pac.RESETS);
            }
        }
    }

    const ROWS: usize = 5;
    const COLS: usize = 6;
    let mut rows = [
        DynPin::from(pins.gpio20),
        DynPin::from(pins.gpio19),
        DynPin::from(pins.gpio18),
        DynPin::from(pins.gpio17),
        DynPin::from(pins.gpio16),
    ];
    rows.iter_mut().for_each(|p| p.into_push_pull_output());

    let mut cols = [
        DynPin::from(pins.gpio21),
        DynPin::from(pins.gpio22),
        DynPin::from(pins.gpio10),
        DynPin::from(pins.gpio11),
        DynPin::from(pins.gpio7),
        DynPin::from(pins.gpio9),
    ];
    cols.iter_mut().for_each(|p| p.into_pull_down_input());

    let mut butmat = ButtonMatrix { rows, cols };

    // let mut p1 = pins.gpio12.into_push_pull_output();
    // let mut p2 = pins.gpio13.into_push_pull_output();


    let uart_pins = (
        pins.gpio12
            .into_mode::<rp_pico::hal::gpio::pin::FunctionUart>(),
        pins.gpio13
            .into_mode::<rp_pico::hal::gpio::pin::FunctionUart>(),
    );
    let mut uart = rp_pico::hal::uart::UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(115200.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    let mut tick_count_down = timer.count_down();
    tick_count_down.start(500.micros());

    let mut scan_count_down = timer.count_down();
    scan_count_down.start(500.micros());

    let mut led_on = false;
    let mut led_pin = pins.led.into_push_pull_output();
    let mut blink_count_down = timer.count_down();
    blink_count_down.start(500.millis());

    let mut t_last_read = Some(0);

    let mut prev_pressed: Option<[[bool; 6]; 5]> = None;
    loop {
        if blink_count_down.wait().is_ok() {
            if let Some(tl) = t_last_read {
                if timer.get_counter().ticks() - tl < 200_000 {
                    blink_count_down.start(200.millis());
                } else {
                    blink_count_down.start(500.millis());
                }
            }
            if led_on {
                led_pin.set_low().unwrap();
            } else {
                led_pin.set_high().unwrap();
            }
            led_on = !led_on;
        }

        if let Ok(_byte) = uart.read() {
            if let Some(cur_pressed) = prev_pressed {
                let encoded = encode(&cur_pressed);
                uart.write_full_blocking(&encoded);
            }
            t_last_read = Some(timer.get_counter().ticks());
        } else {
        }

        if scan_count_down.wait().is_ok() {
            if let Some(pressed) = butmat.scan(&mut delay) {
                if pressed[4][0] {
                    reset_to_usb_boot(0, 0);
                }
                prev_pressed = Some(pressed);
            } else {
            }
        }
    }
}
