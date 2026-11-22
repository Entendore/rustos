use x86_64::instructions::port::Port;

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

pub struct PciConfig {
    address: Port<u32>,
    data: Port<u32>,
}

impl PciConfig {
    pub const fn new() -> Self {
        Self {
            address: Port::new(CONFIG_ADDRESS),
            data: Port::new(CONFIG_DATA),
        }
    }

    pub fn read(&mut self, bus: u8, device: u8, func: u8, offset: u8) -> u32 {
        let address = 0x80000000
            | ((bus as u32) << 16)
            | (((device & 0x1F) as u32) << 11)
            | (((func & 0x07) as u32) << 8)
            | ((offset & 0xFC) as u32);
        
        unsafe {
            self.address.write(address);
            self.data.read()
        }
    }
}

static PCI: spin::Mutex<PciConfig> = spin::Mutex::new(PciConfig::new());

pub fn scan_bus() {
    crate::info!("Scanning PCI bus...");
    let mut pci = PCI.lock();
    for bus in 0..=255 {
        for device in 0..32 {
            for func in 0..8 {
                let vendor_id = (pci.read(bus, device, func, 0) & 0xFFFF) as u16;
                if vendor_id == 0xFFFF { continue; }
                
                let header = pci.read(bus, device, func, 0x0C);
                let class_code = (header >> 24) & 0xFF;
                let subclass = (header >> 16) & 0xFF;
                let device_id = (pci.read(bus, device, func, 0) >> 16) & 0xFFFF;
                
                crate::info!("PCI {:02x}:{:02x}.{:x} Vendor={:04x} Device={:04x} Class={:02x} Sub={:02x}", 
                    bus, device, func, vendor_id, device_id, class_code, subclass);
                
                if (header & 0x7F) == 0 { break; }
            }
        }
    }
}
