use alloc::vec::Vec;
use cortex_m::delay::Delay;
use embedded_hal::timer::CountDown;
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
use usb_device::{
    class_prelude::UsbBusAllocator,
    prelude::{UsbDeviceBuilder, UsbVidPid},
    UsbError,
};
use usbd_human_interface_device::{
    device::keyboard::NKROBootKeyboardConfig, usb_class::UsbHidClassBuilder, UsbHidError,
};

use crate::{
    buttonmatrix::ButtonMatrix,
    comms::ComLink,
    encoding::decode,
    layout::KeyboardLogic,
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

    // unsafe {
    //     hardware::serial::start(pac.USBCTRL_REGS, pac.USBCTRL_DPRAM, clocks.usb_clock, &mut pac.RESETS);
    // }
    let usb_bus = UsbBusAllocator::new(bsp::hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    let mut keyboard = UsbHidClassBuilder::new()
        .add_device(NKROBootKeyboardConfig::default())
        .build(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001))
        .manufacturer("usbd-humane-interface-device")
        .product("Custom Keyboard")
        .serial_number("TEST")
        .build();

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

    let mut led_on = false;
    let mut led_pin = pins.led.into_push_pull_output();
    let mut blink_count_down = timer.count_down();
    blink_count_down.start(500.millis());

    let mut tot_pressed = [[false; 12]; 5];
    let mut prev_pressed: Option<[[bool; 12]; 5]> = None;

    let mut kblogic = KeyboardLogic::new(&timer);

    let mut slave_req = timer.count_down();
    slave_req.start(10.millis());
    let mut comms = ComLink::new(slave_req);

    let mut t_last_read = Some(0);

    loop {
        // if blink_count_down.wait().is_ok() {
        //     if let Some(tl) = t_last_read {
        //         if timer.get_counter().ticks() - tl < 200_000 {
        //             blink_count_down.start(200.millis());
        //         } else {
        //             blink_count_down.start(500.millis());
        //         }
        //     }
        //     if led_on {
        //         led_pin.set_low().unwrap();
        //     } else {
        //         led_pin.set_high().unwrap();
        //     }
        //     led_on = !led_on;
        // }

        if uart.uart_is_readable() {
            t_last_read = Some(timer.get_counter().ticks());
        }
        if let Some(buf) = comms.poll(&mut uart) {
            let mut pressed = [[false; 6]; 5];
            // println!("got {:?}", buf);
            decode(buf, &mut pressed);
            for ri in 0..5 {
                for ci in 0..6 {
                    tot_pressed[ri][6 + ci] = pressed[ri][ci];
                }
            }
        }

        if scan_count_down.wait().is_ok() {
            if let Some(pressed) = butmat.scan(&mut delay) {
                if pressed[4][0] && pressed[0][5] && pressed[0][0] {
                    reset_to_usb_boot(0, 0);
                }
                for ri in 0..5 {
                    for ci in 0..6 {
                        tot_pressed[ri][5 - ci] = pressed[ri][ci];
                    }
                }
                let mut holds = Vec::with_capacity(8);
                let mut actions = Vec::with_capacity(8);
                kblogic.update(&tot_pressed, &timer, &mut holds, &mut actions);
                for pressed in actions {
                    keyboard
                        .device()
                        .write_report(pressed.iter().copied().chain(holds.iter().copied()))
                        .ok();
                }
                // while !actions.is_empty() {
                //     let action = actions.pop();
                // }

                // if let Some(prev_pressed) = prev_pressed {
                //     for ri in 0..5 {
                //         for ci in 0..12 {
                //             if prev_pressed[ri][ci] && !tot_pressed[ri][ci] {
                //                 println!("released {:?}", (ri, ci));
                //             } else if !prev_pressed[ri][ci] && tot_pressed[ri][ci] {
                //                 println!("pressed {:?}", (ri, ci));
                //             }
                //         }
                //     }
                // }
                // prev_pressed = Some(tot_pressed);
            }
        }

        if tick_count_down.wait().is_ok() {
            match keyboard.tick() {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => {}
                Err(_) => {
                    // error!("Sending tick");
                }
            }
        }

        if usb_dev.poll(&mut [&mut keyboard]) {
            match keyboard.device().read_report() {
                Err(UsbError::WouldBlock) => {}
                Err(_e) => {
                    // error!("Failed to read keyeboard report");
                }
                Ok(_leds) => {}
            }
        }
    }
}
