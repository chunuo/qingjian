//! 候选窗口主题：字体、颜色、间距。视觉层级对齐 macOS 端；GDI 没有随外观切换的语义色，浅 / 深各写一套（[`Palette`]）。

mod palette;

use qingjian_platform::{ThemeColor, ThemeConfig};
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DeleteObject,
    FF_DONTCARE, HFONT, OUT_TT_PRECIS, VARIABLE_PITCH,
};
use windows::core::{PCWSTR, w};

use self::palette::Palette;

/// 常规字重；windows crate 未导出。
const FW_NORMAL: i32 = 400;

/// COLORREF 低位到高位是 R、G、B。
pub(super) const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

/// 一套配色 + 按 DPI 造好的字体。字体是 GDI 资源，`Drop` 里删。
pub(crate) struct Theme {
    pub text_font: HFONT,

    pub annotation_font: HFONT,

    /// 候选行右侧译文 / 词性 / 码的字号（`[theme] gloss_size`）；缺省与 [`Self::annotation_font`] 同大小。
    /// 与它分开是让「调大译文」不影响顶部拼音行。
    pub gloss_font: HFONT,

    pub index_font: HFONT,

    /// 符号字体（状态条的齿轮 ⚙）：雅黑没有这些字形。
    pub symbol_font: HFONT,

    pub text_color: COLORREF,

    pub gloss_color: COLORREF,

    pub pos_color: COLORREF,

    /// 生词译文，比普通译文醒目。
    pub fresh_color: COLORREF,

    pub index_color: COLORREF,

    /// 云联想的云朵与文字。
    pub cloud_color: COLORREF,

    pub background: COLORREF,

    /// 当前候选的高亮底色（mac 的半透明蓝预混成不透明值，GDI 无 alpha）。
    pub highlight: COLORREF,

    /// 窗口内边距（已按 DPI 缩放）。
    pub padding: i32,

    /// 行内上下留白。
    pub row_padding: i32,

    /// 列间距。
    pub column_gap: i32,

    /// 窗口圆角半径。
    pub corner_radius: i32,
}

impl Theme {
    /// `dpi` 96 为 100%。`config` 是 `[theme]` 分节：只动译文字号与译文两色。
    pub(crate) fn new(dpi: u32, dark: bool, config: &ThemeConfig) -> Self {
        let scale = |px: i32| (px * dpi as i32) / 96;
        // 负高度 = 字符高度（不含内部行距）。
        let font = |px: i32| create_font(-scale(px), w!("Microsoft YaHei UI"));
        // 译文字号配了就用它（配置层已夹到 8–48）；没改就用内置的 12。
        let gloss_px = config.gloss_size().round() as i32;
        let palette = if dark {
            Palette::dark()
        } else {
            Palette::light()
        };
        Self {
            text_font: font(16),
            annotation_font: font(12),
            gloss_font: if config.gloss_size_changed() {
                font(gloss_px)
            } else {
                font(12)
            },
            index_font: font(11),
            symbol_font: create_font(-scale(15), w!("Segoe UI Symbol")),
            text_color: palette.text_color,
            gloss_color: config.gloss_color().map_or(palette.gloss_color, color_ref),
            pos_color: palette.pos_color,
            fresh_color: config.fresh_color().map_or(palette.fresh_color, color_ref),
            index_color: palette.index_color,
            cloud_color: palette.cloud_color,
            background: palette.background,
            highlight: palette.highlight,
            padding: scale(8),
            row_padding: scale(4),
            column_gap: scale(8),
            corner_radius: scale(8),
        }
    }
}

impl Drop for Theme {
    fn drop(&mut self) {
        for font in [
            self.text_font,
            self.annotation_font,
            self.gloss_font,
            self.index_font,
            self.symbol_font,
        ] {
            if !font.is_invalid() {
                let _ = unsafe { DeleteObject(font.into()) };
            }
        }
    }
}

/// 平台无关颜色 → COLORREF。
fn color_ref(color: ThemeColor) -> COLORREF {
    rgb(color.r, color.g, color.b)
}

/// 缺字由 GDI 字体链回落。`height` 为负的字符高度。
fn create_font(height: i32, face: PCWSTR) -> HFONT {
    unsafe {
        CreateFontW(
            height,
            0,
            0,
            0,
            FW_NORMAL,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_TT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            (VARIABLE_PITCH.0 | FF_DONTCARE.0) as u32,
            face,
        )
    }
}
