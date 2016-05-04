extern crate sigrok_sys;
extern crate glib_sys;

use sigrok_sys::{Struct_sr_context, sr_init, sr_exit, sr_driver_list, Struct_sr_dev_driver};
use sigrok_sys::{sr_dev_list, sr_driver_init, sr_driver_scan, Struct_sr_dev_inst};
use sigrok_sys::{sr_dev_inst_channels_get, Struct_sr_channel, sr_output_find};
use sigrok_sys::{Struct_sr_output_module, sr_output_new, sr_session_new, Struct_sr_session};
use sigrok_sys::{sr_session_datafeed_callback_add, Struct_sr_datafeed_packet, sr_session_dev_add};
use sigrok_sys::{sr_dev_channel_enable};
use std::mem;
use std::io;
use std::ffi::{CStr, CString};
use std::os;
use glib_sys::{GSList, GHashTable, g_main_loop_new, g_main_loop_run};

#[derive(Debug)]
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

    pub fn drivers(&self) -> Vec<Driver> {
        unsafe {
            let mut driver_list: *mut *mut Struct_sr_dev_driver = sr_driver_list(self.context);
            let mut drivers = vec![];
            while (*driver_list) as usize != 0x0 {
                drivers.push(Driver {
                    context: *driver_list
                });
                driver_list = ((driver_list as usize) + mem::size_of::<*mut Struct_sr_dev_driver>()) as *mut *mut Struct_sr_dev_driver;
            }
            drivers
        }
    }

    pub fn init_driver(&self, driver: &Driver) -> Option<DriverContext> {
        unsafe {
            let _ = sr_driver_init(self.context, driver.context);
        }
        Some(DriverContext {
            driver: driver.clone()
        })
    }
}

#[derive(Debug, Clone)]
pub struct Driver {
    context: *mut Struct_sr_dev_driver,
}

impl Driver {
    pub fn name(&self) -> String {
        unsafe {
            CStr::from_ptr((*self.context).name).to_string_lossy().into_owned()
        }
    }

    pub fn long_name(&self) -> String {
        unsafe {
            CStr::from_ptr((*self.context).longname).to_string_lossy().into_owned()
        }
    }

    pub fn api_version(&self) -> i32 {
        unsafe {
            (*self.context).api_version as i32
        }
    }

    // pub fn dev_list(&self) -> Option<()> {
    //     unsafe {
    //         let gslist = sr_dev_list(self.context);
    //         if (gslist as usize) == 0x0 {
    //             None
    //         } else {
    //             Some(())
    //         }
    //     }
    // }
}

#[derive(Debug)]
pub struct DriverContext {
    driver: Driver,
}

impl DriverContext {
    pub fn scan(&self) -> Vec<DriverInstance> {
        unsafe {
            let gslist = sr_driver_scan(self.driver.context, 0x0 as *mut glib_sys::GSList);
            self.enumerate_devices(gslist)
        }
    }

    pub fn devices(&self) -> Vec<DriverInstance> {
        unsafe {
            let gslist = sr_dev_list(self.driver.context);
            self.enumerate_devices(gslist)
        }
    }

    fn enumerate_devices(&self, mut gslist: *mut GSList) -> Vec<DriverInstance> {
        let mut instances = vec![];
        unsafe {
            loop {
                if (gslist as usize) == 0x0 {
                    break;
                }
                instances.push(DriverInstance {
                    context: (*gslist).data as *mut Struct_sr_dev_inst,
                });
                gslist = (*gslist).next;
            }
        }
        instances
    }
}

#[derive(Debug)]
pub struct DriverInstance {
    context: *mut Struct_sr_dev_inst,
}

impl DriverInstance {
    pub fn channels(&self) -> Vec<DriverChannel> {
        let mut channels = vec![];
        unsafe {
            let mut gslist = sr_dev_inst_channels_get(self.context);
            loop {
                if (gslist as usize) == 0x0 {
                    break;
                }
                channels.push(DriverChannel {
                    context: (*gslist).data as *mut Struct_sr_channel,
                });
                gslist = (*gslist).next;
            }
        }
        channels
    }

    // pub fn output(&self, output: &Output) {
    //     unsafe {
    //         let output = sr_output_new(output.context, 0x0 as *mut glib_sys::GHashTable, self.context, 0x0 as *const i8);
    //
    //     }
    // }
}

#[derive(Debug)]
pub struct DriverChannel {
    context: *mut Struct_sr_channel,
}

impl DriverChannel {
    pub fn index(&self) -> u32 {
        unsafe {
            (*self.context).index as u32
        }
    }

    pub fn name(&self) -> String {
        unsafe {
            CStr::from_ptr((*self.context).name).to_string_lossy().into_owned()
        }
    }

    pub fn enable(&self) {
        unsafe {
            let _ = sr_dev_channel_enable(self.context, 1);
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

#[derive(Debug)]
struct Session {
    context: *mut Struct_sr_session,
}

unsafe extern "C" fn sr_session_callback(inst: *const Struct_sr_dev_inst, packet: *const Struct_sr_datafeed_packet, data: *mut os::raw::c_void) {
    // TODO
}

type SesCall = Fn(&DriverInstance, *const Struct_sr_datafeed_packet);

fn ses_call(driver: &DriverInstance, packet: *const Struct_sr_datafeed_packet) {
    println!("got callback!");
}

impl Session {
    fn new(ctx: &mut Sigrok) -> Option<Session> {
        unsafe {
            let mut session = Session {
                context: mem::uninitialized(),
            };
            if sr_session_new(ctx.context, &mut session.context as *mut _) == 0x0 {
                Some(session)
            } else {
                None
            }
        }
    }

    fn callback_add(&self, mut callback: Box<SesCall>) {
        unsafe {
            let _ = sr_session_datafeed_callback_add(self.context, Some(sr_session_callback), &mut callback as *mut _ as *mut _);
        }
    }

    fn add_instance(&self, instance: &DriverInstance) {
        unsafe {
            let _ = sr_session_dev_add(self.context, instance.context);
        }
    }
}


// #[derive(Debug)]
// pub struct Output {
//     context: *const Struct_sr_output_module,
// }
// 
// fn output_find(tag: &str) -> Option<Output> {
//     unsafe {
//         let mut cstr = CString::new(tag).unwrap().into_bytes();
//         let ptr = sr_output_find((&mut cstr[0]) as *mut u8 as *mut i8);
//         if (ptr as usize) == 0x0 {
//             None
//         } else {
//             Some(Output {
//                 context: ptr,
//             })
//         }
//     }
// }

#[test]
fn it_works() {
    let mut ctx = Sigrok::new().unwrap();
    for driver in ctx.drivers() {
        println!("- {:?}: {} v{}", driver.name(), driver.long_name(), driver.api_version());
    }

    let mut ses = Session::new(&mut ctx).unwrap();
    ses.callback_add(Box::new(ses_call));

    if let Some(driver) = ctx.drivers().iter().find(|x| x.name() == "demo") {
        println!("demo {:?}", driver);
        let demo = ctx.init_driver(driver).unwrap();
        demo.scan();
        for device in demo.devices() {
            ses.add_instance(&device);

            for chan in device.channels() {
                println!("channel {:?}", chan.name());
                chan.enable();
            }

            //select_channels
            // device.output(&output_find("binary").unwrap());
        }

        unsafe {
            let main_loop = g_main_loop_new(0x0 as *mut _, 0);

            g_main_loop_run(main_loop);
        }
    }
}
