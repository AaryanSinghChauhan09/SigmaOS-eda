//! SigmaOS component binary: eda

use sigmaos_eda::Component;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Component::new().operate()?;
    println!("sigmaos-eda: ok");
    Ok(())
}
