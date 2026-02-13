use std::process::ExitCode;

/// Parsed command-line arguments
#[derive(Default)]
pub struct Args {
    /// Start the application minimized to the system tray
    pub minimized: bool,
}

impl Args {
    pub fn parse() -> Result<Self, ExitCode> {
        let mut args = Args::default();

        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "--version" | "-v" => {
                    print_version();
                    return Err(ExitCode::SUCCESS);
                }
                "--help" | "-h" => {
                    print_help();
                    return Err(ExitCode::SUCCESS);
                }
                "--minimized" => {
                    args.minimized = true;
                }
                _ => {
                    eprintln!("Error: Unknown argument '{}'", arg);
                    return Err(ExitCode::FAILURE);
                }
            }
        }

        Ok(args)
    }
}

fn print_version() {
    println!(env!("CARGO_PKG_VERSION"));
}

fn print_help() {
    println!(concat!(
        "WSL USB Manager - Manage USB devices for WSL\n\n",
        "USAGE:\n",
        "    wsl-usb-manager [OPTIONS]\n\n",
        "OPTIONS:\n",
        "    -h, --help         Print help information\n",
        "    -v, --version      Print version information\n",
        "        --minimized    Start minimized to the system tray\n",
    ));
}
