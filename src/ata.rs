use crate::storage::{BlockDevice, BLOCK_SIZE};
use crate::KernelError;
use x86_64::instructions::port::Port;

pub struct AtaDrive {
    data: Port<u16>,
    error: Port<u8>,
    sector_count: Port<u8>,
    lba_lo: Port<u8>,
    lba_mid: Port<u8>,
    lba_hi: Port<u8>,
    drive_head: Port<u8>,
    status: Port<u8>,
    command: Port<u8>,
}

impl AtaDrive {
    pub fn new() -> Self {
        Self {
            data: Port::new(0x1F0),
            error: Port::new(0x1F1),
            sector_count: Port::new(0x1F2),
            lba_lo: Port::new(0x1F3),
            lba_mid: Port::new(0x1F4),
            lba_hi: Port::new(0x1F5),
            drive_head: Port::new(0x1F6),
            status: Port::new(0x1F7),
            command: Port::new(0x1F7),
        }
    }

    fn wait_ready(&mut self) {
        loop {
            let status = unsafe { self.status.read() };
            if status & 0x80 == 0 && status & 0x08 != 0 {
                break;
            }
        }
    }
}

impl BlockDevice for AtaDrive {
    fn read_block(&mut self, lba: u32, buf: &mut [u8; BLOCK_SIZE]) -> Result<(), KernelError> {
        unsafe {
            self.drive_head.write(0xE0 | ((lba >> 24) & 0x0F) as u8);
            self.sector_count.write(1);
            self.lba_lo.write(lba as u8);
            self.lba_mid.write((lba >> 8) as u8);
            self.lba_hi.write((lba >> 16) as u8);
            self.command.write(0x20);
        }

        self.wait_ready();

        let status = unsafe { self.status.read() };
        if status & 0x01 != 0 {
            return Err(KernelError::IoError);
        }

        for i in 0..256 {
            let val = unsafe { self.data.read() };
            buf[i * 2] = val as u8;
            buf[i * 2 + 1] = (val >> 8) as u8;
        }
        Ok(())
    }

    fn write_block(&mut self, lba: u32, buf: &[u8; BLOCK_SIZE]) -> Result<(), KernelError> {
        unsafe {
            self.drive_head.write(0xE0 | ((lba >> 24) & 0x0F) as u8);
            self.sector_count.write(1);
            self.lba_lo.write(lba as u8);
            self.lba_mid.write((lba >> 8) as u8);
            self.lba_hi.write((lba >> 16) as u8);
            self.command.write(0x30);
        }

        self.wait_ready();

        for i in 0..256 {
            let val = (buf[i * 2] as u16) | ((buf[i * 2 + 1] as u16) << 8);
            unsafe { self.data.write(val) };
        }

        unsafe { self.command.write(0xE7) };
        self.wait_ready();
        Ok(())
    }
}
