#![no_std]

pub mod heap;
pub mod phys;
pub mod virt;

pub use phys::{alloc_page, free_page};
pub use virt::{map_page, unmap_page};

use crate::{heap::init_heap, phys::init_physical_memory, virt::init_kernel_mapper};

pub fn init_memory() {
    init_physical_memory();
    init_kernel_mapper();
    init_heap();
}
