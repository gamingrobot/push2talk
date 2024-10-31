use input::event::keyboard::KeyState::*;
use input::event::keyboard::KeyboardEventTrait;
use input::{Libinput, LibinputInterface};
use libc::{O_RDWR, O_WRONLY};
use log::{debug, trace};
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::{
    fs::OpenOptionsExt,
    io::{AsRawFd, OwnedFd},
};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::Mutex;
use std::{cell::Cell, env, sync::Arc};

pub struct Controller {
    key: u32,
    key_pressed: Cell<bool>,
    last_mute: Cell<bool>,
}

impl Controller {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let keybind_parsed = parse_keybind()?;

        Ok(Controller {
            key: keybind_parsed,
            key_pressed: Cell::new(false),
            last_mute: Cell::new(false),
        })
    }

    pub fn run(&self, tx: Sender<bool>, is_paused: Arc<Mutex<bool>>) -> Result<(), Box<dyn Error>> {
        // Mute on init
        tx.send(false)?; // this is needed to 'bootstrap' pulseaudio's subscribe callback, otherwise the very first unmute won't work
        tx.send(true)?;
        self.last_mute.set(true);

        let mut libinput_context = Libinput::new_with_udev(Push2TalkLibinput);
        libinput_context
            .udev_assign_seat("seat0")
            .map_err(|err| format!("Can't connect to libinput on seat0: {err:?}"))?;

        let mut fds = [libc::pollfd {
            fd: libinput_context.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        }];

        let poll_timeout = 1000;
        let mut is_running = true;

        loop {
            let poll_err = unsafe { libc::poll(fds.as_mut_ptr(), 1, poll_timeout) } < 0;
            if poll_err {
                // on pause signal send, libc abort polling and
                // receive EINTR error
                if io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                return Err("Unable to poll libinput, aborting".into());
            }

            libinput_context.dispatch()?;

            let is_paused_now = is_paused
                .lock()
                .map_err(|err| format!("Deadlock in libinput checking if we are paused: {err}"))?;

            if is_running == *is_paused_now {
                is_running = !is_running;

                // Toggle mute on pause/resume
                tx.send(is_running)?;

                // ignore final events that happened just before the resume signal
                if is_running {
                    libinput_context.by_ref().for_each(drop);
                }
            }

            for event in libinput_context.by_ref() {
                if is_running {
                    self.handle(event, tx.clone())?;
                }
            }
        }
    }

    fn handle(&self, event: input::Event, tx: Sender<bool>) -> Result<(), Box<dyn Error>> {
        if let input::Event::Keyboard(key_event) = event {
            let pressed = check_pressed(&key_event);
            trace!(
                "Key {}",
                if pressed { "pressed" } else { "released" },
            );

            self.update(&key_event, pressed);

            let should_mute = self.should_mute();
            if should_mute != self.last_mute.get() {
                debug!(
                    "Microphone is {}",
                    if should_mute { "muted" } else { "unmuted" }
                );
                self.last_mute.set(should_mute);
                tx.send(should_mute)?;
            }
        };

        Ok(())
    }

    fn update(&self, key_event: &input::event::KeyboardEvent, pressed: bool) {
        match key_event.key() {
            k if k == self.key => self.key_pressed.set(pressed),
            _ => {}
        }
    }

    fn should_mute(&self) -> bool {
        !self.key_pressed.get()
    }
}

fn parse_keybind() -> Result<u32, Box<dyn Error>> {
    env::var("PUSH2TALK_KEYBIND")
        .unwrap_or("68".to_string())
        .parse::<u32>().map_err(|e| e.into())
}

fn check_pressed(key_event: &input::event::KeyboardEvent) -> bool {
    match key_event.key_state() {
        Released => false,
        Pressed => true,
    }
}

struct Push2TalkLibinput;

impl LibinputInterface for Push2TalkLibinput {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        OpenOptions::new()
            .custom_flags(flags)
            .read(true)
            .write((flags & O_WRONLY != 0) | (flags & O_RDWR != 0))
            .open(path)
            .map(|file| file.into())
            .map_err(|err| err.raw_os_error().unwrap())
    }

    fn close_restricted(&mut self, fd: OwnedFd) {
        let file = File::from(fd);
        drop(file);
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_parse_keybind_default() {
//         // Assuming default keybinds are Control_L and Space
//         std::env::remove_var("PUSH2TALK_KEYBIND");
//         let keybind = parse_keybind().unwrap();
//         assert_eq!(keybind.len(), 2);
//         // Assuming default keybinds are Control_L and Space
//         assert_eq!(
//             keybind[0],
//             xkb::keysym_from_name("Control_L", xkb::KEYSYM_CASE_INSENSITIVE)
//         );
//         assert_eq!(
//             keybind[1],
//             xkb::keysym_from_name("Space", xkb::KEYSYM_CASE_INSENSITIVE)
//         );
//     }

//     #[test]
//     fn test_parse_keybind_with_2_valid_keys() {
//         std::env::set_var("PUSH2TALK_KEYBIND", "Control_L,O");
//         let keybind = parse_keybind().unwrap();
//         assert_eq!(keybind.len(), 2);
//         assert_eq!(
//             keybind[0],
//             xkb::keysym_from_name("Control_L", xkb::KEYSYM_CASE_INSENSITIVE)
//         );
//         assert_eq!(
//             keybind[1],
//             xkb::keysym_from_name("O", xkb::KEYSYM_CASE_INSENSITIVE)
//         );
//         std::env::remove_var("PUSH2TALK_KEYBIND");
//     }

//     #[test]
//     fn test_parse_keybind_with_invalid_key() {
//         std::env::set_var("PUSH2TALK_KEYBIND", "InvalidKey");
//         assert!(parse_keybind().is_err());
//         std::env::remove_var("PUSH2TALK_KEYBIND");
//     }

//     #[test]
//     fn test_validate_keybind_with_2_keys_is_valid() {
//         let keybind = vec![
//             xkb::keysym_from_name("Control_L", xkb::KEYSYM_CASE_INSENSITIVE),
//             xkb::keysym_from_name("Space", xkb::KEYSYM_CASE_INSENSITIVE),
//         ];
//         assert!(validate_keybind(&keybind).is_ok());
//     }

//     #[test]
//     fn test_validate_keybind_with_3_keys_is_invalid() {
//         let keybind = vec![
//             xkb::keysym_from_name("Control_L", xkb::KEYSYM_CASE_INSENSITIVE),
//             xkb::keysym_from_name("Space", xkb::KEYSYM_CASE_INSENSITIVE),
//             xkb::keysym_from_name("Shift_R", xkb::KEYSYM_CASE_INSENSITIVE),
//         ];
//         assert!(validate_keybind(&keybind).is_err());
//     }
// }
