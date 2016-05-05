extern crate sigrok_sys;
extern crate glib_sys;
extern crate time;

use sigrok_sys::{Struct_sr_context, sr_init, sr_exit, sr_driver_list, Struct_sr_dev_driver};
use sigrok_sys::{sr_dev_list, sr_driver_init, sr_driver_scan, Struct_sr_dev_inst};
use sigrok_sys::{sr_dev_inst_channels_get, Struct_sr_channel, sr_output_find};
use sigrok_sys::{Struct_sr_output_module, sr_output_new, sr_session_new, Struct_sr_session};
use sigrok_sys::{sr_session_datafeed_callback_add, Struct_sr_datafeed_packet, sr_session_dev_add};
use sigrok_sys::{sr_dev_channel_enable, sr_session_start, Enum_sr_packettype, Struct_sr_datafeed_analog};
use sigrok_sys::{Struct_sr_datafeed_logic, Enum_sr_configkey, Struct_sr_channel_group};
use sigrok_sys::{sr_dev_inst_channel_groups_get, sr_config_set, Struct_sr_datafeed_header};
use std::mem;
use std::io;
use std::ffi::{CStr, CString};
use std::os;
use std::slice;
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
pub struct DriverChannelGroup {
    context: *mut Struct_sr_channel_group,
}

impl DriverChannelGroup {
    pub fn name(&self) -> String {
        unsafe {
            CStr::from_ptr((*self.context).name).to_string_lossy().into_owned()
        }
    }
}

#[derive(Debug)]
pub enum ConfigOption {
    PatternMode(String),
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

    pub fn channel_groups(&self) -> Vec<DriverChannelGroup> {
        let mut channels = vec![];
        unsafe {
            let mut gslist = sr_dev_inst_channel_groups_get(self.context);
            loop {
                if (gslist as usize) == 0x0 {
                    break;
                }
                channels.push(DriverChannelGroup {
                    context: (*gslist).data as *mut Struct_sr_channel_group,
                });
                gslist = (*gslist).next;
            }
        }
        channels
    }

