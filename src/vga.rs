use core::fmt;
use spin::Mutex;
use volatile::Volatile;
use x86_64::instructions::port::Port;

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0x0, Blue = 0x1, Green = 0x2, Cyan = 0x3,
    Red = 0x4, Magenta = 0x5, Brown = 0x6, LightGray = 0x7,
    DarkGray = 0x8, LightBlue = 0x9, LightGreen = 0xA, LightCyan = 0xB,
    LightRed = 0xC, LightMagenta = 0xD, Yellow = 0xE, White = 0xF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(fg: Color, bg: Color) -> Self {
        Self((bg as u8) << 4 | (fg as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar { ascii: u8, color: ColorCode }

#[repr(transparent)]
struct Buffer([[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT]);

pub struct Writer {
    col: usize, row: usize, color: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub const fn new(color: ColorCode, buffer: &'static mut Buffer) -> Self {
        Self { col: 0, row: 0, color, buffer }
    }

    pub fn set_color(&mut self, color: ColorCode) { self.color = color; }

    fn update_cursor(&self) {
        let pos = self.row * BUFFER_WIDTH + self.col;
        unsafe {
            let mut ctrl = Port::new(0x3D4);
            let mut data = Port::new(0x3D5);
            ctrl.write(0x0F);
            data.write((pos & 0xFF) as u8);
            ctrl.write(0x0E);
            data.write(((pos >> 8) & 0xFF) as u8);
        }
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.newline(),
            byte => {
                if self.col >= BUFFER_WIDTH { self.newline(); }
                let row = self.row; let col = self.col;
                self.buffer.0[row][col].write(ScreenChar { ascii: byte, color: self.color });
                self.col += 1;
            }
        }
        self.update_cursor();
    }

    fn newline(&mut self) {
        if self.row < BUFFER_HEIGHT - 1 { self.row += 1; } else { self.scroll_up(); }
        self.col = 0;
        self.update_cursor();
    }

    fn scroll_up(&mut self) {
        for r in 1..BUFFER_HEIGHT {
            for c in 0..BUFFER_WIDTH {
                let ch = self.buffer.0[r][c].read();
                self.buffer.0[r - 1][c].write(ch);
            }
        }
        let blank = ScreenChar { ascii: b' ', color: self.color };
        for c in 0..BUFFER_WIDTH { self.buffer.0[BUFFER_HEIGHT - 1][c].write(blank); }
    }

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() { self.write_byte(byte); }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result { Writer::write_str(self, s); Ok(()) }
}

static WRITER: Mutex<Writer> = Mutex::new(Writer::new(
    ColorCode::new(Color::LightGreen, Color::Black),
    unsafe { &mut *(0xB8000 as *mut Buffer) },
));

pub fn init() {
    let blank = ScreenChar { ascii: b' ', color: ColorCode::new(Color::White, Color::Black) };
    let mut w = WRITER.lock();
    for r in 0..BUFFER_HEIGHT {
        for c in 0..BUFFER_WIDTH { w.buffer.0[r][c].write(blank); }
    }
    w.row = 0; w.col = 0;
    w.update_cursor();
}

pub fn set_color(cc: ColorCode) { WRITER.lock().set_color(cc); }

pub fn print(args: fmt::Arguments) {
    use core::fmt::Write;
    let _ = WRITER.lock().write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => { $crate::vga::print(format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => { $crate::print!("{}\n", format_args!($($arg)*)) };
}
