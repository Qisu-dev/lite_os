use core::sync::atomic::{AtomicU64, Ordering};

use logging::info;
use spin::{Mutex, Once};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB, mapper::TranslateError,
    },
};

use crate::{
    free_page,
    phys::{PAGE_SIZE, alloc_page},
};

/// 内核高半区直接映射偏移（Limine 默认将物理内存映射到虚拟地址 +0xffffffff80000000）
const HHDM_OFFSET: u64 = 0xffffffff80000000;

struct KernelFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for KernelFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        alloc_page().map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

pub struct KernelMapper {
    inner: OffsetPageTable<'static>,
    frame_allocator: KernelFrameAllocator, // 你需要实现的 FrameAllocator
}

impl KernelMapper {
    /// 初始化，从当前的页表 (CR3) 构建映射器
    pub fn init() -> Self {
        let (cr3, _) = x86_64::registers::control::Cr3::read();
        let phys_addr = cr3.start_address();
        let virt_addr = VirtAddr::new(phys_addr.as_u64() + HHDM_OFFSET);
        let page_table = unsafe { &mut *(virt_addr.as_mut_ptr::<PageTable>()) };
        let mapper = unsafe { OffsetPageTable::new(page_table, VirtAddr::new(HHDM_OFFSET)) };
        Self {
            inner: mapper,
            frame_allocator: KernelFrameAllocator,
        }
    }

    /// 建立从虚拟地址到物理地址的映射
    pub fn map_page(
        &mut self,
        virt_page: Page<Size4KiB>,
        phys_frame: PhysFrame<Size4KiB>,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        unsafe {
            self.inner
                .map_to(virt_page, phys_frame, flags, &mut self.frame_allocator)
                .map_err(|_| "Mapping failed")?
                .flush();
        }
        Ok(())
    }

    /// 取消映射
    pub fn unmap_page(
        &mut self,
        virt_page: Page<Size4KiB>,
    ) -> Result<PhysFrame<Size4KiB>, &'static str> {
        let (phys_frame, frame) = self
            .inner
            .unmap(virt_page)
            .map_err(|_| "Unmapping failed")?;
        frame.flush();
        Ok(phys_frame)
    }
}

static KERNEL_MAPPER: Once<Mutex<KernelMapper>> = Once::new();

pub fn init_kernel_mapper() {
    let mapper = KernelMapper::init();
    KERNEL_MAPPER.call_once(|| spin::Mutex::new(mapper));
    info!("虚拟内存初始化成功");
}

pub fn map_page(virt: u64, phys: u64, flags: PageTableFlags) {
    let mut mapper = KERNEL_MAPPER.get().expect("mapper not initialized").lock();
    let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virt));
    let frame = PhysFrame::containing_address(PhysAddr::new(phys));
    mapper.map_page(page, frame, flags).unwrap();
}

pub fn unmap_page(virt: u64) -> u64 {
    let mut mapper = KERNEL_MAPPER.get().expect("mapper not initialized").lock();
    let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virt));
    mapper.unmap_page(page).unwrap().start_address().as_u64()
}

/// 内核虚拟地址空间分配器（简单游标）
/// 从 KERNEL_VIRT_AREA_START 开始向上分配
const KERNEL_VIRT_AREA_START: u64 = 0xffff888000000000; // 与堆起始相同，但你可以另选区域
static NEXT_VIRT_ADDR: AtomicU64 = AtomicU64::new(KERNEL_VIRT_AREA_START);

/// 分配一个单独的内核页，返回其虚拟地址
pub fn alloc_kernel_page(flags: PageTableFlags) -> Option<u64> {
    let virt = NEXT_VIRT_ADDR.fetch_add(4096, Ordering::Relaxed);

    let phys = alloc_page()?;

    map_page(virt, phys, flags);

    Some(virt)
}

/// 释放内核页（解除映射并释放物理页）
pub fn free_kernel_page(virt: u64) {
    let phys = get_page_phys(virt).expect("Page not mapped");
    unmap_page(virt);
    free_page(phys);
}

pub fn get_page_phys(virt: u64) -> Result<u64, TranslateError> {
    let mapper = KERNEL_MAPPER.get().expect("mapper not initialized").lock();
    let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virt));
    mapper
        .inner
        .translate_page(page)
        .map(|frame| frame.start_address().as_u64())
}

pub fn alloc_kernel_heap(size: usize) -> Option<usize> {
    let start = NEXT_VIRT_ADDR.fetch_add(size as u64, Ordering::Relaxed);
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    for offset in (0..size).step_by(PAGE_SIZE) {
        let virt = start + offset as u64;
        let phys = alloc_page()?;
        map_page(virt, phys, flags);
    }
    Some(start as usize)
}
