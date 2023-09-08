use alloc::vec::Vec;
use rp_pico::hal::{timer::Instant, Timer};
use usbd_human_interface_device::page::Keyboard;

use crate::hardware::serial::println;

pub enum Key {
    Press(Keyboard),
    Empty,
}

mod layout {
    use super::Key::Press as PR;
    use super::Key::{self, Empty, Press};
    use usbd_human_interface_device::page::Keyboard;
    use usbd_human_interface_device::page::Keyboard::*;

    #[rustfmt::skip]
    pub const LAYOUT: [[Key; 12]; 5] = [
        [ Empty, PR(Q), PR(W), PR(F), PR(P), PR(G), PR(J), PR(L), PR(U), PR(Y), PR(Semicolon), PR(DeleteBackspace), ],
        [ PR(Escape), PR(A), PR(R), PR(F), PR(P), PR(G), PR(J), PR(L), PR(U), PR(Y), PR(Semicolon), PR(DeleteBackspace), ],
        [ Empty, PR(Q), PR(W), PR(F), PR(P), PR(G), PR(J), PR(L), PR(U), PR(Y), PR(Semicolon), PR(DeleteBackspace), ],
        [ Empty, PR(Q), PR(W), PR(F), PR(P), PR(G), PR(J), PR(L), PR(U), PR(Y), PR(Semicolon), PR(DeleteBackspace), ],
        [ Empty, PR(Q), PR(W), PR(F), PR(P), PR(G), PR(J), PR(L), PR(U), PR(Y), PR(Semicolon), PR(DeleteBackspace), ],
    ];
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonState {
    pressed: bool,
    t_change: Instant,
}

pub struct KeyboardLogic {
    prev_pressed: [[ButtonState; 12]; 5],
}

impl KeyboardLogic {
    pub fn new(timer: &Timer) -> Self {
        let t = timer.get_counter();
        KeyboardLogic {
            prev_pressed: [[ButtonState {
                pressed: false,
                t_change: t,
            }; 12]; 5],
        }
    }

    pub fn update(&mut self, new_state: &[[bool; 12]; 5], actions: &mut Vec<Keyboard>) {
        for ri in 0..5 {
            for ci in 0..12 {
                if new_state[ri][ci] != self.prev_pressed[ri][ci].pressed {
                    if let Key::Press(key) = layout::LAYOUT[ri][ci] {
                        actions.push(key);
                    }
                    // println!("{:?}", (ri, ci));
                    self.prev_pressed[ri][ci].pressed = new_state[ri][ci];
                }
            }
        }
    }
}
