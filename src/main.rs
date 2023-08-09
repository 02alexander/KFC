#![no_std]
#![no_main]

extern crate alloc;
mod hardware;

use bitvec::prelude::*;
use alloc::{format, vec::Vec};
use fugit::ExtU32;
use cortex_m::delay::Delay;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::{digital::v2::{OutputPin, InputPin}, timer::CountDown};
use panic_probe as _;

use rp_pico as bsp;

use bsp::{
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        sio::Sio,
        watchdog::Watchdog,
        Timer, gpio::DynPin, rom_data::reset_to_usb_boot,
    },
    pac::{CorePeripherals, watchdog::tick},
    pac::Peripherals,
    Pins,
};
use usb_device::{prelude::{UsbDeviceBuilder, UsbVidPid}, class_prelude::UsbBusAllocator, UsbError};
use usbd_human_interface_device::{page::Keyboard, usb_class::UsbHidClassBuilder, interface::{InterfaceBuilder, InBytes8, OutBytes8, ReportSingle}, device::keyboard::{BootKeyboardReport, NKROBootKeyboardConfig}, UsbHidError};

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


fn start() -> ! {


    // info!("Program start");
    // Hardware setup.
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

    // let inp_pin = pins.gpio21.into_pull_down_input();
    // let mut out_pin = pins.gpio20.into_push_pull_output();
    // loop {

    //     out_pin.set_high().unwrap();
    //     hardware::serial::println!("{:?}", inp_pin.is_high());
    //     delay.delay_ms(1);
    //     out_pin.set_low().unwrap();
    //     hardware::serial::println!("{:?}\n", inp_pin.is_high());


    //     delay.delay_ms(300);
    // }


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
        DynPin::from(pins.gpio12),
        DynPin::from(pins.gpio13),
        DynPin::from(pins.gpio7),
        DynPin::from(pins.gpio9),
    ];
    cols.iter_mut().for_each(|p| p.into_pull_down_input());


    let mut input_count_down = timer.count_down();
    input_count_down.start(500.millis());

    let mut tick_count_down = timer.count_down();
    tick_count_down.start(500.micros());

    let mut scan_count_down = timer.count_down();
    scan_count_down.start(500.micros());

    // let mut pressed = false;

    let mut scan = || -> [[bool; 6]; 5] {
        let mut pressed = [[false; 6]; 5];

        for (ri, row_pin) in rows.iter_mut().enumerate() {
            for (ci, col_pin) in cols.iter_mut().enumerate() {
                row_pin.set_high().unwrap();
                delay.delay_ms(10);
                
                if col_pin.is_high().unwrap() {
                    pressed[ri][ci] = true;
                }

                row_pin.set_low().unwrap();

            }
        }
        pressed
    };

    let mut prev_pressed: Option<[[bool; 6]; 5]> = None;
    loop {

        if scan_count_down.wait().is_ok() {
            let pressed = scan();
            
            if let Some(p) = prev_pressed {
                #[cfg(debug_assertions)]
                {
                    for ri in 0..5 {
                        for ci in 0..6 {
                            if pressed[ri][ci] && !p[ri][ci] {
                                hardware::serial::println!("Pressed {} {}", ri, ci);
                                if ri == 4 && ci == 0 {
                                    reset_to_usb_boot(0, 0);
                                }
                            } else if !pressed[ri][ci] && p[ri][ci] {
                                hardware::serial::println!("Released {} {}", ri, ci);
                            }
                        }
                    }
                }    
            }
            prev_pressed = Some(pressed);
        }

        // if input_count_down.wait().is_ok() {
        //     let key = if pressed {
        //         Keyboard::A
        //     } else {
        //         Keyboard::B
        //     };
        //     pressed = !pressed;
        //     let keys = [key];
        //     match keyboard.device().write_report(keys) {
        //         Ok(()) => {
        //             info!("Succesfully sent key!");
        //         },
        //         Err(UsbHidError::Duplicate) => {
        //             info!("Duplicate");
        //         },
        //         Err(UsbHidError::WouldBlock) => {
        //             info!("WouldBlock");
        //         }
        //         Err(_e) => {
        //             error!("Sending keys.");
        //         }
        //     };
        // }

        
        #[cfg(not(debug_assertions))] 
        {
            if tick_count_down.wait().is_ok() {
                match keyboard.tick() {
                    Err(UsbHidError::WouldBlock) => {},
                    Ok(_) => {},
                    Err(_) => {
                        error!("Sending tick");
                    }
                }
            }

            if usb_dev.poll(&mut [&mut keyboard]) {
                match keyboard.device().read_report() {
                    Err(UsbError::WouldBlock) => {},
                    Err(_e) => {
                        error!("Failed to read keyeboard report");
                    },
                    Ok(_leds) => {
                    }
                }
            }
        }
    }

}



