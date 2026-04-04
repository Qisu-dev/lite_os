use logging::{debug, info};
use spin::{Mutex, Once};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// red
    pub r: u8,
    /// green
    pub g: u8,
    /// blue
    pub b: u8,
}

impl Color {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const RED: Self = Self { r: 255, g: 0, b: 0 };
    pub const GREEN: Self = Self { r: 0, g: 255, b: 0 };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 255 };

    #[inline]
    pub const fn to_rgba(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32) | 0xFF000000
    }

    #[inline]
    pub const fn to_bgra(self) -> u32 {
        ((self.b as u32) << 16) | ((self.g as u32) << 8) | (self.r as u32) | 0xFF000000
    }
}

#[derive(Debug, Clone, Copy)]
struct FramebufferInner {
    pub addr: usize, // 显存基地址
    pub width: u64,  // 屏幕宽度（像素）
    pub height: u64, // 屏幕高度（像素）
    pub pitch: u64,  // 每行字节数
    pub bpp: u16,    // 每像素位数（通常为 32）
}

impl FramebufferInner {
    /// 绘制一个像素
    pub unsafe fn put_pixel(&self, x: u64, y: u64, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = y * self.pitch + x * (self.bpp as u64 / 8);
        let ptr = (self.addr + offset as usize) as *mut u32;
        unsafe { ptr.write(color.to_rgba()) };
    }

    pub unsafe fn clear(&self, color: Color) {
        let total_pixels = self.width * self.height;
        let pixel_value = color.to_rgba();
        let ptr = self.addr as *mut u32;
        for i in 0..total_pixels {
            unsafe { ptr.add(i as usize).write(pixel_value) };
        }
    }
}

#[derive(Debug)]
pub struct FrameBuffer {
    inner: Mutex<FramebufferInner>,
}

impl FrameBuffer {
    pub fn from_limine_frame_buffer(fb: &limine::framebuffer::Framebuffer) -> Self {
        Self {
            inner: Mutex::new(FramebufferInner {
                addr: fb.address() as usize,
                width: fb.width,
                height: fb.height,
                pitch: fb.pitch,
                bpp: fb.bpp,
            }),
        }
    }

    pub fn put_pixel(&self, x: u64, y: u64, color: Color) {
        let guard = self.inner.lock();
        unsafe {
            guard.put_pixel(x, y, color);
        }
    }

    pub fn clear(&self, color: Color) {
        let guard = self.inner.lock();
        unsafe {
            guard.clear(color);
        }
    }

    pub fn width(&self) -> u64 {
        self.inner.lock().width
    }

    pub fn height(&self) -> u64 {
        self.inner.lock().height
    }

    pub fn pitch(&self) -> u64 {
        self.inner.lock().pitch
    }

    pub fn addr(&self) -> usize {
        self.inner.lock().addr
    }
}