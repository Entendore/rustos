use conquer_once::LateInit;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub const PIC_1_OFFSET: u8 = 0x20;
pub const PIC_2_OFFSET: u8 = 0x28;

pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
pub static IDT: LateInit<InterruptDescriptorTable> = LateInit::new();

type IrqHandler = fn(&InterruptStackFrame);
static IRQ_HANDLERS: Mutex<[Option<IrqHandler>; 16]> = Mutex::new([None; 16]);

pub fn register_irq(irq: u8, handler: IrqHandler) {
    assert!(irq < 16, "8259 only handles IRQ 0..16");
    IRQ_HANDLERS.lock()[irq as usize] = Some(handler);
}

pub fn init_idt() {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint);
    unsafe { idt.double_fault.set_handler_fn(double_fault).set_stack_index(gdt::DOUBLE_FAULT_IST); }
    idt.page_fault.set_handler_fn(page_fault);
    idt.general_protection_fault.set_handler_fn(gpf);
    
    unsafe { idt[0x20].set_handler_fn(timer_handler).set_stack_index(gdt::TIMER_IST); }
    idt[0x21].set_handler_fn(keyboard_handler);
    
    unsafe {
        idt[0x80].set_handler_fn(syscall_entry);
        idt[0x80].set_privilege_level(x86_64::PrivilegeLevel::Ring3);
    }
    
    IDT.init(idt);
    IDT.load();
    gdt::load();
}

pub fn enable() {
    x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn breakpoint(sf: InterruptStackFrame) {
    serial_println!("[exception] breakpoint\n{sf:#?}");
}

extern "x86-interrupt" fn double_fault(sf: InterruptStackFrame, ec: u64) -> ! {
    panic!("DOUBLE FAULT (code={ec})\n{sf:#?}");
}

extern "x86-interrupt" fn page_fault(sf: InterruptStackFrame, err: PageFaultErrorCode) {
    let cr2 = x86_64::registers::control::Cr2::read();
    panic!("PAGE FAULT @ {cr2:?} code={err:?}\n{sf:#?}");
}

extern "x86-interrupt" fn gpf(sf: InterruptStackFrame, ec: u64) {
    panic!("GENERAL PROTECTION FAULT (code={ec})\n{sf:#?}");
}

extern "x86-interrupt" fn timer_handler(_sf: InterruptStackFrame) {
    crate::scheduler::tick();
    unsafe { PICS.lock().notify_end_of_interrupt(0x20); }
}

extern "x86-interrupt" fn keyboard_handler(sf: InterruptStackFrame) {
    if let Some(handler) = IRQ_HANDLERS.lock()[1] {
        handler(&sf);
    }
    unsafe { PICS.lock().notify_end_of_interrupt(0x21); }
}

#[repr(C)]
pub struct SyscallRegs {
    pub rax: u64, pub rdi: u64, pub rsi: u64, pub rdx: u64,
    pub r10: u64, pub r8: u64, pub r9: u64,
}

#[naked]
unsafe extern "C" fn syscall_entry() {
    core::arch::asm!(
        "push rax", "push rdi", "push rsi", "push rdx",
        "push r10", "push r8", "push r9",
        "mov rdi, rsp",
        "call {handler}",
        "add rsp, 56",
        "iretq",
        handler = sym syscall_handler,
        options(noreturn)
    );
}

extern "C" fn syscall_handler(regs: &mut SyscallRegs) {
    let res = crate::syscall::dispatch(regs.rax, regs.rdi, regs.rsi, regs.rdx, regs.r10, regs.r8, regs.r9);
    regs.rax = res;
}
