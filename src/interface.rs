use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::{ContextCompat, Result};
use rppal::gpio::{IoPin, Pin};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum Level {
    Low,
    High,
}

impl From<Level> for bool {
    fn from(value: Level) -> Self {
        match value {
            Level::Low => false,
            Level::High => true,
        }
    }
}

impl From<bool> for Level {
    fn from(value: bool) -> Self {
        match value {
            true => Self::High,
            false => Level::Low,
        }
    }
}

impl From<Level> for rppal::gpio::Level {
    fn from(value: Level) -> Self {
        match value {
            Level::Low => Self::Low,
            Level::High => Self::High,
        }
    }
}

impl From<rppal::gpio::Level> for Level {
    fn from(value: rppal::gpio::Level) -> Self {
        match value {
            rppal::gpio::Level::Low => Self::Low,
            rppal::gpio::Level::High => Self::High,
        }
    }
}

impl From<Level> for rppal::gpio::PullUpDown {
    fn from(value: Level) -> Self {
        match value {
            Level::Low => Self::PullDown,
            Level::High => Self::PullUp,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
enum Mode {
    Input,
    Output,
    Alt0,
    Alt1,
    Alt2,
    Alt3,
    Alt4,
    Alt5,
}

impl From<Mode> for rppal::gpio::Mode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Input => Self::Input,
            Mode::Output => Self::Output,
            Mode::Alt0 => Self::Alt0,
            Mode::Alt1 => Self::Alt1,
            Mode::Alt2 => Self::Alt2,
            Mode::Alt3 => Self::Alt3,
            Mode::Alt4 => Self::Alt4,
            Mode::Alt5 => Self::Alt5,
        }
    }
}

impl From<rppal::gpio::Mode> for Mode {
    fn from(value: rppal::gpio::Mode) -> Self {
        match value {
            rppal::gpio::Mode::Input => Self::Input,
            rppal::gpio::Mode::Output => Self::Output,
            rppal::gpio::Mode::Alt0 => Self::Alt0,
            rppal::gpio::Mode::Alt1 => Self::Alt1,
            rppal::gpio::Mode::Alt2 => Self::Alt2,
            rppal::gpio::Mode::Alt3 => Self::Alt3,
            rppal::gpio::Mode::Alt4 => Self::Alt4,
            rppal::gpio::Mode::Alt5 => Self::Alt5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OpenDrainState {
    Low,
    Open,
}

impl std::convert::From<OpenDrainState> for rppal::gpio::Mode {
    fn from(value: OpenDrainState) -> Self {
        match value {
            OpenDrainState::Low => Self::Output,
            OpenDrainState::Open => Self::Input,
        }
    }
}

// Simulating open-drain pin configuration by switching between input and low output
#[derive(Debug)]
pub struct OpenDrainPin {
    pin: IoPin,
    state: OpenDrainState,
    initial_state: PinState,
    final_state: PinDropState,
}

impl OpenDrainPin {
    pub fn new(pin: Pin, state: OpenDrainState, final_state: PinDropState) -> Self {
        // We disable the default drop behavior and handle it manually, such that the pin configuration can be maintained even after dropping
        let initial_state = PinState {
            mode: pin.mode().into(),
            level: pin.read().into(),
            pull: None, // Can't read pull up/down resistor configuration
        };
        let mut pin = pin.into_io(state.into());
        pin.set_reset_on_drop(false);
        let mut pin = Self {
            pin,
            state,
            initial_state,
            final_state,
        };
        pin.set(state);
        pin
    }

    pub fn set_low(&mut self) {
        // Ideally, we would like to set the logic level before changing the mode.
        // It is not clear whether this works as intended, so we set it both before and after, just to make sure
        self.pin.set_low();
        self.pin.set_mode(rppal::gpio::Mode::Output);
        self.pin.set_low();

        self.state = OpenDrainState::Low;
    }

    pub fn set_open(&mut self) {
        self.pin.set_mode(rppal::gpio::Mode::Input);
        self.pin.set_pullupdown(rppal::gpio::PullUpDown::PullUp);

        self.state = OpenDrainState::Open;
    }

    pub fn set(&mut self, state: OpenDrainState) {
        match state {
            OpenDrainState::Low => self.set_low(),
            OpenDrainState::Open => self.set_open(),
        }
    }
}

impl Drop for OpenDrainPin {
    fn drop(&mut self) {
        self.pin.set_mode(
            self.final_state
                .mode
                .unwrap_or(self.initial_state.mode)
                .into(),
        );
        self.pin.write(
            self.final_state
                .level
                .unwrap_or(self.initial_state.level)
                .into(),
        );
        if let Some(pull) = self.final_state.pull {
            self.pin.set_pullupdown(pull.into());
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PinState {
    mode: Mode,
    level: Level,
    pull: Option<Level>,
}

#[derive(Debug, Deserialize)]
pub struct PinDropState {
    mode: Option<Mode>,
    level: Option<Level>,
    pull: Option<Level>,
}

#[derive(Debug, Deserialize)]
pub struct PinConfig {
    pub pin: u8,
    pub state: PinDropState,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Recipe {
    pub command: String,
    pub arguments: Vec<String>,
}

impl Recipe {
    pub fn new(command: String, arguments: Vec<String>) -> Self {
        Self { command, arguments }
    }

    pub fn execute(&self) -> std::io::Result<std::process::ExitStatus> {
    // pub fn execute(&self) -> std::io::Result<std::process::Child> {
        // std::process::Command::new(&self.command)
        //     .args(&self.arguments)
        //     .stdout(std::process::Stdio::piped())
        //     .stderr(std::process::Stdio::piped())
        //     .spawn()

        std::process::Command::new(&self.command)
            .args(&self.arguments)
            .status()
    }
}

impl From<Vec<String>> for Recipe {
    fn from(value: Vec<String>) -> Self {
        let command = if value.is_empty() {
            String::new()
        } else {
            value[0].clone()
        };

        let arguments = (value[1..]).to_owned();

        Self::new(command, arguments)
    }
}

// Perform reset on drop and then set the Some settings afterwards
#[derive(Debug, Deserialize)]
// https://toml.io/en/
pub struct Configuration {
    pub boot: PinConfig, // GPIO pin on the Raspberry Pi connected to D3 on the MCU (for boot configuration)
    pub reset: PinConfig, // GPIO pin on the Raspberry Pi connected to the reset pin on the MCU
    // wake: u8, // GPIO pin on the Taspberry Pi used for waking it up from the MCU
    pub default_recipe: String,
    pub recipes: std::collections::HashMap<String, Recipe>,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    /// Path of the configuration file
    #[arg(short, long, value_name = "FILE", value_parser = valid_path)]
    pub config_path: PathBuf,
    /// The recipe to use.
    /// If no recipe is specified, the default is used.
    /// If a single string is specified, the corresponding recipe is used. If no matching
    /// recipe exists, the string is interpreted and executed as a command.
    /// If multiple strings are specified, they are interpreted and executed as a command
    /// followed by a set of arguments. Existing recipies are not considered.
    #[clap(verbatim_doc_comment)]
    pub recipe: Vec<String>,
    
    /// Whether or not the ESP should be rebooted.
    #[arg(short, long)]
    pub reset: bool,

    /// Whether or not the ESP should be rebooted into flash mode.
    /// flash implies reset.
    #[arg(short, long)]
    pub flash: bool
}

fn valid_path(path: &str) -> Result<PathBuf, color_eyre::Report> {
    let path = PathBuf::from(path);
    // return path.try_exists()?.then_some(path).context("Not a valid path.");
    path.is_file().then_some(path).context("Not a valid file.")
}