extern crate sigrok_sys;

use sigrok_sys::{Struct_sr_context, sr_init};
use std::mem;

pub fn init() {
    unsafe {
        let mut ctx: *mut Struct_sr_context = mem::uninitialized();
        let res = sr_init(&mut ctx as *mut _);
    }
}

#[test]
fn it_works() {
    init();
}
