use sigrok::Sigrok;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ctx = Sigrok::new()?;
    for driver in ctx.drivers() {
        println!(
            "- {:?}: {} v{}",
            driver.name(),
            driver.long_name(),
            driver.api_version()
        );
    }
    Ok(())
}
