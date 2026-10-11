use x86_64::instructions::port::Port;
use spin::Mutex;

pub struct Rtc {
    index: Port<u8>,
    data: Port<u8>,
}

impl Rtc {
    pub const fn new() -> Self {
        Self {
            index: Port::new(0x70),
            data: Port::new(0x71),
        }
    }

    fn read_reg(&mut self, reg: u8) -> u8 {
        unsafe {
            self.index.write(reg);
            self.data.read()
        }
    }

    fn is_updating(&mut self) -> bool {
        self.read_reg(0x0A) & 0x80 != 0
    }
}

static RTC: Mutex<Rtc> = Mutex::new(Rtc::new());

pub fn read_time() -> Time {
    let mut rtc = RTC.lock();
    while rtc.is_updating() {}
    
    let mut second = rtc.read_reg(0x00);
    let mut minute = rtc.read_reg(0x02);
    let mut hour = rtc.read_reg(0x04);
    let mut day = rtc.read_reg(0x07);
    let mut month = rtc.read_reg(0x08);
    let mut year = rtc.read_reg(0x09);

    let reg_b = rtc.read_reg(0x0B);

    if reg_b & 0x04 == 0 {
        second = bcd_to_binary(second);
        minute = bcd_to_binary(minute);
        hour = bcd_to_binary(hour) | (if hour & 0x80 != 0 { 0x80 } else { 0 });
        day = bcd_to_binary(day);
        month = bcd_to_binary(month);
        year = bcd_to_binary(year);
    }
    
    let hour = if reg_b & 0x02 != 0 {
        hour
    } else {
        (hour & 0x0F) + if hour & 0x80 != 0 { 12 } else { 0 }
    };

    Time {
        year: 2000 + year as u32,
        month,
        day,
        hour,
        minute,
        second,
    }
}

fn bcd_to_binary(bcd: u8) -> u8 {
    (bcd / 16) * 10 + (bcd & 0x0F)
}

#[derive(Debug, Clone, Copy)]
pub struct Time {
    pub year: u32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl core::fmt::Display for Time {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second)
    }
}
