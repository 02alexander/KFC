#![no_std]
#![no_main]

extern crate alloc;
mod buttonmatrix;
mod encoding;
mod hardware;
mod layout;
mod master;
mod slave;

fn start() -> ! {
    // Hardware setup.

    // slave::run()
    master::run()

    // if cfg!(slave) {
    //     slave::run()
    // } else {
    //     master::run();
    // }
}
