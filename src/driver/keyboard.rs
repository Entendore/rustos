use core::sync::atomic::{AtomicBool, Ordering};
use crossbeam_queue::ArrayQueue;
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts};
use conquer_once::LateInit;
use spin::Mutex;

pub static QUEUE: LateInit<ArrayQueue<u8>> = LateInit::new();
static KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
    Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)
);
static INIT: AtomicBool = AtomicBool::new(false);

pub fn init() {
    QUEUE.init(ArrayQueue::new(256));
    crate::interrupts::register_irq(1, handle);
    INIT.store(true, Ordering::Release);
    crate::info!("[driver] keyboard ready");
}

fn handle(_: &x86_64::structures::idt::InterruptStackFrame) {
    let port = x86_64::instructions::port::Port::new(0x60);
    let scancode = unsafe { port.read() };
    if let Some(decoded) = KEYBOARD.lock().process_byte(scancode)
        .and_then(|ev| ev.and_then(|p| KEYBOARD.lock().process_keyevent(p)))
    {
        let ch = match decoded {
            DecodedKey::Unicode(c) => Some(c as u8),
            DecodedKey::RawKey(pc_keyboard::RawKey::ArrowUp) => Some(0xE0),
            DecodedKey::RawKey(pc_keyboard::RawKey::ArrowDown) => Some(0xE1),
            _ => None,
        };
        if let Some(b) = ch {
            let _ = QUEUE.push(b);
        }
    }
}

pub fn try_get() -> Option<u8> {
    if !INIT.load(Ordering::Acquire) { return None; }
    QUEUE.pop().ok()
}
