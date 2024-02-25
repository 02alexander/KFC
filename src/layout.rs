use alloc::vec::Vec;
use rp_pico::hal::{timer::Instant, Timer};
use smallvec::SmallVec;
use usbd_human_interface_device::page::Keyboard;

use self::layout::LAYOUT;

const ROWS: usize = 5;
const COLS: usize = 12;

#[derive(Clone, Copy, PartialEq)]
pub enum Key {
    Press(Keyboard),
    LayerChange(u8),
    OnClick(Keyboard, Keyboard, u64),
    Combo(Keyboard, Keyboard),
    Hold(Keyboard),
    Drop,
    Empty,
}

mod layout {
    #![allow(non_upper_case_globals)]

    use super::Key::Press as PR;
    use super::Key::{self, Drop, Empty, Hold, LayerChange, OnClick};
    // use usbd_human_interface_edvice::page::Keyboard;
    use super::Key::Combo as CB;
    use usbd_human_interface_device::page::Keyboard::LeftShift as LS;
    use usbd_human_interface_device::page::Keyboard::*;

    const Exclamation: Key = CB(LS, Keyboard1);
    const At: Key = CB(LS, Keyboard2);
    const Octohorp: Key = CB(LS, Keyboard3);
    const Dollar: Key = CB(LS, Keyboard4);
    const Percent: Key = CB(LS, Keyboard5);
    const Exponent: Key = CB(LS, Keyboard6);
    const Ampersand: Key = CB(LS, Keyboard7);
    const Mul: Key = CB(LS, Keyboard8);
    const LeftPar: Key = CB(LS, Keyboard9);
    const RightPar: Key = CB(LS, Keyboard0);
    const Underscore: Key = CB(LS, Minus);
    const LeftCurly: Key = CB(LS, LeftBrace);
    const RightCurly: Key = CB(LS, RightBrace);
    const Bar: Key = CB(LS, Backslash);

    #[rustfmt::skip]
    pub const LAYOUT: [[[Key; 12]; 5]; 4] = [
        [
            [ Empty, PR(Q), PR(W), PR(F), PR(P), PR(G), PR(J), PR(L), PR(U), PR(Y), PR(Semicolon), PR(DeleteBackspace), ],
            [ OnClick(Escape, LeftShift, 150), PR(A), PR(R), PR(S), PR(T), PR(D), PR(H), PR(N), PR(E), PR(I), PR(O), PR(Apostrophe), ],
            [ Hold(LeftControl), PR(Z), PR(X), PR(C), PR(V), PR(B), PR(K), PR(M), PR(Comma), PR(Dot), PR(ForwardSlash), PR(ReturnEnter), ],
            [ Empty, Empty, Empty, Empty, LayerChange(2), PR(Space), Hold(RightShift), LayerChange(1), Empty, Empty, Empty, Empty, ],
            [ Empty, Empty, Empty, Empty, Empty, Hold(LeftGUI), Hold(LeftAlt), Empty, Empty, Empty, Empty, Empty, ],
        ],        
        [
            [ Drop, At, Percent, PR(Grave), Octohorp, PR(LeftBrace), PR(RightBrace), PR(Keyboard7), PR(Keyboard8), PR(Keyboard9), PR(KeypadAdd), Drop],
            [ Bar, Underscore, Ampersand, Mul, PR(Equal), LeftPar, RightPar, PR(Keyboard4), PR(Keyboard5), PR(Keyboard6), PR(Keyboard0), Drop,],
            [ Drop, Exponent, PR(Backslash), Exclamation, Dollar, LeftCurly, RightCurly, PR(Keyboard1), PR(Keyboard2), PR(Keyboard3), PR(KeypadSubtract), Drop, ],
            [ Empty, Empty, Empty, Empty, Drop, Drop, Drop, Drop, Empty, Empty, Empty, Empty, ],
            [ Empty, Empty, Empty, Empty, Empty, Drop, Drop, Empty, Empty, Empty, Empty, Empty, ],
        ],

        [
            [ Drop, PR(F1), PR(F2), PR(F3), PR(F4), PR(F5), PR(F10), PR(F11), PR(F12), PR(PrintScreen), Empty, Drop, ],
            [ Drop, CB(Q, RightAlt), CB(W, RightAlt), CB(P, RightAlt), Hold(RightShift), PR(F6), PR(F9), PR(LeftArrow), PR(DownArrow), PR(UpArrow), PR(RightArrow), Drop, ],
            [ Drop, Empty, CB(LeftControl, Tab), PR(Tab), Empty, PR(F7), PR(F8), PR(Home), PR(PageDown), PR(PageUp), PR(End), Drop, ],
            [ Drop, Empty, Empty, Empty, Drop, Drop, Drop, Drop, Empty, Empty, Empty, Empty, ],
            [ Empty, Empty, Empty, Empty, Empty, Drop, Drop, Empty, Empty, Empty, Empty, Empty, ],
        ],

        [
            [ Drop, PR(F1), PR(F2), PR(F3), PR(F4), PR(F5), PR(F10), PR(F11), PR(F12), PR(PrintScreen), Empty, Drop, ],
            [ Drop, CB(Q, RightAlt), CB(W, RightAlt), CB(P, RightAlt), PR(RightShift), PR(F6), PR(F9), PR(LeftArrow), PR(DownArrow), PR(UpArrow), PR(RightArrow), Drop, ],
            [ Drop, Empty, CB(LeftControl, Tab), PR(Tab), Empty, PR(F7), PR(F8), PR(Home), PR(PageDown), PR(PageUp), PR(End), Drop, ],
            [ Drop, Empty, Empty, Empty, Drop, Drop, Drop, Drop, Empty, Empty, Empty, Empty, ],
            [ Empty, Empty, Empty, Empty, Empty, Drop, Drop, Empty, Empty, Empty, Empty, Empty, ],
        ],
        
    ];
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonState {
    pressed: bool,
    t_change: Instant,
    pressed_layer: u8,
}

pub struct KeyboardLogic {
    prev_pressed: [[ButtonState; COLS]; ROWS],
    t_last_key_sent: Instant,
}

impl KeyboardLogic {
    pub fn new(timer: &Timer) -> Self {
        let t = timer.get_counter();
        KeyboardLogic {
            prev_pressed: [[ButtonState {
                pressed: false,
                t_change: t,
                pressed_layer: 0,
            }; COLS]; ROWS],
            t_last_key_sent: t,
        }
    }

