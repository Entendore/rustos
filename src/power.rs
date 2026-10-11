use x86_64::instructions::port::Port;

pub fn reboot() -> ! {
    info!("Rebooting system...");
    x86_64::instructions::interrupts::disable();
    unsafe {
        let mut port: Port<u8> = Port::new(0x64);
        loop {
            if port.read() & 0x02 == 0 { break; }
        }
        port.write(0xFE);
    }
    loop { x86_64::instructions::halt(); }
}

pub fn shutdown() -> ! {
    info!("Shutting down...");
    x86_64::instructions::interrupts::disable();
    unsafe {
        Port::new(0xf500).write(0x00 as u8);
    }
    loop { x86_64::instructions::halt(); }
}
