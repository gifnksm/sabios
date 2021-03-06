use crate::{
    layer,
    prelude::*,
    sync::{mpsc, OnceCell},
};
use core::future::Future;
use enumflags2::{bitflags, BitFlags};

const KEYCODE_MAP: [char; 256] = [
    '\0', '\0', '\0', '\0', 'a', 'b', 'c', 'd', // 0
    'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', // 8
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', // 16
    'u', 'v', 'w', 'x', 'y', 'z', '1', '2', // 24
    '3', '4', '5', '6', '7', '8', '9', '0', // 32
    '\n', '\x08', '\x08', '\t', ' ', '-', '=', '[', // 40
    ']', '\\', '#', ';', '\'', '`', ',', '.', // 48
    '/', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 56
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 64
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 72
    '\0', '\0', '\0', '\0', '/', '*', '-', '+', // 80
    '\n', '1', '2', '3', '4', '5', '6', '7', // 88
    '8', '9', '0', '.', '\\', '\0', '\0', '=', // 96
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 104
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 112
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 120
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 128
    '\0', '\\', '\0', '\0', '\0', '\0', '\0', '\0', // 136
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 144
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 152
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 160
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 168
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 176
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 184
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 192
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 200
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 208
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 216
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 224
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 232
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 240
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 248
];

const KEYCODE_MAP_SHIFT: [char; 256] = [
    '\0', '\0', '\0', '\0', 'A', 'B', 'C', 'D', // 0
    'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', // 8
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', // 16
    'U', 'V', 'W', 'X', 'Y', 'Z', '!', '@', // 24
    '#', '$', '%', '^', '&', '*', '(', ')', // 32
    '\n', '\x08', '\x08', '\t', ' ', '_', '+', '{', // 40
    '}', '|', '~', ':', '"', '~', '<', '>', // 48
    '?', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 56
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 64
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 72
    '\0', '\0', '\0', '\0', '/', '*', '-', '+', // 80
    '\n', '1', '2', '3', '4', '5', '6', '7', // 88
    '8', '9', '0', '.', '\\', '\0', '\0', '=', // 96
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 104
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 112
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 120
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 128
    '\0', '|', '\0', '\0', '\0', '\0', '\0', '\0', // 136
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 144
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 152
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 160
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 168
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 176
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 184
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 192
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 200
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 208
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 216
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 224
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 232
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 240
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 248
];

#[bitflags]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Modifier {
    LControl = 0b00000001,
    LShift = 0b00000010,
    LAlt = 0b00000100,
    LGui = 0b00001000,
    RControl = 0b00010000,
    RShift = 0b00100000,
    RAlt = 0b01000000,
    RGui = 0b10000000,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawKeyboardEvent {
    modifier: BitFlags<Modifier>,
    keycode: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyboardEvent {
    pub(crate) modifier: BitFlags<Modifier>,
    pub(crate) keycode: u8,
    pub(crate) ascii: char,
}

static KEYBOARD_EVENT_TX: OnceCell<mpsc::Sender<RawKeyboardEvent>> = OnceCell::uninit();

pub(crate) extern "C" fn observer(modifier: u8, keycode: u8) {
    let modifier = BitFlags::<Modifier>::from_bits_truncate(modifier);
    let event = RawKeyboardEvent { modifier, keycode };
    let res = KEYBOARD_EVENT_TX.try_get().and_then(|tx| tx.send(event));

    if let Err(err) = res {
        error!("failed to enqueue to the queue: {}", err);
    }
}

pub(crate) fn handler_task() -> impl Future<Output = Result<()>> {
    // Initialize KEYBOARD_EVENT_TX before co-task starts
    let (tx, mut rx) = mpsc::channel(100);
    KEYBOARD_EVENT_TX.init_once(|| tx);

    async move {
        let tx = layer::event_tx();

        while let Some(event) = rx.next().await {
            let ascii = if event
                .modifier
                .intersects(Modifier::LShift | Modifier::RShift)
            {
                KEYCODE_MAP_SHIFT[usize::from(event.keycode)]
            } else {
                KEYCODE_MAP[usize::from(event.keycode)]
            };
            let event = KeyboardEvent {
                modifier: event.modifier,
                keycode: event.keycode,
                ascii,
            };
            tx.keyboard_event(event).await?;
        }
        Ok(())
    }
}
