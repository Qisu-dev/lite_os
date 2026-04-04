#![no_std]
#![no_main]

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use core::arch::asm;
use framebuffer::{Font, console::init_console, pixel::FrameBuffer, println};
use limine::BaseRevision;
use limine::request::FramebufferRequest;
use logging::{debug, error, info, init_logger};
use memory::{
    alloc_page, free_page, init_memory, map_page, unmap_page,
    virt::{alloc_kernel_page, free_kernel_page, get_page_phys},
};
use x86_64::structures::paging::PageTableFlags;

#[used]
#[unsafe(link_section = ".limine_requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

const FONT_DATA: &[u8] = include_bytes!("../../fonts/Lat7-Terminus16.psf");

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    init_logger();
    info!("系统启动成功");
    let response = match FRAMEBUFFER_REQUEST.response() {
        Some(response) => {
            info!("FRAMEBUFFER_REQUEST加载成功");
            response
        },
        None => {
            error!("加载FRAMEBUFFER_REQUEST失败");
            panic!();
        }
    };
    debug!("init_memory前");
    init_memory();
    debug!("init_memory后");

    let fb_info = response.framebuffers()[0];
    let fb = FrameBuffer::from_limine_frame_buffer(&fb_info);

    let font = Font::from_bytes(FONT_DATA).unwrap_or_else(|| {
        error!("字体解析失败");
        panic!()
    });
    init_console(fb, font);

    test_heap_allocator();
    test_kernel_page_alloc();
    test_page_mapping();
    test_physical_pages();

    loop {}
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    println!("at {} : {}", info.location().unwrap(), info.message());
    hcf();
}

fn hcf() -> ! {
    loop {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            asm!("hlt");
            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
            asm!("wfi");
            #[cfg(target_arch = "loongarch64")]
            asm!("idle 0");
        }
    }
}

fn test_physical_pages() {
    info!("Testing physical page allocator...");
    let page1 = alloc_page().expect("alloc_page failed");
    info!("Allocated physical page at 0x{:x}", page1);
    let page2 = alloc_page().expect("alloc_page failed");
    info!("Allocated physical page at 0x{:x}", page2);
    assert!(page1 != page2, "Pages should be different");
    free_page(page1);
    info!("Freed page1");
    let page3 = alloc_page().expect("alloc_page failed");
    info!("Re-allocated physical page at 0x{:x}", page3);
    // 由于 page1 已释放，page3 很可能等于 page1（但并非绝对，取决于分配器实现）
    // 这里只检查能分配成功即可
    free_page(page2);
    free_page(page3);
    info!("Physical page allocator test passed");
}

fn test_page_mapping() {
    info!("Testing page mapping...");
    let phys = alloc_page().expect("alloc_page failed");
    let virt = 0xffff888000000000; // 选择临时虚拟地址（避免冲突）
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    map_page(virt, phys, flags);
    info!("Mapped 0x{:x} -> 0x{:x}", virt, phys);

    let retrieved_phys = get_page_phys(virt).expect("translate failed");
    assert_eq!(retrieved_phys, phys, "Physical address mismatch");
    info!("Translation verified");

    // 写入测试
    let ptr = virt as *mut u32;
    unsafe {
        ptr.write(0xdeadbeef);
    }
    unsafe {
        assert_eq!(ptr.read(), 0xdeadbeef, "Write/read failed");
    }
    info!("Write/read test passed");

    // 解除映射
    let freed_phys = unmap_page(virt);
    assert_eq!(freed_phys, phys);
    free_page(phys);
    info!("Page mapping test passed");
}

fn test_kernel_page_alloc() {
    info!("Testing kernel page allocator...");
    let virt1 = alloc_kernel_page(PageTableFlags::PRESENT | PageTableFlags::WRITABLE)
        .expect("alloc_kernel_page failed");
    info!("Allocated kernel page at 0x{:x}", virt1);
    // 写入数据
    let ptr = virt1 as *mut u32;
    unsafe {
        ptr.write(0xcafebabe);
    }
    // 释放
    free_kernel_page(virt1);
    info!("Kernel page allocator test passed");
}

fn test_heap_allocator() {
    info!("Testing heap allocator (Box, Vec)...");
    // 使用 Box
    let boxed = Box::new(42);
    assert_eq!(*boxed, 42);
    info!("Box test passed");

    // 使用 Vec
    let mut vec = Vec::new();
    for i in 0..100 {
        vec.push(i);
    }
    assert_eq!(vec.len(), 100);
    assert_eq!(vec.iter().sum::<i32>(), (0..100).sum());
    info!("Vec test passed");

    // 分配较大内存
    let large_vec = Vec::from_iter(0..10000);
    assert_eq!(large_vec.len(), 10000);
    info!("Large Vec test passed");
}
