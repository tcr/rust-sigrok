# rust-sigrok

High level bindings for libsigrok in Rust.

```
sigrok = "0.3"
```

## Example

```rust
use sigrok::{Sigrok, Session, DriverInstance, Datafeed};

// Create sigrok and session.
let mut ctx = Sigrok::new().unwrap();
let mut ses = Session::new(&mut ctx).unwrap();

// Get demo driver.
if let Some(driver) = ctx.drivers().iter().find(|x| x.name() == "demo") {
    // Initialize driver.
    let demo = ctx.init_driver(driver).unwrap();

    // Scan for devices.
    demo.scan();
    for device in demo.devices() {
        // Attach device.
        ses.add_instance(&device);
    }

    // Register callback, start session and loop endlessly.
    ses.callback_add(Box::new(on_data));
    ses.start();
    main_loop();
}

fn on_data(_: &DriverInstance, data: &Datafeed) {
  match data {
      &Datafeed::Logic { unit_size, data } => {
          println!("Received {:?} bytes of {:?}-byte units.", data.len(), unit_size);
      }
      _ => { }
  }
}
```

## License

GPL-3.0
