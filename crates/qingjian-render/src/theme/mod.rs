//! 主题：字体、颜色、间距。所有可视参数都在这里，单位是点；将来从 TOML 读。
//!
//! 视觉层级（产品决定）：候选词最深，译文稍浅，词性最浅，序号弱化。数值对齐 macOS 壳的 AppKit 实现。

mod font_spec;
mod palette;

use crate::color::Color;

pub use font_spec::FontSpec;
pub use palette::Palette;

/// 译文小字的行高相对字号的倍数。内置 12 pt 字号配 15 pt 行高，用户改字号时按同一比例放行高。
const GLOSS_LINE_RATIO: f32 = 15.0 / 12.0;

/// 主题里可由用户覆盖的项（配置 `[theme]` 分节）。`None` 用内置值；只覆盖译文相关的几项，
/// 其余留白给以后的完整主题文件。见 `docs/design/candidate-ui.md`。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ThemeOverrides {
    /// 候选旁译文的字号（点）。`None` 用内置 12。
    pub gloss_size: Option<f32>,

    /// 普通译文颜色。`None` 用内置的次要标签灰。
    pub gloss_color: Option<Color>,

    /// 生词译文颜色（用户还没见过几轮、强调用）。`None` 用内置橙。
    pub fresh_color: Option<Color>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// 候选词字体。
    pub text_font: FontSpec,

    /// 译文与词性字体。
    pub annotation_font: FontSpec,

    /// 候选行里译文 / 词性 / 码的字号；用户可改（[`ThemeOverrides::gloss_size`]），
    /// 与顶部的拼音行分开，改译文大小不会顺带把拼音行放大。
    pub gloss_font: FontSpec,

    /// 序号字体。
    pub index_font: FontSpec,

    /// 配色。
    pub colors: Palette,

    /// 窗口内边距。
    pub padding: f32,

    /// 行内上下留白。
    pub row_padding: f32,

    /// 序号与候选词、候选词与译文之间的间距。
    pub column_gap: f32,

    /// 窗口与高亮条的圆角。
    pub corner_radius: f32,

    /// 最多显示几行。
    pub max_rows: usize,

    /// 文字抗锯齿覆盖率的 gamma：小于 1 笔画显粗。CoreText 对文字有一层类似的加深，深色背景上尤其明显，
    /// 线性混合出来的字会偏细；这个值按真机截图并排调。
    pub text_gamma: f32,
}

impl Theme {
    /// 浅色，对齐 macOS 系统外观。
    pub fn light() -> Self {
        Self::with_palette(Palette::light(), 0.85)
    }

    /// 深色，对齐 macOS 系统外观。
    pub fn dark() -> Self {
        Self::with_palette(Palette::dark(), 0.75)
    }

    fn with_palette(colors: Palette, text_gamma: f32) -> Self {
        Self {
            // 行高取 AppKit 系统字体在这几个字号下 NSAttributedString.size() 的高度
            text_font: FontSpec::new(16.0, 19.0),
            annotation_font: FontSpec::new(12.0, 15.0),
            gloss_font: FontSpec::new(12.0, 15.0),
            index_font: FontSpec::new(11.0, 14.0),
            colors,
            padding: 8.0,
            row_padding: 4.0,
            column_gap: 8.0,
            corner_radius: 8.0,
            max_rows: 9,
            text_gamma,
        }
    }

    /// 套用用户覆盖：只动译文相关的字号与颜色，其余保持不变；字号带上下限，避免配置里写疯值。
    pub fn with_overrides(mut self, overrides: &ThemeOverrides) -> Self {
        if let Some(size) = overrides.gloss_size {
            let size = size.clamp(MIN_GLOSS_SIZE, MAX_GLOSS_SIZE);
            self.gloss_font = FontSpec::new(size, size * GLOSS_LINE_RATIO);
        }
        if let Some(color) = overrides.gloss_color {
            self.colors.gloss = color;
        }
        if let Some(color) = overrides.fresh_color {
            self.colors.fresh = color;
        }
        self
    }
}

/// 译文字号的合法范围（点）：太小看不清、太大把候选窗撑坏。
pub const MIN_GLOSS_SIZE: f32 = 8.0;

/// 见 [`MIN_GLOSS_SIZE`]。
pub const MAX_GLOSS_SIZE: f32 = 48.0;
