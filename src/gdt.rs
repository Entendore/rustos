use x86_64::instructions::segmentation::{CS, DS, ES, FS, GS, Segment};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;
use spin::Once;

pub const DOUBLE_FAULT_IST: u16 = 0;
pub const TIMER_IST: u16 = 1;
pub const SYSCALL_IST: u16 = 2;

#[repr(C, align(16))]
struct Stack([u8; 4096 * 4]);
static mut DOUBLE_FAULT_STACK: Stack = Stack([0; 4096 * 4]);
static mut TIMER_STACK: Stack = Stack([0; 4096 * 4]);
static mut SYSCALL_STACK: Stack = Stack([0; 4096 * 4]);

static mut TSS: TaskStateSegment = TaskStateSegment::new();
static GDT: Once<(GlobalDescriptorTable, Selectors)> = Once::new();

struct Selectors {
    kernel_code: SegmentSelector,
    kernel_data: SegmentSelector,
    tss: SegmentSelector,
    user_code: SegmentSelector,
    user_data: SegmentSelector,
}

pub fn init() {
    let tss = unsafe {
        TSS.interrupt_stack_table[DOUBLE_FAULT_IST as usize] = VirtAddr::from_ptr(
            core::ptr::addr_of_mut!(DOUBLE_FAULT_STACK) as *mut u8) + 4096 * 4;
        TSS.interrupt_stack_table[TIMER_IST as usize] = VirtAddr::from_ptr(
            core::ptr::addr_of_mut!(TIMER_STACK) as *mut u8) + 4096 * 4;
        TSS.interrupt_stack_table[SYSCALL_IST as usize] = VirtAddr::from_ptr(
            core::ptr::addr_of_mut!(SYSCALL_STACK) as *mut u8) + 4096 * 4;
        &TSS
    };

    let mut gdt = GlobalDescriptorTable::new();
    let kernel_code = gdt.append(Descriptor::kernel_code_segment());
    let kernel_data = gdt.append(Descriptor::kernel_data_segment());
    let user_code = gdt.append(Descriptor::user_code_segment());
    let user_data = gdt.append(Descriptor::user_data_segment());
    let tss_sel = gdt.append(Descriptor::tss_segment(tss));

    GDT.call_once(|| (gdt, Selectors {
        kernel_code, kernel_data, tss: tss_sel, user_code, user_data,
    }));
}

pub fn load() {
    let (gdt, s) = GDT.get().expect("gdt not initialized");
    gdt.load();
    unsafe {
        CS::set_reg(s.kernel_code);
        DS::set_reg(s.kernel_data);
        ES::set_reg(s.kernel_data);
        FS::set_reg(s.kernel_data);
        GS::set_reg(s.kernel_data);
        load_tss(s.tss);
    }
}

pub fn selectors() -> &'static Selectors {
    &GDT.get().expect("gdt not initialized").1
}
