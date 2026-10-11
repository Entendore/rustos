#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]
#![feature(stmt_expr_attributes)]

extern crate alloc;

mod allocator;
mod ata;
mod cpuid;
mod driver;
mod env;
mod executor;
mod fs;
mod gdt;
mod interrupts;
mod logger;
mod memory;
mod pci;
mod power;
mod resource;
mod rtc;
mod scheduler;
mod serial;
mod shell;
mod storage;
mod syscall;
mod sync;
mod vga;

use bootloader::{entry_point, BootInfo};
use core::sync::atomic::{AtomicBool, Ordering};

entry_point!(kernel_main);

static INIT_DONE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    Invalid, BadFd, NotFound, IoError, Permission, OutOfMemory, NotSupported,
}

impl core::fmt::Display for KernelError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            KernelError::Invalid => write!(f, "Invalid argument"),
            KernelError::BadFd => write!(f, "Bad file descriptor"),
            KernelError::NotFound => write!(f, "Not found"),
            KernelError::IoError => write!(f, "I/O error"),
            KernelError::Permission => write!(f, "Permission denied"),
            KernelError::OutOfMemory => write!(f, "Out of memory"),
            KernelError::NotSupported => write!(f, "Not supported"),
        }
    }
}

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    serial::init();
    vga::init();
    info!("RustOS booting...");

    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    interrupts::enable();

    let mut memory = memory::init(boot_info);
    allocator::init(&mut memory).expect("heap init failed");

    fs::init();
    env::init();
    storage::init();
    scheduler::init();
    syscall::init();
    driver::keyboard::init();
    
    pci::scan_bus();
    info!("Boot time: {}", rtc::read_time());

    info!("Probing ATA drive...");
    let mut ata = ata::AtaDrive::new();
    let mut buf = [0u8; 512];
    match ata.read_block(0, &mut buf) {
        Ok(_) => info!("ATA read successful! First byte: 0x{:02X}", buf[0]),
        Err(e) => warn!("ATA read failed (expected on diskless QEMU): {}", e),
    }

    INIT_DONE.store(true, Ordering::Release);
    info!("All subsystems online. Spawning shell.");

    shell::spawn();
    scheduler::run()
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::sync::atomic::AtomicUsize;
    static PANIC_COUNT: AtomicUsize = AtomicUsize::new(0);
    if PANIC_COUNT.fetch_add(1, Ordering::SeqCst) > 0 {
        hlt_loop();
    }
    if INIT_DONE.load(Ordering::Acquire) {
        error!("{info}");
        vga::set_color(vga::ColorCode::new(vga::Color::LightRed, vga::Color::Black));
        println!("[panic] {info}");
    }
    hlt_loop()
}

pub fn hlt_loop() -> ! {
    loop { x86_64::instructions::halt(); }
}
