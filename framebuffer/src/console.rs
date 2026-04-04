//! 文本控制台模块（基于帧缓冲和字体）

use crate::Color;
use crate::font::Font;
use crate::pixel::FrameBuffer;
use core::fmt::{self, Write};
use limine::framebuffer;
use spin::{Mutex, Once};

/// 控制台结构
pub struct Console {
    fb: FrameBuffer,   // 帧缓冲（用于绘图）
    font: Font,        // 字体
    cursor_x: u64,     // 当前光标 X 坐标（像素）
    cursor_y: u64,     // 当前光标 Y 坐标（像素）
    fg_color: Color,   // 前景色
    bg_color: Color,   // 背景色
    width_chars: u64,  // 每行可容纳的字符数（屏幕宽度 / 字体宽度）
    height_chars: u64, // 屏幕可容纳的行数（屏幕高度 / 字体高度）
}

impl Console {
    /// 创建新的控制台
    pub fn new(fb: FrameBuffer, font: Font) -> Self {
        let fb_width = fb.width();
        let fb_height = fb.height();
        let font_width = font.width();
        let font_height = font.height();
        let width_chars = fb_width / font_width;
        let height_chars = fb_height / font_height;
        Self {
            fb,
            font,
            cursor_x: 0,
            cursor_y: 0,
            fg_color: Color::WHITE,
            bg_color: Color::BLACK,
            width_chars,
            height_chars,
        }
    }

    /// 设置前景色和背景色
    pub fn set_colors(&mut self, fg: Color, bg: Color) {
        self.fg_color = fg;
        self.bg_color = bg;
    }

    /// 清屏并重置光标
    pub fn clear(&mut self) {
        self.fb.clear(self.bg_color);
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    /// 滚动一行（将屏幕上移一行）
    fn scroll(&mut self) {
        let font_height = self.font.height();
        let fb_height = self.fb.height();
        let bytes_per_row = self.fb.pitch();

        // 计算需要滚动的区域：从第 1 行开始复制到第 0 行
        let src_y = font_height;
        let dst_y = 0;
        let copy_height = fb_height - font_height;
        let copy_bytes = copy_height * bytes_per_row;

        unsafe {
            let src_ptr = (self.fb.addr() + (src_y * bytes_per_row) as usize) as *mut u8;
            let dst_ptr = (self.fb.addr() + (dst_y * bytes_per_row) as usize) as *mut u8;
            core::ptr::copy(src_ptr, dst_ptr, copy_bytes as usize);
            // 清除最后一行
            let last_row_start = (fb_height - font_height) * bytes_per_row;
            // let last_row_ptr = (self.fb.addr() + last_row_start as usize) as *mut u8;
            for y in 0..font_height {
                for x in 0..self.fb.width() {
                    self.fb
                        .put_pixel(x, fb_height - font_height + y, self.bg_color);
                }
            }
        }
    }

    /// 换行处理
    fn newline(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += self.font.height();
        if self.cursor_y + self.font.height() > self.fb.height() {
            self.scroll();
            self.cursor_y -= self.font.height();
        }
    }

    pub fn putchar(&mut self, ch: char) {
        match ch {
            '\n' => self.newline(),
            '\r' => self.cursor_x = 0,
            '\t' => {
                let tab_width = 8 * self.font.width();
                self.cursor_x = ((self.cursor_x / tab_width) + 1) * tab_width;
                if self.cursor_x >= self.fb.width() {
                    self.newline();
                }
            }
            _ => {
                // 绘制字符
                self.font.draw_char(
                    &self.fb,
                    self.cursor_x,
                    self.cursor_y,
                    ch,
                    self.fg_color,
                    self.bg_color,
                );
                self.cursor_x += self.font.width();
                if self.cursor_x + self.font.width() > self.fb.width() {
                    self.newline();
                }
            }
        }
    }

    /// 输出字符串
    pub fn write_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.putchar(ch);
        }
    }
}

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

// 全局控制台单例（使用 Mutex 保证多线程安全）
static CONSOLE: Once<Mutex<Console>> = spin::Once::new();

/// 初始化全局控制台（必须在 kmain 中调用一次）
pub fn init_console(fb: FrameBuffer, font: Font) {
    CONSOLE.call_once(|| Mutex::new(Console::new(fb, font)));
}

/// 获取全局控制台的锁（用于内部宏）
fn with_console<F, R>(f: F) -> R
where
    F: FnOnce(&mut Console) -> R,
{
    let console = CONSOLE.get().expect("Console not initialized");
    let mut guard = console.lock();
    f(&mut guard)
}

/// 打印格式化字符串（内部使用）
#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    with_console(|console| {
        console.write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        ::framebuffer::_print(core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => {
        ::framebuffer::print!("\n")
    };
    ($($arg:tt)*) => {
        ::framebuffer::print!("{}\n", core::format_args!($($arg)*))
    };
}
