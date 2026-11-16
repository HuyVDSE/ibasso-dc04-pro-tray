# iBasso DC04 Pro Tray

A Windows system tray app that reads and adjusts the hardware volume of the **iBasso DC04 Pro** USB DAC over USB HID. The app runs as a portable executable, `dc04-tray.exe`, and supports volume levels **1–99**.

Tested on **Windows 11 x64** with a **DC04 Pro running FPGA version 1.58**. See the [hardware and portable build test results](research/dc04pro-milestone-4.md).

## Usage

1. Connect the DC04 Pro over USB and run `dc04-tray.exe` without administrator privileges.
2. Left-click the headphones icon in the system tray to view or adjust the volume. If the icon is hidden, click the **^** arrow to show hidden icons.
3. Right-click the icon to open the menu. Select **Khởi động cùng Windows** (Start with Windows) to launch the app automatically when you sign in.

If you move the executable after enabling startup, run it from its new location and select **Khởi động cùng Windows (cập nhật đường dẫn)** (Start with Windows, update path). If the DAC fails to connect, **Chi tiết lỗi DAC** (DAC error details) shows the most recent error.

## Build from source

Install stable Rust with the `x86_64-pc-windows-msvc` target and **Visual Studio Build Tools** with the C++ toolchain and Windows SDK. Run these commands in PowerShell from the repository root so Cargo picks up the static runtime settings in `.cargo/config.toml`.

```powershell
cargo build --locked --release --target x86_64-pc-windows-msvc --manifest-path tray/Cargo.toml
.\tray\target\x86_64-pc-windows-msvc\release\dc04-tray.exe
```

Run the Rust test suite with:

```powershell
cargo test --locked --target x86_64-pc-windows-msvc --manifest-path tray/Cargo.toml
```

## Check the device with the CLI

The `list` command lists the DAC's HID collections. The `get` command reads the current volume level.

```powershell
.\tray\target\x86_64-pc-windows-msvc\release\dc04-cli.exe list
.\tray\target\x86_64-pc-windows-msvc\release\dc04-cli.exe get
```

## Local web control

The repository includes a script that downloads frontend assets from the iBasso control site for local use with WebHID in Chrome or Edge. From the repository root, use Python 3 and `curl.exe` to populate `local-web/`, then start the server.

```powershell
python .\mirror_ibasso.py
.\Start-Local.ps1
```

Open `http://127.0.0.1:8044/`. See [README-local.md](README-local.md) (Vietnamese) for instructions on granting device access, stopping the server, and understanding its limitations. The downloaded frontend assets belong to their original owners.

## Source and documentation

- [`tray/`](tray/) contains the Windows tray app, CLI, and shared HID code.
- [`docs/system-design.md`](docs/system-design.md) describes the application design.
- [`research/`](research/) contains HID protocol notes and hardware test results.
