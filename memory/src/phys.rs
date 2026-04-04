use limine::request::MemmapRequest;
use logging::{debug, error, info};
use spin::Mutex;

#[used]
#[unsafe(link_section = ".limine_requests")]
static MEMMAP_REQUEST: MemmapRequest = MemmapRequest::new();

pub const PAGE_SIZE: usize = 4096;

pub struct PhysMemoryManager {
    bitmap: &'static mut [u8],
    min_addr: u64,
}

impl PhysMemoryManager {
    fn page_idx(&self, phys: u64) -> usize {
        ((phys - self.min_addr) / PAGE_SIZE as u64) as usize
    }

    pub fn alloc_page(&mut self) -> Option<u64> {
        for (byte_idx, byte) in self.bitmap.iter_mut().enumerate() {
            if *byte != 0xff {
                for bit in 0..8 {
                    if (*byte >> bit) & 1 == 0 {
                        *byte |= 1 << bit;
                        let page_idx = byte_idx * 8 + bit;
                        let phys = self.min_addr + page_idx as u64 * PAGE_SIZE as u64;
                        return Some(phys);
                    }
                }
            }
        }
        None
    }

    pub fn free_page(&mut self, phys: u64) {
        let idx = self.page_idx(phys);
        let byte_idx = idx / 8;
        let bit_idx = idx % 8;
        self.bitmap[byte_idx] &= !(1 << bit_idx);
    }
}

static PHYS_ALLOC: Mutex<Option<PhysMemoryManager>> = Mutex::new(None);

pub(crate) fn init_physical_memory() {
    let response = match MEMMAP_REQUEST.response() {
        Some(response) => {
            info!("MEMMAP_REQUEST加载成功");
            response
        }
        None => {
            error!("MEMMAP_REQUEST加载失败");
            panic!();
        }
    };

    let mut min_addr = u64::MAX;
    let mut max_addr = 0u64;
    for entry in response.entries() {
        let start = entry.base;
        let end = entry.base + entry.length;
        if start < min_addr {
            min_addr = start;
        }
        if end > max_addr {
            max_addr = end;
        }
    }
    min_addr = (min_addr / PAGE_SIZE as u64) * PAGE_SIZE as u64;
    max_addr = (max_addr + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64 * PAGE_SIZE as u64;

    let total_pages = ((max_addr - min_addr) / PAGE_SIZE as u64) as usize;
    let bitmap_bytes = (total_pages + 7) / 8;


    let bitmap_base  = response
        .entries()
        .into_iter()
        .filter(|e| e.type_ == limine::memmap::MEMMAP_USABLE)
        .find(|e| e.length >= bitmap_bytes as u64)
        .expect("No suitable region for bitmap")
        .base;

    let bitmap_slice =
        unsafe { core::slice::from_raw_parts_mut(bitmap_base as *mut u8, bitmap_bytes) };

    let mgr = PhysMemoryManager {
        bitmap: bitmap_slice,
        min_addr,
    };

    for byte in mgr.bitmap.iter_mut() {
        *byte = 0xff;
    }

    debug!("test");

    for entry in response.entries() {
        if entry.type_ == limine::memmap::MEMMAP_USABLE {
            let start = entry.base;
            let end = entry.base + entry.length;
            let mut addr = start;
            while addr < end {
                let idx = mgr.page_idx(addr);
                let byte_idx = idx / 8;
                let bit_idx = idx % 8;
                mgr.bitmap[byte_idx] &= !(1 << bit_idx);
                addr += PAGE_SIZE as u64;
            }
        }
    }

    let bitmap_pages = (bitmap_bytes + PAGE_SIZE - 1) / PAGE_SIZE;
    for i in 0..bitmap_pages {
        let page_addr = bitmap_base + i as u64 * PAGE_SIZE as u64;
        let idx = mgr.page_idx(page_addr);
        let byte_idx = idx / 8;
        let bit_idx = idx % 8;
        mgr.bitmap[byte_idx] |= 1 << bit_idx;
    }

    PHYS_ALLOC.lock().replace(mgr);
    info!("物理内存初始化成功");
}

pub fn alloc_page() -> Option<u64> {
    PHYS_ALLOC.lock().as_mut().and_then(|a| a.alloc_page())
}

pub fn free_page(phys: u64) {
    if let Some(a) = PHYS_ALLOC.lock().as_mut() {
        a.free_page(phys);
    }
}
