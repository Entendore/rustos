use core::fmt;
use spin::Mutex;
use uart_16550::SerialPort;

pub static COM1: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(0x3F8) });

pub fn init() {
    COM1.lock().init();
    let _ = write!(COM1.lock(), "\n--- RustOS serial console ---\n");
}

pub fn print(args: fmt::Arguments) {
    use core::fmt::Write;
    let _ = COM1.lock().write_fmt(args);
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => { $crate::serial::print(format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! serial_println {
    () => { $crate::serial_print!("\n") };
    ($($arg:tt)*) => { $crate::serial_print!("{}\n", format_args!($($arg)*)) };
}
