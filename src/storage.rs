use crate::KernelError;
use alloc::vec::Vec;
use spin::Mutex;

pub const BLOCK_SIZE: usize = 512;

pub trait BlockDevice {
    fn read_block(&mut self, lba: u32, buf: &mut [u8; BLOCK_SIZE]) -> Result<(), KernelError>;
    fn write_block(&mut self, lba: u32, buf: &[u8; BLOCK_SIZE]) -> Result<(), KernelError>;
}

pub struct RamDisk {
    storage: Vec<[u8; BLOCK_SIZE]>,
}

impl RamDisk {
    pub fn new(blocks: u32) -> Self {
        let mut storage = Vec::with_capacity(blocks as usize);
        for _ in 0..blocks {
            storage.push([0u8; BLOCK_SIZE]);
        }
        Self { storage }
    }
}

impl BlockDevice for RamDisk {
    fn read_block(&mut self, lba: u32, buf: &mut [u8; BLOCK_SIZE]) -> Result<(), KernelError> {
        let lba = lba as usize;
        if lba >= self.storage.len() {
            return Err(KernelError::IoError);
        }
        buf.copy_from_slice(&self.storage[lba]);
        Ok(())
    }

    fn write_block(&mut self, lba: u32, buf: &[u8; BLOCK_SIZE]) -> Result<(), KernelError> {
        let lba = lba as usize;
        if lba >= self.storage.len() {
            return Err(KernelError::IoError);
        }
        self.storage[lba].copy_from_slice(buf);
        Ok(())
    }
}

pub static RAMDISK: Mutex<RamDisk> = Mutex::new(RamDisk {
    storage: Vec::new(),
});

pub fn init() {
    let mut disk = RAMDISK.lock();
    *disk = RamDisk::new(64);
    crate::info!("Initialized 32KB RAM Disk");
}
