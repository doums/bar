// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::module::{BaruMod, RunPtr};
use crate::nl_data::{self, WirelessState};
use crate::pulse::Pulse;
use crate::{Config as MainConfig, ModuleMsg};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PLACEHOLDER: &str = "-";
const TICK_RATE: Duration = Duration::from_millis(500);
const DISPLAY: Display = Display::Signal;
const MAX_ESSID_LEN: usize = 10;
const INTERFACE: &str = "wlan0";
const TEXT: &str = "wle";
const DISCONNECTED_TEXT: &str = ".wl";

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
enum Display {
    Essid,
    Signal,
    TextOnly,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    tick: Option<u32>,
    display: Option<Display>,
    max_essid_len: Option<usize>,
    interface: Option<String>,
    placeholder: Option<String>,
    text: Option<String>,
    disconnected_text: Option<String>,
}

#[derive(Debug)]
pub struct InternalConfig<'a> {
    display: Display,
    max_essid_len: usize,
    interface: &'a str,
    tick: Duration,
    text: &'a str,
    disconnected_text: &'a str,
}

impl<'a> From<&'a MainConfig> for InternalConfig<'a> {
    fn from(config: &'a MainConfig) -> Self {
        let mut tick = TICK_RATE;
        let mut display = DISPLAY;
        let mut max_essid_len = MAX_ESSID_LEN;
        let mut interface = INTERFACE;
        let mut text = TEXT;
        let mut disconnected_text = DISCONNECTED_TEXT;
        if let Some(c) = &config.wireless {
            if let Some(t) = c.tick {
                tick = Duration::from_millis(t as u64)
            }
            if let Some(d) = &c.display {
                display = *d
            }
            if let Some(m) = c.max_essid_len {
                max_essid_len = m
            }
            if let Some(i) = &c.interface {
                interface = i
            }
            if let Some(v) = &c.text {
                text = v;
            }
            if let Some(v) = &c.disconnected_text {
                disconnected_text = v;
            }
        };
        InternalConfig {
            display,
            max_essid_len,
            interface,
            tick,
            text,
            disconnected_text,
        }
    }
}

#[derive(Debug)]
pub struct Wireless<'a> {
    placeholder: &'a str,
    config: &'a MainConfig,
}

impl<'a> Wireless<'a> {
    pub fn with_config(config: &'a MainConfig) -> Self {
        let mut placeholder = PLACEHOLDER;
        if let Some(c) = &config.wireless {
            if let Some(p) = &c.placeholder {
                placeholder = p
            }
        }
        Wireless {
            placeholder,
            config,
        }
    }
}

impl<'a> BaruMod for Wireless<'a> {
    fn run_fn(&self) -> RunPtr {
        run
    }

    fn placeholder(&self) -> &str {
        self.placeholder
    }

    fn name(&self) -> &str {
        "wireless"
    }
}

pub fn run(
    key: char,
    main_config: MainConfig,
    _: Arc<Mutex<Pulse>>,
    tx: Sender<ModuleMsg>,
) -> Result<(), Error> {
    let config = InternalConfig::from(&main_config);
    loop {
        let state = nl_data::wireless_data(&config.interface);
        let text;
        let mut essid = "".to_owned();
        let mut signal = None;
        if let WirelessState::Connected(data) = state {
            text = config.text;
            if let Some(strength) = data.signal {
                signal = Some(strength);
            };
            if let Some(val) = data.essid {
                essid = if val.chars().count() > config.max_essid_len {
                    val[..config.max_essid_len].to_owned()
                } else {
                    val
                }
            }
        } else {
            text = config.disconnected_text;
        }
        match config.display {
            Display::TextOnly => tx.send(ModuleMsg(key, text.to_string()))?,
            Display::Essid => tx.send(ModuleMsg(key, format!("{}{}", essid, text)))?,
            Display::Signal => {
                if let Some(s) = signal {
                    tx.send(ModuleMsg(key, format!("{:3}%{}", s, text)))?;
                } else {
                    tx.send(ModuleMsg(key, format!("    {}", text)))?;
                }
            }
        }
        thread::sleep(config.tick);
    }
}
