use linked_list_allocator::LockedHeap;
use crate::virt::alloc_kernel_heap;
use logging::info;

pub const HEAP_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub(crate) fn init_heap() {
    let heap_start = alloc_kernel_heap(HEAP_SIZE).expect("Failed to allocate heap");
    unsafe {
        ALLOCATOR.lock().init(heap_start as *mut u8, HEAP_SIZE);
    }
    info!("堆初始化成功 0x{:x}", heap_start);
}
