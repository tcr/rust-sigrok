extern crate sigrok_sys;

use sigrok_sys::{Struct_sr_context, sr_init, sr_exit};
use std::mem;
use std::io;

pub struct Sigrok {
    context: *mut Struct_sr_context,
}

impl Sigrok {
    pub fn new() -> io::Result<Sigrok> {
        unsafe {
            let mut ctx: Sigrok = Sigrok {
                context: mem::uninitialized(),
            };
            let res = sr_init(&mut ctx.context as *mut _);
            if res == 0 {
                Ok(ctx)
            } else {
                Err(io::Error::new(io::ErrorKind::Interrupted, "Could not initialize context"))
            }
        }
    }
}

impl Drop for Sigrok {
    fn drop(&mut self) {
        unsafe {
            let res = sr_exit(self.context);
            if res == 0 {
                // noop
            } else {
                panic!("Failed on sigrok context destructor")
            }
        }
    }
}

#[test]
fn it_works() {
    let ctx = Sigrok::new().unwrap();
    let _ = ctx;
}
