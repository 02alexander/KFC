use cortex_m::delay::Delay;
use embedded_hal::{digital::v2::OutputPin, serial::Write, timer::CountDown};
use fugit::{ExtU32, RateExtU32};
// use panic_probe as _;

use rp_pico as bsp;

use bsp::{
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::{DynPin, Function, Uart},
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
    encoding::decode,
    hardware::{self, serial::println},
};

#[rustfmt::skip]
pub const LOGITECH_GAMING_KEYBOARD_REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x01, //    Usage Page (Generic Desktop)
    0x09, 0x06, //    Usage (Keyboard)
    0xA1, 0x01, //    Collection (Application)
    0x05, 0x07, //        Usage Page (Keyboard/Keypad)
    0x19, 0xE0, //        Usage Minimum (Keyboard Left Control)
    0x29, 0xE7, //        Usage Maximum (Keyboard Right GUI)
    0x15, 0x00, //        Logical Minimum (0)
    0x25, 0x01, //        Logical Maximum (1)
    0x75, 0x01, //        Report Size (1)
    0x95, 0x08, //        Report Count (8)
    0x81, 0x02, //        Input (Data,Var,Abs,NWrp,Lin,Pref,NNul,Bit)
    0x95, 0x01, //        Report Count (1)
    0x75, 0x08, //        Report Size (8)
    0x81, 0x01, //        Input (Const,Ary,Abs)
    0x95, 0x05, //        Report Count (5)
    0x75, 0x01, //        Report Size (1)
    0x05, 0x08, //        Usage Page (LEDs)
    0x19, 0x01, //        Usage Minimum (Num Lock)
    0x29, 0x05, //        Usage Maximum (Kana)
    0x91, 0x02, //        Output (Data,Var,Abs,NWrp,Lin,Pref,NNul,NVol,Bit)
    0x95, 0x01, //        Report Count (1)
    0x75, 0x03, //        Report Size (3)
    0x91, 0x01, //        Output (Const,Ary,Abs,NWrp,Lin,Pref,NNul,NVol,Bit)
    0x95, 0x06, //        Report Count (6)
    0x75, 0x08, //        Report Size (8)
    0x15, 0x00, //        Logical Minimum (0)
    0x26, 0x97, 0x00, //        Logical Maximum (151)
    0x05, 0x07, //        Usage Page (Keyboard/Keypad)
    0x19, 0x00, //        Usage Minimum (Undefined)
    0x29, 0x97, //        Usage Maximum (Keyboard LANG8)
    0x81, 0x00, //        Input (Data,Ary,Abs)
    0xC0, //        End Collection
];

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
        } else {
            let usb_bus = UsbBusAllocator::new(bsp::hal::usb::UsbBus::new(
                pac.USBCTRL_REGS,
                pac.USBCTRL_DPRAM,
                clocks.usb_clock,
                true,
                &mut pac.RESETS
            ));
            let mut keyboard = UsbHidClassBuilder::new()
                .add_device(NKROBootKeyboardConfig::default())
                .build(&usb_bus);

            let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001))
                .manufacturer("usbd-humane-interface-device")
                .product("Custom Keyboard")
                .serial_number("TEST")
                .build();
        }
    }

    let mut rows = [
        DynPin::from(pins.gpio8),
        DynPin::from(pins.gpio10),
        DynPin::from(pins.gpio15),
        DynPin::from(pins.gpio13),
        DynPin::from(pins.gpio12),
    ];
    rows.iter_mut().for_each(|p| p.into_push_pull_output());

    let mut cols = [
        DynPin::from(pins.gpio7),
        DynPin::from(pins.gpio6),
        DynPin::from(pins.gpio22),
        DynPin::from(pins.gpio26),
        DynPin::from(pins.gpio2),
        DynPin::from(pins.gpio0),
    ];
    cols.iter_mut().for_each(|p| p.into_pull_down_input());

    let mut butmat = ButtonMatrix { rows, cols };

    let uart_pins = (
        pins.gpio16.into_mode::<Function<Uart>>(),
        pins.gpio17.into_mode::<Function<Uart>>(),
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

    let mut test_count_down = timer.count_down();
    test_count_down.start(10.millis());

    let mut led_on = false;
    let mut led_pin = pins.led.into_push_pull_output();
    let mut blink_count_down = timer.count_down();
    blink_count_down.start(500.millis());

    delay.delay_ms(1000);

    let mut tot_pressed = [[false; 12]; 5];
    let mut counter = 1;
    let mut prev_pressed: Option<[[bool; 12]; 5]> = None;
    loop {
        if blink_count_down.wait().is_ok() {
            if led_on {
                led_pin.set_low().unwrap();
            } else {
                led_pin.set_high().unwrap();
            }
            led_on = !led_on;
        }

        if test_count_down.wait().is_ok() {
            uart.write(counter).unwrap();
            let mut buffer = [0 as u8; 4];
            if let Ok(_n) = uart.read_raw(&mut buffer) {
                let mut pressed = [[false; 6]; 5];
                decode(&buffer, &mut pressed);
                for ri in 0..5 {
                    for ci in 0..6 {
                        tot_pressed[ri][11 - ci] = pressed[ri][ci];
                    }
                }
            }
        }

        if scan_count_down.wait().is_ok() {
            if let Some(pressed) = butmat.scan(&mut delay) {
                if pressed[4][0] {
                    reset_to_usb_boot(0, 0);
                }
                for ri in 0..5 {
                    for ci in 0..6 {
                        tot_pressed[ri][ci] = pressed[ri][ci];
                    }
                }
                if let Some(prev_pressed) = prev_pressed {
                    for ri in 0..5 {
                        for ci in 0..12 {
                            if prev_pressed[ri][ci] && !tot_pressed[ri][ci] {
                                println!("released {:?}", (ri, ci));
                            } else if !prev_pressed[ri][ci] && tot_pressed[ri][ci] {
                                println!("pressed {:?}", (ri, ci));
                            }
                        }
                    }
                }
                prev_pressed = Some(tot_pressed);
            }
        }

        #[cfg(not(debug_assertions))]
        {
            if tick_count_down.wait().is_ok() {
                match keyboard.tick() {
                    Err(UsbHidError::WouldBlock) => {}
                    Ok(_) => {}
                    Err(_) => {
                        error!("Sending tick");
                    }
                }
            }

            if usb_dev.poll(&mut [&mut keyboard]) {
                match keyboard.device().read_report() {
                    Err(UsbError::WouldBlock) => {}
                    Err(_e) => {
                        error!("Failed to read keyeboard report");
                    }
                    Ok(_leds) => {}
                }
            }
        }
    }
}
