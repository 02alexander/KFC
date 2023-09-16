use alloc::vec::Vec;
use rp_pico::hal::{timer::Instant, Timer};
use usbd_human_interface_device::page::Keyboard;

use crate::hardware::serial::println;

use self::layout::LAYOUT;

const ROWS: usize = 5;
const COLS: usize = 12;

#[derive(Clone, Copy, PartialEq)]
pub enum Key {
    Press(Keyboard),
    LayerChange(u8),
    Combo(Keyboard, Keyboard),
    Hold(Keyboard),
    Drop,
    Empty,
}

mod layout {
    use super::Key::Press as PR;
    use super::Key::{self, Empty, LayerChange, Combo, Drop, Hold};
    // use usbd_human_interface_edvice::page::Keyboard;
    use usbd_human_interface_device::page::Keyboard::*;

    #[rustfmt::skip]
    pub const LAYOUT: [[[Key; 12]; 5]; 4] = [
        [
            [ Empty, PR(Q), PR(W), PR(F), PR(P), PR(G), PR(J), PR(L), PR(U), PR(Y), PR(Semicolon), PR(DeleteBackspace), ],
            [ PR(Escape), PR(A), PR(R), PR(S), PR(T), PR(D), PR(H), PR(N), PR(E), PR(I), PR(O), PR(Apostrophe), ],
            [ Hold(LeftControl), PR(Z), PR(X), PR(C), PR(V), PR(B), PR(K), PR(M), PR(Comma), PR(Dot), PR(ForwardSlash), PR(ReturnEnter), ],
            [ Empty, Empty, Empty, Empty, LayerChange(2), PR(Space), Hold(RightShift), LayerChange(1), Empty, Empty, Empty, Empty, ],
            [ Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, ],
        ],        
        [
            [ Empty, PR(Q), PR(W), PR(F), PR(P), PR(G), PR(J), PR(Keyboard7), PR(Keyboard8), PR(Keyboard9), PR(KeypadAdd), PR(DeleteBackspace), ],
            [ PR(Escape), PR(A), PR(R), PR(S), PR(T), PR(D), PR(H), PR(Keyboard4), PR(Keyboard5), PR(Keyboard6), PR(Keyboard0), PR(Apostrophe), ],
            [ Hold(LeftControl), PR(Z), PR(X), PR(C), PR(V), PR(B), PR(K), PR(Keyboard1), PR(Keyboard2), PR(Keyboard3), PR(KeypadSubtract), PR(ReturnEnter), ],
            [ Empty, Empty, Empty, Empty, Drop, Drop, Drop, Drop, Empty, Empty, Empty, Empty, ],
            [ Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, ],
        ],

        [
            [ Drop, PR(F1), PR(F2), PR(F3), PR(F4), PR(F5), PR(F10), PR(F11), PR(F12), PR(PrintScreen), Empty, Drop, ],
            [ Drop, Combo(Q, RightAlt), Combo(W, RightAlt), Combo(P, RightAlt), Hold(RightShift), PR(F6), PR(F9), PR(LeftArrow), PR(DownArrow), PR(UpArrow), PR(RightArrow), Drop, ],
            [ Drop, Empty, Combo(LeftControl, Tab), PR(Tab), PR(F7), PR(F8), PR(Home), PR(PageDown), PR(PageUp), PR(End), PR(ForwardSlash), Drop, ],
            [ Drop, Empty, Empty, Empty, Drop, Drop, Drop, Drop, Empty, Empty, Empty, Empty, ],
            [ Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, ],
        ],

        [
            [ Drop, PR(F1), PR(F2), PR(F3), PR(F4), PR(F5), PR(F10), PR(F11), PR(F12), PR(PrintScreen), Empty, Drop, ],
            [ Drop, Combo(Q, RightAlt), Combo(W, RightAlt), Combo(P, RightAlt), PR(RightShift), PR(F6), PR(F9), PR(LeftArrow), PR(DownArrow), PR(UpArrow), PR(RightArrow), Drop, ],
            [ Drop, Empty, Combo(LeftControl, Tab), PR(Tab), PR(F7), PR(F8), PR(Home), PR(PageDown), PR(PageUp), PR(End), PR(ForwardSlash), Drop, ],
            [ Drop, Empty, Empty, Empty, Drop, Drop, Drop, Drop, Empty, Empty, Empty, Empty, ],
            [ Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty, ],
        ],
        
    ];
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonState {
    pressed: bool,
    t_change: Instant,
}

pub struct KeyboardLogic {
    prev_pressed: [[ButtonState; COLS]; ROWS],
}

impl KeyboardLogic {
    pub fn new(timer: &Timer) -> Self {
        let t = timer.get_counter();
        KeyboardLogic {
            prev_pressed: [[ButtonState {
                pressed: false,
                t_change: t,
            }; COLS]; ROWS],
        }
    }

    pub fn update(&mut self, new_state: &[[bool; COLS]; ROWS], actions: &mut Vec<Keyboard>) {
        let mut current_layer: usize = 0;

        for ri in 0..ROWS {
            for ci in 0..COLS {
                if new_state[ri][ci] {
                    if let Key::LayerChange(n) = layout::LAYOUT[0][ri][ci] {
                        current_layer += n as usize;
                    }    
                }
            }
        }

        let mut used_layer = [[current_layer as u8; COLS]; ROWS];
        for ri in 0..ROWS {
            for ci in 0..COLS {
                let mut cur_layer = current_layer;
                while cur_layer > 0 && Key::Drop == layout::LAYOUT[cur_layer][ri][ci] {
                    cur_layer -= 1;
                } 
                used_layer[ri][ci] = cur_layer as u8;
            }
        }


        for ri in 0..ROWS {
            for ci in 0..COLS {
                if used_layer[ri][ci] as usize > LAYOUT.len() {
                    actions.push(Keyboard::Q);
                    continue;
                }
                match LAYOUT[used_layer[ri][ci] as usize][ri][ci] {
                    Key::Press(key) => {
                        if new_state[ri][ci] && !self.prev_pressed[ri][ci].pressed {
                            actions.push(key);
                        }
                    }
                    Key::Combo(_, _) => {},
                    Key::Empty => {},
                    Key::Hold(key) => {
                        if new_state[ri][ci] {
                            actions.push(key);
                        }
                    },
                    Key::Drop => {},
                    Key::LayerChange(_) => {},
                }
                self.prev_pressed[ri][ci].pressed = new_state[ri][ci];
            }
        }
    }
}
