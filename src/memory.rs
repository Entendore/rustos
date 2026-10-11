use bootloader::boot_info::{BootInfo, MemoryRegionKind};
use x86_64::structures::paging::{OffsetPageTable, PageTable, PhysFrame, Mapper, FrameAllocator};
use x86_64::{PhysAddr, VirtAddr, PageSize, Size4KiB};

pub const PHYSICAL_OFFSET: u64 = 0xFFFF_8880_0000_0000;

pub struct Memory {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BitmapFrameAllocator,
}

pub fn init(boot_info: &'static BootInfo) -> Memory {
    let mut mapper = unsafe { active_offset_table() };
    let mut frame_allocator = unsafe { BitmapFrameAllocator::from_boot_info(boot_info) };
    
    use x86_64::structures::paging::Page;
    let heap_start = VirtAddr::new(0xFFFF_AAAA_0000_0000);
    for i in 0..512 {
        let page = Page::<Size4KiB>::containing_address(heap_start + i * 4096);
        let frame = match frame_allocator.allocate_frame() {
            Some(f) => f,
            None => panic!("oom mapping heap"),
        };
        unsafe {
            mapper.map_to(page, frame,
                x86_64::structures::paging::PageTableFlags::PRESENT
                    | x86_64::structures::paging::PageTableFlags::WRITABLE,
                &mut frame_allocator).unwrap().flush();
        }
    }
    crate::allocator::set_heap(heap_start, 512 * 4096);
    Memory { mapper, frame_allocator }
}

unsafe fn active_offset_table() -> OffsetPageTable<'static> {
    let (l4_frame, _) = x86_64::registers::control::Cr3::read();
    let phys = l4_frame.start_address();
    let virt = VirtAddr::new(phys.as_u64() + PHYSICAL_OFFSET);
    let table: *mut PageTable = virt.as_mut_ptr();
    unsafe { OffsetPageTable::new(&mut *table, VirtAddr::new(PHYSICAL_OFFSET)) }
}

pub struct BitmapFrameAllocator {
    bitmap: &'static mut [u64],
    region_base: PhysAddr,
    region_len: usize,
}

impl BitmapFrameAllocator {
    pub unsafe fn from_boot_info(boot_info: &'static BootInfo) -> Self {
        let regions = boot_info.memory_map.iter().filter(|r|
            r.kind == MemoryRegionKind::Usable);
        let mut total_frames: usize = 0;
        let mut start: Option<PhysAddr> = None;
        for r in regions.clone() {
            total_frames += r.end - r.start;
            if start.is_none() { start = Some(PhysAddr::new((r.start * 4096) as u64)); }
        }
        let start = start.expect("no usable memory");
        let bitmap_words = total_frames.div_ceil(64);
        let bitmap_size = bitmap_words * 8;
        let bitmap_virt = VirtAddr::new(start.as_u64() + PHYSICAL_OFFSET);
        let bitmap = unsafe { 
            core::slice::from_raw_parts_mut(
                bitmap_virt.as_u64() as *mut u64, bitmap_words
            ) 
        };
        for w in bitmap.iter_mut() { *w = 0; }
        Self {
            bitmap,
            region_base: start,
            region_len: total_frames * 4096,
        }
    }

    fn idx_of(&self, frame: PhysFrame) -> usize {
        (frame.start_address().as_u64() - self.region_base.as_u64()) as usize / 4096
    }

    fn frame_at(&self, idx: usize) -> PhysFrame {
        let addr = self.region_base + (idx * 4096) as u64;
        PhysFrame::containing_address(addr)
    }
}

unsafe impl FrameAllocator<Size4KiB> for BitmapFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        for (word_idx, word) in self.bitmap.iter_mut().enumerate() {
            if *word != u64::MAX {
                let bit = word.trailing_ones() as usize;
                *word |= 1u64 << bit;
                let frame = self.frame_at(word_idx * 64 + bit);
                return Some(frame);
            }
        }
        None
    }

    fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let i = self.idx_of(frame);
        let (w, b) = (i / 64, i % 64);
        self.bitmap[w] &= !(1u64 << b);
    }
}
