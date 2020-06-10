use sigrok::config::{config_items, Configurable};
use sigrok::data::{Datafeed, Logic};
use sigrok::{Session, Sigrok};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Print out available drivers.
    let ctx = Sigrok::new()?;

    let ses = Session::new(&ctx)?;

    let driver = ctx
        .drivers()
        .into_iter()
        .find(|x| x.name() == "demo")
        .unwrap();

    // Initialize driver.
    let driver = driver.init()?;
    // Scan for devices.
    for device in driver.scan(None)? {
        // Attach device.
        ses.add_device(&device)?;
        device.config_set(config_items::LimitSamples, &64)?;

        // Set pattern mode on digital outputs.
        if let Some(group) = device.channel_groups().get(0) {
            group.config_set(config_items::PatternMode, "sigrok")?;
        }

        // Set sample rate.
        device.config_set(config_items::SampleRate, &1_000_000)?;
    }

    // Register callback, start session and loop endlessly.
    ses.start(None, |_, data| match data {
        Datafeed::Logic(Logic { unit_size, data }) => {
            let _ = unit_size;
            for byte in data {
                println!(
                    "{}",
                    format!("{:08b}", byte).replace("1", " ").replace("0", "█")
                );
            }
        }
        _ => {}
    })?;

    Ok(())
}
