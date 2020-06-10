use crate::config::Configurable;
use crate::data::{Datafeed, Logic};
use crate::Trigger;
use crate::{config, Function, Session, Sigrok, TriggerType, Triggers};

#[test]
fn it_works() {
    // Print out available drivers.
    let ctx = Sigrok::new().unwrap();
    for driver in ctx.drivers() {
        println!(
            "- {:?}: {} v{}",
            driver.name(),
            driver.long_name(),
            driver.api_version()
        );
    }

    // Create session.
    let ses = Session::new(&ctx).unwrap();

    // Get demo driver.
    if let Some(driver) = ctx.drivers().iter().find(|x| x.name() == "demo") {
        dbg!(driver);

        // Initialize driver.
        assert_eq!(
            &driver.functions().unwrap(),
            &[
                Function::DemoDev,
                Function::LogicAnalyzer,
                Function::Oscilloscope
            ]
        );
        let demo = driver.init().unwrap();

        // Scan for devices.
        let mut triggers = vec![];
        for device in demo.scan(None).unwrap() {
            dbg!(device.vendor());
            dbg!(device.model());
            dbg!(device.version());
            dbg!(device.serial_number());
            dbg!(device.conn_id());
            dbg!(device.config_options().unwrap());
            let channels = [
                "D0", "D1", "D2", "D3", "D4", "D5", "D6", "D7", "A0", "A1", "A2", "A3", "A4",
            ];
            assert!(device
                .channels()
                .iter()
                .map(|c| c.name())
                .eq(channels.iter().copied()));
            // Attach device.
            ses.add_device(&device).unwrap();
            device
                .config_set(config::config_items::LimitSamples, &64)
                .unwrap();

            // Set pattern mode on digital outputs.
            if let Some(group) = device.channel_groups().get(0) {
                group
                    .config_set(config::config_items::PatternMode, "sigrok")
                    .unwrap();
            }

            for c in device
                .channels()
                .into_iter()
                .filter(|c| c.name() == "D0" || c.name() == "D1")
            {
                triggers.push(Trigger {
                    channel: c.clone(),
                    trigger_match: TriggerType::Falling,
                    value: 0.0,
                });
            }

            // Set sample rate.
            for group in device.channel_groups() {
                dbg!(group.name());
                dbg!(group.config_options().unwrap());
                device
                    .config_set(config::config_items::SampleRate, &1_000_000)
                    .unwrap();
            }
        }

        // Register callback, start session and loop endlessly.
        ses.start_with_cancel(
            Some(&Triggers::new(&mut [triggers.iter()]).unwrap()),
            |_| {},
            |_, data| match data {
                Datafeed::Logic(Logic { unit_size, data }) => {
                    let _ = unit_size;
                    for byte in data {
                        println!(
                            "{}",
                            format!("{:08b}", byte).replace("1", " ").replace("0", "█")
                        );
                    }
                    // ses.stop();
                }
                Datafeed::Trigger => println!("Trigger!"),
                _ => {}
            },
        )
        .unwrap();
    }
}
