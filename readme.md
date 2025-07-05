# Ingest Application

This application watches for new removable drives and copies their content to a user selected destination. It is written in Rust and uses `eframe`/`egui` for the GUI.

## Features

- Cross platform (Windows and Linux) GUI
- Watches for new disks using `sysinfo` in the background
- When a new drive is detected, files are copied to the configured destination
- Destination folder is persisted between runs

## Building

```
make build
```

## Running

```
make run
```

The GUI allows selecting a destination folder. Once set, any newly mounted drive will automatically be copied to that destination.

### Common tasks

Install development tools (like `rustfmt` and `clippy`).
These require [`rustup`](https://rustup.rs) to be installed:

```
make install-deps
```

Run checks and tests:

```
make check
make test
```

Format the code:

```
make fmt
```

Autostart at boot is platform specific and not yet implemented. You can create a startup entry or service that runs this binary.
