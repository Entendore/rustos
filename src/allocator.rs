use alloc::alloc::Layout;
use core::alloc::GlobalAlloc;
use core::sync::atomic::{AtomicUsize, Ordering};
use conquer_once::LateInit;
use spin::Mutex;
use x86_64::VirtAddr;

struct Heap {
    start: VirtAddr,
    size: usize,
    inner: linked_list_allocator::Heap,
}

static HEAP: Mutex<Heap> = Mutex::new(Heap {
    start: VirtAddr::zero(),
    size: 0,
    inner: linked_list_allocator::Heap::empty(),
});

pub static HEAP_READY: LateInit<bool> = LateInit::new();
static USED: AtomicUsize = AtomicUsize::new(0);

pub fn set_heap(start: VirtAddr, size: usize) {
    let mut h = HEAP.lock();
    h.start = start; h.size = size;
    unsafe { h.inner.init(start.as_mut_ptr(), size); }
    HEAP_READY.init(true);
}

pub fn init(_mem: &mut crate::memory::Memory) -> Result<(), &'static str> {
    if HEAP_READY.get().copied().unwrap_or(false) { Ok(()) } else { Err("heap not mapped") }
}

pub fn stats() -> (usize, usize) {
    let h = HEAP.lock();
    (h.size, USED.load(Ordering::Relaxed))
}

pub struct KernelAlloc;
unsafe impl GlobalAlloc for KernelAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = HEAP.lock().inner.allocate_first_fit(layout).map(|p| p.as_ptr()).unwrap_or(core::ptr::null_mut());
        if !ptr.is_null() {
            USED.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HEAP.lock().inner.deallocate(core::ptr::NonNull::new_unchecked(ptr), layout);
        USED.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[alloc_error_handler]
fn handle(layout: Layout) -> ! { panic!("alloc error: {layout:?}"); }

#[global_allocator]
static ALLOC: KernelAlloc = KernelAlloc;
