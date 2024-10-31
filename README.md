![](https://img.shields.io/github/issues-raw/cyrinux/push2talk)
![](https://img.shields.io/github/stars/cyrinux/push2talk)
![](https://img.shields.io/aur/version/push2talk-git)
![](https://img.shields.io/crates/d/push2talk)
![](https://img.shields.io/crates/v/push2talk)

![Push-to-Talk Logo](./pictures/logo-small.png)

# Push-to-Talk: Seamless Integration with Wayland, X11, PulseAudio & PipeWire

Fork of [cyrinux/push2talk](https://github.com/cyrinux/push2talk) but removes xkbcommon because it doesnt support F13+ function keys

## 🥅 Quick Start

Upon initialization, the application mutes all microphones. To unmute, press <kbd>F10</kbd>, and release to mute again.

- Suspend/resume functionality available via `SIGUSR1`.

## ⚠️ Prerequisites

### Install depends
```
sudo dnf install rust-libudev-devel rust-input-devel
```

### Setup UDEV rule
Get Vendor ID of input device via lsusb (should be in the format `<vendorid>:<productid>`)
```
lsusb 
```

Edit /etc/udev/rules.d/60-push2talk.rules replace `<vendorid>` with the ID from lsusb
```
KERNEL=="event[0-9]*", SUBSYSTEM=="input", ATTRS{idVendor}=="<vendorid>", MODE="0660", TAG+="uaccess"
```

Reload udev rules
```
sudo udevadm control --reload-rules && udevadm trigger
```

## 📦 Installation Methods

- Clone the repo and run
```
make build
sudo make install
```

## 🎤 Usage

- Start `push2talk` binary.
- Systemd unit provided: `systemctl --user start push2talk.service`.

## 🎤 Advanced Configuration

- Trace mode for key and source device identification: `env RUST_LOG=trace push2talk`.
- Custom keybinds via environment variables: `env PUSH2TALK_KEYBIND="68" push2talk`. [Here is a list of keycodes](https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h)
- Debug logging: `RUST_LOG=debug push2talk`.
- Specify a particular audio source: `env PUSH2TALK_SOURCE="OpenComm by Shokz" push2talk`.
- Systemd unit provided: `systemctl --user start push2talk.service`.

## 😅 Additional Information

- Excludes Easy Effects sources to prevent unintentional "push-to-listen" scenarios.

## 👥 How to Contribute

Contributions are highly welcome.

## 💑 Acknowledgments

Made with love by @cyrinux and @maximbaz.
