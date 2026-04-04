use crate::{Color, pixel::FrameBuffer};

/// 字体数据
pub struct Font {
    /// 字体原始数据
    data: &'static [u8],
    /// 每个字符的宽度（像素）
    width: u64,
    /// 每个字符的高度（像素）
    height: u64,
    /// 字符数据起始偏移（跳过头部）
    glyph_offset: u64,
    /// 每个字符占用的字节数
    bytes_per_glyph: u64,
}

#[repr(C, packed)]
struct Psf1Header {
    magic: [u8; 2],
    mode: u8,
    charsize: u8,
}

/// PSF2 头部结构（32 字节）
#[repr(C, packed)]
struct Psf2Header {
    magic: [u8; 4],
    version: u32,
    headersize: u32,
    flags: u32,
    length: u32,   // 字符数据总字节数
    charsize: u32, // 每个字符的字节数
    height: u32,   // 字符高度（像素）
    width: u32,    // 字符宽度（像素）
}

const PSF1_MAGIC: [u8; 2] = [0x36, 0x04];
const PSF2_MAGIC: [u8; 4] = [0x72, 0xb5, 0x4a, 0x86];

impl Font {
    pub fn from_bytes(data: &'static [u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }

        if data[0] == PSF1_MAGIC[0] && data[1] == PSF1_MAGIC[1] {
            let header = unsafe { &*(data.as_ptr() as *const Psf1Header) };
            let width = 8; // PSF1 固定宽度为 8
            let height = header.charsize as u64;
            let glyph_offset = core::mem::size_of::<Psf1Header>() as u64;
            let bytes_per_glyph = height;
            return Some(Font {
                data,
                width,
                height,
                glyph_offset,
                bytes_per_glyph,
            });
        }

        if data.len() >= 4 && data[0..4] == PSF2_MAGIC {
            let header = unsafe { &*(data.as_ptr() as *const Psf2Header) };
            let width = header.width as u64;
            let height = header.height as u64;
            let glyph_offset = header.headersize as u64;
            let bytes_per_glyph = header.charsize as u64;
            return Some(Font {
                data,
                width,
                height,
                glyph_offset,
                bytes_per_glyph,
            });
        }

        None
    }

    pub fn width(&self) -> u64 {
        self.width
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    fn glyph_data(&self, ch: char) -> Option<&[u8]> {
        let code = ch as u64;
        let start = (self.glyph_offset + code * self.bytes_per_glyph) as usize;
        if start + self.bytes_per_glyph as usize > self.data.len() {
            return None; // 超出范围，显示空白
        }
        Some(&self.data[start..start + self.bytes_per_glyph as usize])
    }

    pub fn draw_char(&self, fb: &FrameBuffer, x: u64, y: u64, ch: char, fg: Color, bg: Color) {
        let glyph = match self.glyph_data(ch) {
            Some(g) => g,
            None => return,
        };
        let width = self.width;
        let height = self.height;
        let fb_width = fb.width(); // 需要 FrameBuffer 提供 width() 方法
        let fb_height = fb.height();

        if x >= fb_width || y >= fb_height {
            return;
        }

        let bytes_per_row = (width + 7) / 8;
        let max_draw_width = core::cmp::min(width, fb_width - x);
        let max_draw_height = core::cmp::min(height, fb_height - y);

        for row in 0..max_draw_height {
            let row_start = (row * bytes_per_row) as usize;
            for col in 0..max_draw_width {
                let byte_idx = row_start + (col / 8) as usize;
                let bit_mask = 1 << (7 - (col % 8));
                let bit = (glyph[byte_idx] & bit_mask) != 0;
                let color = if bit { fg } else { bg };
                fb.put_pixel(x + col, y + row, color);
            }
        }
    }

    /// 绘制字符串（不处理换行）
    pub fn draw_string(&self, fb: &FrameBuffer, x: u64, y: u64, s: &str, fg: Color, bg: Color) {
        let mut cx = x;
        for ch in s.chars() {
            self.draw_char(fb, cx, y, ch, fg, bg);
            cx += self.width;
        }
    }
}
