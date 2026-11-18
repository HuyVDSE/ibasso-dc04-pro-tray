use hidapi::HidApi;
use ibasso_dc04_pro::dc04::{self, Device, GainLevel, VolumeDownEvent};
use std::{
    env,
    error::Error,
    process::ExitCode,
    thread,
    time::{Duration, Instant},
};

const USAGE: &str = "Usage: dc04-cli <list | get [--raw] | fpga | volume-down [--unplug-test] | volume-set <1..99> | set-gain <low|mid|high> | bench-edge <0|1|100> | --version>\nPrefix get, fpga, or set-gain with --trace to log HID reports to stderr.\nbench-edge is for hardware tests only. Stop playback and unplug headphones first.";

enum Command {
    List,
    Get { raw: bool },
    Fpga,
    VolumeDown { unplug_test: bool },
    VolumeSet(u8),
    SetGain(GainLevel),
    BenchEdge(u8),
}

fn run(command: Command, trace: bool) -> Result<(), Box<dyn Error>> {
    let api = HidApi::new()?;
    match command {
        Command::List => {
            let mut collections = dc04::collections(&api).peekable();
            if collections.peek().is_none() {
                println!("No iBasso DC04 Pro HID collections found.");
            }
            for info in collections {
                println!(
                    "{} VID={:04x} PID={:04x} usage_page={:04x} usage={:04x} interface={} path={}",
                    info.product_string().unwrap_or("iBasso DC04 Pro"),
                    info.vendor_id(),
                    info.product_id(),
                    info.usage_page(),
                    info.usage(),
                    info.interface_number(),
                    info.path().to_string_lossy(),
                );
            }
        }
        Command::Get { raw } => {
            let reading = Device::open(&api)?.with_hid_trace(trace).volume_reading()?;
            if raw {
                println!("{reading}");
            } else {
                println!("{}", reading.step());
            }
        }
        Command::Fpga => println!(
            "FPGA version byte: 0x{:02X}",
            Device::open(&api)?.with_hid_trace(trace).fpga_version()?
        ),
        Command::VolumeSet(step) => {
            println!(
                "Verified: {}",
                Device::open(&api)?.set_volume(step, || true)?
            );
        }
        Command::SetGain(level) => {
            Device::open(&api)?.with_hid_trace(trace).set_gain(
                level,
                || true,
                |command, address| {
                    eprintln!("Received matching reply for gain command 0x{command:02X} (DAC 0x{address:02X}).");
                },
            )?;
            println!("Sent gain level: {level}");
        }
        Command::BenchEdge(step) => {
            println!("Verified: {}", Device::open(&api)?.bench_edge(step)?);
        }
        Command::VolumeDown { unplug_test } => {
            let mut resumed_at = None;
            let result = Device::open(&api)?.volume_down(|event| match event {
                VolumeDownEvent::Planned { before, target } => {
                    eprintln!("Before: {before}");
                    eprintln!("Target: {target}");
                }
                VolumeDownEvent::LeftStoreReply if unplug_test => {
                    eprintln!(
                        "Received reply for 0x13 (register 0x10). Unplug the DAC now; keep it unplugged until the CLI exits. Resuming in 5 seconds."
                    );
                    thread::sleep(Duration::from_secs(5));
                    resumed_at = Some(Instant::now());
                }
                VolumeDownEvent::LeftStoreReply => {}
            });
            if let Some(resumed_at) = resumed_at {
                eprintln!(
                    "Time from resuming I/O to result: {} ms",
                    resumed_at.elapsed().as_millis()
                );
            }
            println!("Verified: {}", result?);
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1).peekable();
    let trace = args.next_if(|arg| arg == "--trace").is_some();
    let command = match (args.next().as_deref(), args.next().as_deref(), args.next()) {
        (Some("-V" | "--version"), None, None) => {
            println!(concat!("dc04-cli ", env!("CARGO_PKG_VERSION")));
            return ExitCode::SUCCESS;
        }
        (Some("-h" | "--help"), None, None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        (Some("list"), None, None) => Command::List,
        (Some("get"), option @ (None | Some("--raw")), None) => Command::Get {
            raw: option.is_some(),
        },
        (Some("fpga"), None, None) => Command::Fpga,
        (Some("volume-down"), option @ (None | Some("--unplug-test")), None) => {
            Command::VolumeDown {
                unplug_test: option.is_some(),
            }
        }
        (Some("volume-set"), Some(step), None)
            if step
                .parse::<u8>()
                .is_ok_and(|step| (dc04::MIN_TARGET..=dc04::MAX_TARGET).contains(&step)) =>
        {
            Command::VolumeSet(step.parse().unwrap())
        }
        (Some("bench-edge"), Some(step @ ("0" | "1" | "100")), None) => {
            Command::BenchEdge(step.parse().unwrap())
        }
        (Some("set-gain"), Some("low"), None) => Command::SetGain(GainLevel::Low),
        (Some("set-gain"), Some("mid"), None) => Command::SetGain(GainLevel::Mid),
        (Some("set-gain"), Some("high"), None) => Command::SetGain(GainLevel::High),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    if trace
        && !matches!(
            command,
            Command::Get { .. } | Command::Fpga | Command::SetGain(_)
        )
    {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    match run(command, trace) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
