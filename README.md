# bose-nc

A macOS command-line tool to control noise cancellation on Bose headphones over Bluetooth RFCOMM.

Communicates with headphones using the BMAP (Bose Multi-device Application Protocol), reverse-engineered from the Bose Music Android application.

## Supported devices

- Bose Noise Cancelling Headphones 700
- Bose QuietComfort Ultra Headphones
- Bose QuietComfort 35 (needs testing)
- Bose QuietComfort 45 (needs testing)
- Bose QuietComfort Earbuds (needs testing)

## Requirements

- macOS (uses IOBluetooth framework)
- Rust toolchain
- Headphones paired and connected via Bluetooth

## Installation

### Homebrew

```sh
brew install mishamyrt/tap/bose-nc
```

### From source

```sh
cargo build --release
cp target/release/bose-nc /usr/local/bin/
```

## Usage

### List connected Bose devices

```sh
bose-nc scan
```

```
  Bose NC 700 HP (C8:7B:23:58:A3:7B)
```

### Check noise cancellation status

```sh
bose-nc status
```

```
Noise cancellation: ON (level 5/10)
```

### Set noise cancellation level

Level ranges from `0` (full transparency) to `10` (maximum noise cancelling):

```sh
bose-nc set 10   # max noise cancelling
bose-nc set 0    # full transparency
```

### Turn off noise cancellation

```sh
bose-nc off
```

### Select a specific device

When multiple Bose devices are connected, use `-d` to pick one by name:

```sh
bose-nc -d NC700 status
```

## Disclaimer

Bose™ is a registered trademark of Bose Corporation. This project is an independent, unofficial tool and is not affiliated with, endorsed by, or connected to Bose Corporation in any way.