    pub fn update(
        &mut self,
        new_state: &[[bool; COLS]; ROWS],
        timer: &Timer,
        holds: &mut Vec<Keyboard>, // To be sent along with all keypresses.
        actions: &mut Vec<SmallVec<[Keyboard; 4]>>,
    ) {
        let mut current_layer: usize = 0;

        let mut normal_presses = SmallVec::new();
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
                    normal_presses.push(Keyboard::Q);
                    continue;
                }
                let cur_pressed = new_state[ri][ci];
                let prev_button_state = &mut self.prev_pressed[ri][ci];
                let t = timer.get_counter();

                // So that if the layer is changed while any key is pressed it won't automatically
                // press the corresponding key in the new layer.
                if cur_pressed != prev_button_state.pressed {
                    prev_button_state.pressed_layer = used_layer[ri][ci]
                }

                if prev_button_state.pressed_layer == used_layer[ri][ci] {
                    match LAYOUT[used_layer[ri][ci] as usize][ri][ci] {
                        Key::Press(key) => {
                            if cur_pressed {
                                normal_presses.push(key);
                                self.t_last_key_sent = t;
                            }
                        }
                        Key::Combo(k1, k2) => {
                            if cur_pressed && !prev_button_state.pressed {
                                let mut sv = SmallVec::new();
                                sv.push(k1);
                                sv.push(k2);
                                actions.push(sv);
                                self.t_last_key_sent = t;
                            }
                        }
                        Key::Empty => {}
                        Key::Hold(key) => {
                            if cur_pressed {
                                holds.push(key);
                            }
                        }
                        Key::OnClick(click_key, hold_mod, ms) => {
                            if cur_pressed {
                                holds.push(hold_mod);
                            } else if !cur_pressed
                                && prev_button_state.pressed
                                && (t - prev_button_state.t_change).ticks() < ms * 1000
                                && prev_button_state.t_change.ticks()
                                    >= self.t_last_key_sent.ticks()
                            {
                                normal_presses.push(click_key);
                                self.t_last_key_sent = t;
                            }
                        }
                        Key::Drop => {}
                        Key::LayerChange(_) => {}
                    }
                }
                if cur_pressed != prev_button_state.pressed {
                    prev_button_state.t_change = t;
                }
                prev_button_state.pressed = new_state[ri][ci];
            }
        }

        actions.push(normal_presses);
    }
}