    pub fn config_set(&self, group: &DriverChannelGroup, config: &ConfigOption) {
        unsafe {
            match config {
                &ConfigOption::PatternMode(ref value) => {
                    let gvar = glib_sys::g_variant_new_string(CString::new(value.as_bytes()).unwrap().as_ptr());
                    sr_config_set(self.context, group.context, Enum_sr_configkey::SR_CONF_PATTERN_MODE as u32, gvar);
                }
            }
        }
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

    pub fn disable(&self) {
        unsafe {
            let res = sr_dev_channel_enable(self.context, 0);
            println!("disabling: {:?}", res);
        }
    }

    pub fn enable(&self) {
        unsafe {
            let res = sr_dev_channel_enable(self.context, 1);
            println!("enabling: {:?}", res);
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

struct Session {
    context: *mut Struct_sr_session,
    _callbacks: Vec<Box<SesCall>>,
}

enum Datafeed<'a> {
    Header {
        feed_version: i32,
        start_time: time::Timespec,
    },
    Logic {
        unit_size: u32,
        data: &'a [u8],
    }
}

unsafe extern "C" fn sr_session_callback(inst: *const Struct_sr_dev_inst, packet: *const Struct_sr_datafeed_packet, data: *mut os::raw::c_void) {
    // See session.c in sigrok-cli line 186
    let kind = (*packet)._type;

    let cb: &Box<SesCall> = mem::transmute(data);
    let driver = DriverInstance {
        context: inst as *mut _,
    };

    if kind == (Enum_sr_packettype::SR_DF_HEADER as u16) {
        let header: *const Struct_sr_datafeed_header = (*packet).payload as usize as *const _;

        cb(&driver, &Datafeed::Header {
            feed_version: (*header).feed_version as i32,
            start_time: time::Timespec {
                sec: (*header).starttime.tv_sec as i64,
                nsec: ((*header).starttime.tv_usec as i32) * 1000,
            },
        });
    } else if kind == (Enum_sr_packettype::SR_DF_LOGIC as u16) {
        let logic: *const Struct_sr_datafeed_logic = (*packet).payload as usize as *const _;
        let parts = slice::from_raw_parts::<u8>((*logic).data as usize as *const _, (*logic).length as usize);

        cb(&driver, &Datafeed::Logic {
            unit_size: (*logic).unitsize as u32,
            data: parts,
        });
    } else if kind == (Enum_sr_packettype::SR_DF_ANALOG as u16) {
        // let analog: *const Struct_sr_datafeed_analog = (*packet).payload as usize as *const _;
        // println!("TODO: analog");
        // pub data: *mut ::std::os::raw::c_void,
        // pub num_samples: uint32_t,
        // pub encoding: *mut Struct_sr_analog_encoding,
        // pub meaning: *mut Struct_sr_analog_meaning,
        // pub spec: *mut Struct_sr_analog_spec,
    } else if kind == (Enum_sr_packettype::SR_DF_END as u16) {
        println!("TODO: end");
    } else if kind == (Enum_sr_packettype::SR_DF_META as u16) {
        println!("TODO: meta");
    } else if kind == (Enum_sr_packettype::SR_DF_TRIGGER as u16) {
        println!("TODO: trigger");
    } else if kind == (Enum_sr_packettype::SR_DF_ANALOG_OLD as u16) {
        println!("TODO: analog old");
    } else if kind == (Enum_sr_packettype::SR_DF_FRAME_BEGIN as u16) {
        println!("TODO: frame begin");
    } else if kind == (Enum_sr_packettype::SR_DF_FRAME_END as u16) {
        println!("TODO: frame end");
    }
}

type SesCall = Fn(&DriverInstance, &Datafeed);

impl Session {
    fn new(ctx: &mut Sigrok) -> Option<Session> {
        unsafe {
            let mut session = Session {
                context: mem::uninitialized(),
                _callbacks: vec![],
            };
            if sr_session_new(ctx.context, &mut session.context as *mut _) == 0x0 {
                Some(session)
            } else {
                None
            }
        }
    }

    fn callback_add(&mut self, mut callback: Box<SesCall>) {
        unsafe {
            self._callbacks.push(callback);
            let _ = sr_session_datafeed_callback_add(self.context, Some(sr_session_callback), mem::transmute(&self._callbacks[self._callbacks.len() - 1]));
        }
    }

    fn add_instance(&self, instance: &DriverInstance) {
        unsafe {
            let _ = sr_session_dev_add(self.context, instance.context);
        }
    }

    fn start(&self) {
        unsafe {
            sr_session_start(self.context);
        }
    }
}


fn main_loop() {
    unsafe {
        let main_loop = g_main_loop_new(0x0 as *mut _, 0);
        g_main_loop_run(main_loop);
    }
}

#[cfg(test)]
fn it_works_datafeed(driver: &DriverInstance, data: &Datafeed) {
    match data {
        &Datafeed::Logic { unit_size, data } => {
            for i in 0..32 {
                println!("{}", format!("{:08b}", data[i]).replace("1", ".").replace("0", "X"));
            }
            println!("");
        }
        _ => { }
    }
}

#[test]
fn it_works() {
    let mut ctx = Sigrok::new().unwrap();
    for driver in ctx.drivers() {
        println!("- {:?}: {} v{}", driver.name(), driver.long_name(), driver.api_version());
    }

    let mut ses = Session::new(&mut ctx).unwrap();
    ses.callback_add(Box::new(it_works_datafeed));

    if let Some(driver) = ctx.drivers().iter().find(|x| x.name() == "demo") {
        println!("demo {:?}", driver);
        let demo = ctx.init_driver(driver).unwrap();
        demo.scan();
        for device in demo.devices() {
            ses.add_instance(&device);

            // Set pattern mode on digital outputs.
            if let Some(group) = device.channel_groups().get(0) {
                device.config_set(&group, &ConfigOption::PatternMode("pattern".to_owned()));
            }
        }

        ses.start();
        main_loop();
    }
}
