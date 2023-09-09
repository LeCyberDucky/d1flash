use clap::Parser;
use color_eyre::Result;
use config::Config;
use rppal::gpio::Gpio;

use d1flash::interface::{self, OpenDrainPin, OpenDrainState, Recipe};

fn main() -> Result<()> {
    let cli = interface::Cli::parse();
    let config: interface::Configuration = Config::builder()
        .add_source(config::File::from(cli.config_path))
        .build()?
        .try_deserialize()?;

    if !config.recipes.contains_key(&config.default_recipe) {
        color_eyre::eyre::bail!("The default recipe does not match any of the given recipes.");
    }

    // Configure pins
    let gpio = Gpio::new()?;
    let mut boot = OpenDrainPin::new(
        gpio.get(config.boot.pin)?,
        OpenDrainState::Open,
        config.boot.state,
    );
    let mut reset = OpenDrainPin::new(
        gpio.get(config.reset.pin)?,
        OpenDrainState::Open,
        config.reset.state,
    );

    // Reboot ESP into flash mode. This is necessary for both flashing and monitoring
    println!("Triggering boot mode pin (state: Low).");
    boot.set_low();
    std::thread::sleep(std::time::Duration::from_millis(20));

    println!("Triggering reset pin (state: Low).");
    reset.set_low();
    std::thread::sleep(std::time::Duration::from_millis(100));

    println!("Releasing reset pin (state: Open).");
    reset.set_open();
    std::thread::sleep(std::time::Duration::from_millis(100));

    std::thread::scope(|scope| {
        let reset_task = scope.spawn(|| {
            if let Some(reset_flag) = cli.reset {
                std::thread::sleep(std::time::Duration::from_millis(reset_flag.or(2000)));
                boot.set_open();
                println!("Triggering reset pin (state: Low).");
                reset.set_low();
                std::thread::sleep(std::time::Duration::from_millis(100));
    
                println!("Releasing reset pin (state: Open).");
                reset.set_open();
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        // Execute recipe
        let recipe = match cli.recipe.len() {
            0 => config.recipes[&config.default_recipe].clone(),
            1 if config.recipes.contains_key(&cli.recipe[0]) => {
                config.recipes[&cli.recipe[0]].clone()
            }
            _ => Recipe::from(cli.recipe).clone(),
        };
        println!("Executing {:?}", recipe);
        recipe.execute().expect("Meh");

        reset_task.join().expect("Meh");
    });

    // Reboot ESP into normal mode, if flash mode was entered previously
    println!("Releasing boot mode pin (state: Open).");
    boot.set_open();
    std::thread::sleep(std::time::Duration::from_millis(20));

    println!("Triggering reset pin (state: Low).");
    reset.set_low();

    println!("Releasing reset pin (state: Open).");
    std::thread::sleep(std::time::Duration::from_millis(100));
    reset.set_open();


    println!("Done!");
    Ok(())
}
