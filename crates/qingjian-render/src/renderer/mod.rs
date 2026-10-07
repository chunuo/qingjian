//! 渲染器：一帧 + 排布 + 主题 → 位图。排版逻辑与 macOS 壳的 `CandidateView` 一致：顶部拼音行，竖排一行一个候选、横排排成一行。
//!
//! 内部全用像素：主题里的点数进来先乘缩放倍数。文字的 y 都指行框顶边，字形在行高里垂直居中。

mod columns;
mod horizontal;
mod item;
mod matrix;
mod rendered;
mod status;
mod top_line;
mod vertical;

use crate::canvas::Canvas;
use crate::cloud::draw_cloud;
use crate::color::Color;
use crate::error::RenderError;
use crate::fonts::FontLibrary;
use crate::frame::{Frame, Row, Tone};
use crate::layout::Layout;
use crate::shadow::Shadow;
use crate::text::{TextPainter, TextSize, TextStyle};
use crate::theme::{FontSpec, Theme};

pub use rendered::Rendered;
pub use status::{RenderedStatus, StatusCell};

/// preedit 光标的宽度（点）。
const CARET_WIDTH: f32 = 1.5;

/// 云朵图标边长（点）。
const CLOUD_SIZE: f32 = 13.0;

/// 云朵与后面文字的间距（点）。
const CLOUD_GAP: f32 = 4.0;

/// preedit 与右侧整句补全之间的间距（点）。
const SENTENCE_GAP: f32 = 16.0;

/// 横排时序号与候选词之间的间距（点）。
const INDEX_GAP: f32 = 3.0;

/// 横排时高亮底色在候选两侧多出的宽度（点）。
const HIGHLIGHT_INSET: f32 = 5.0;

/// 光学字号（点）：20 pt 以下 CoreText 给系统字体用的就是这一档。
const OPTICAL_SIZE: f32 = 17.0;

/// 竖排候选窗口的最小宽度（点）。
const MIN_VERTICAL_WIDTH: f32 = 200.0;

pub struct Renderer {
    /// 文字测绘。
    text: TextPainter,
}

/// 一次渲染期间的上下文：主题按倍数换算后的像素值。
pub(super) struct Metrics<'a> {
    pub(super) theme: &'a Theme,
    pub(super) scale: f32,
}

impl Metrics<'_> {
    pub(super) fn px(&self, points: f32) -> f32 {
        points * self.scale
    }

    pub(super) fn padding(&self) -> f32 {
        self.px(self.theme.padding)
    }

    fn row_padding(&self) -> f32 {
        self.px(self.theme.row_padding)
    }

    fn column_gap(&self) -> f32 {
        self.px(self.theme.column_gap)
    }

    pub(super) fn corner_radius(&self) -> f32 {
        self.px(self.theme.corner_radius)
    }

    pub(super) fn style(&self, font: FontSpec, color: Color) -> TextStyle {
        TextStyle::new(
            font.scaled(self.scale),
            font.size,
            color,
            self.theme.text_gamma,
        )
    }

    pub(super) fn text_style(&self) -> TextStyle {
        self.style(self.theme.text_font, self.theme.colors.text)
    }

    /// 顶部拼音行 / 提示行的小字。
    fn annotation_style(&self, color: Color) -> TextStyle {
        self.style(self.theme.annotation_font, color)
    }

    /// 候选行右侧的译文 / 词性 / 码；字号可被用户配置覆盖（[`Theme::gloss_font`]），与拼音行分开。
    fn gloss_style(&self, color: Color) -> TextStyle {
        self.style(self.theme.gloss_font, color)
    }

    fn index_style(&self) -> TextStyle {
        self.style(self.theme.index_font, self.theme.colors.index)
    }

    fn tone_color(&self, tone: Tone) -> Color {
        match tone {
            Tone::Gloss => self.theme.colors.gloss,
            Tone::Fresh => self.theme.colors.fresh,
            Tone::Faint => self.theme.colors.pos,
            Tone::Code => self.theme.colors.gloss,
        }
    }

    /// 一行 annotation 片段按色调取样式：译文 / 生词 / 码用可调的 `gloss_font`，词性用固定小字。
    fn tone_style(&self, tone: Tone) -> TextStyle {
        self.style(self.tone_font(tone), self.tone_color(tone))
    }

    /// 某段 annotation 用哪个字号：译文 / 生词 / 码是被用户调过的 [`Theme::gloss_font`]，词性用固定小字。
    fn tone_font(&self, tone: Tone) -> FontSpec {
        match tone {
            Tone::Gloss | Tone::Fresh | Tone::Code => self.theme.gloss_font,
            Tone::Faint => self.theme.annotation_font,
        }
    }

    /// 一行 annotation 占的高度（像素）：取各段里最高的那个；没有段时退回固定小字。
    fn annotation_line_height(&self, annotation: &[(String, Tone)]) -> f32 {
        annotation
            .iter()
            .map(|(_, tone)| self.px(self.tone_font(*tone).line_height))
            .fold(self.px(self.theme.annotation_font.line_height), f32::max)
    }

    /// 一个候选行的高度（像素，不含 [`Theme::row_padding`]）：候选词与译文里高的那个。
    /// 译文调大后会超过候选词，行高不跟着长就会把下一行压上来。
    fn row_height(&self, text_height: f32, annotation: &[(String, Tone)]) -> f32 {
        text_height.max(self.annotation_line_height(annotation))
    }

    /// 一段小字相对候选词往下挪多少，让两者底部大致对齐。`line_height` 是它自己的行高（点）。
    fn small_offset(&self, text_height: f32, line_height: f32) -> f32 {
        (text_height - self.px(line_height)).max(0.0)
    }

    /// 某个色调的 annotation 相对候选词的行内偏移。
    fn tone_offset(&self, tone: Tone, text_height: f32) -> f32 {
        self.small_offset(text_height, self.tone_font(tone).line_height)
    }

    /// 译文小字相对候选词往下挪多少（按 [`Theme::gloss_font`] 的行高）。
    fn gloss_offset(&self, text_height: f32) -> f32 {
        self.small_offset(text_height, self.theme.gloss_font.line_height)
    }

    /// 序号相对候选词往下挪多少。
    fn index_offset(&self, text_height: f32) -> f32 {
        self.small_offset(text_height, self.theme.index_font.line_height)
    }

    /// 云朵图标占的宽度（含后面的间距）。
    fn cloud_width(&self) -> f32 {
        self.px(CLOUD_SIZE + CLOUD_GAP)
    }
}

impl Renderer {
    pub fn new(library: FontLibrary) -> Self {
        let mut text = TextPainter::new(library);
        text.set_optical_size(Some(OPTICAL_SIZE));
        Self { text }
    }

    /// 画一帧。`scale` 是点 → 像素的倍数（Retina 为 2）；带 `shadow` 时位图四周留出阴影的边。
    pub fn render(
        &mut self,
        frame: &Frame,
        layout: Layout,
        theme: &Theme,
        scale: f32,
        shadow: Option<&Shadow>,
    ) -> Result<Rendered, RenderError> {
        let metrics = Metrics { theme, scale };
        let (content_width, content_height) = self.preferred_size(frame, layout, &metrics);
        let margin = shadow.map_or(0.0, |s| metrics.px(s.margin()));
        let width = (content_width + margin * 2.0).ceil();
        let height = (content_height + margin * 2.0).ceil();
        let mut canvas = Canvas::new(width as u32, height as u32)?;
        let radius = metrics.corner_radius();
        if let Some(shadow) = shadow
            && let Some(content) =
                tiny_skia::Rect::from_xywh(margin, margin, content_width, content_height)
        {
            shadow.paint(&mut canvas, content, radius, scale);
        }
        canvas.fill_round_rect(
            margin,
            margin,
            content_width,
            content_height,
            radius,
            theme.colors.background,
        );
        let mut y = margin + metrics.padding();
        y += self.draw_top_line(&mut canvas, frame, &metrics, margin, y);
        match layout {
            Layout::Vertical => {
                self.draw_vertical(&mut canvas, frame, &metrics, margin, y, content_width);
            }
            Layout::Horizontal if frame.columns > 0 => {
                self.draw_matrix(&mut canvas, frame, &metrics, margin, y, content_width);
            }
            Layout::Horizontal => {
                self.draw_horizontal(&mut canvas, frame, &metrics, margin, y, content_width);
            }
        }
        Ok(Rendered {
            pixmap: canvas.into_pixmap(),
            content_x: margin as u32,
            content_y: margin as u32,
            content_width: content_width.ceil() as u32,
            content_height: content_height.ceil() as u32,
            scale,
        })
    }

    /// 量一段文字在 `size` 点字号下的宽度（点），与原生排版的数值对照用。
    pub fn measure_points(&mut self, text: &str, size: f32) -> f32 {
        let style = TextStyle::new(FontSpec::new(size, size), size, Color::rgb(0, 0, 0), 1.0);
        self.text.measure(text, &style).width
    }

    /// 每个字形用到的字族名，验证回退链用。
    pub fn trace_families(&mut self, text: &str, theme: &Theme) -> Vec<String> {
        let metrics = Metrics { theme, scale: 1.0 };
        self.text.trace_families(text, &metrics.text_style())
    }

    /// 内容需要的像素宽高（不含阴影边）。
    fn preferred_size(&mut self, frame: &Frame, layout: Layout, m: &Metrics) -> (f32, f32) {
        let (top_width, top_height) = self.top_line_size(frame, m);
        let (body_width, body_height) = match layout {
            Layout::Vertical => self.vertical_size(frame, m),
            Layout::Horizontal if frame.columns > 0 => self.matrix_size(frame, m),
            Layout::Horizontal => self.horizontal_size(frame, m),
        };
        let width = top_width.max(body_width) + m.padding() * 2.0;
        // 竖排时候选都很短（没有译词）窗口会窄得难看，给个下限
        let width = match layout {
            Layout::Vertical => width.max(m.px(MIN_VERTICAL_WIDTH)),
            Layout::Horizontal => width,
        };
        (width, top_height + body_height + m.padding() * 2.0)
    }

    pub(super) fn measure(&mut self, text: &str, style: &TextStyle) -> TextSize {
        self.text.measure(text, style)
    }

    /// 画一段文字（`x` 左边、`y` 行框顶边），返回它的宽度。
    pub(super) fn draw_text(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        style: &TextStyle,
        x: f32,
        y: f32,
    ) -> f32 {
        self.text.draw(canvas, text, style, x, y)
    }

    /// 画云朵，返回占用宽度（含间距）。`top` 是所在行文字的顶边，`line_height` 用来垂直居中。
    fn draw_cloud(
        &mut self,
        canvas: &mut Canvas,
        m: &Metrics,
        x: f32,
        top: f32,
        line_height: f32,
    ) -> f32 {
        let size = m.px(CLOUD_SIZE);
        draw_cloud(
            canvas,
            x,
            top + (line_height - size) / 2.0,
            size,
            m.theme.colors.cloud,
        );
        m.cloud_width()
    }

    /// 候选词本体：云端词前带云朵、换颜色。
    fn draw_word(
        &mut self,
        canvas: &mut Canvas,
        m: &Metrics,
        row: &Row,
        x: f32,
        top: f32,
        text_height: f32,
    ) {
        let mut word_x = x;
        if row.cloud {
            word_x += self.draw_cloud(canvas, m, word_x, top, text_height);
        }
        let color = if row.cloud {
            m.theme.colors.cloud
        } else {
            m.theme.colors.text
        };
        let style = m.style(m.theme.text_font, color);
        word_x += self.draw_text(canvas, &row.text, &style, word_x, top);
        if let Some(code) = &row.code {
            let style = m.gloss_style(m.tone_color(Tone::Code));
            self.draw_text(
                canvas,
                code,
                &style,
                word_x,
                top + m.gloss_offset(text_height),
            );
        }
    }

    /// 候选词后面那段码的宽度；没有码是 0。
    fn code_width(&mut self, row: &Row, m: &Metrics) -> f32 {
        let Some(code) = &row.code else {
            return 0.0;
        };
        let style = m.gloss_style(m.tone_color(Tone::Code));
        self.measure(code, &style).width
    }

    fn fill_highlight(
        &mut self,
        canvas: &mut Canvas,
        m: &Metrics,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        canvas.fill_round_rect(
            x,
            y,
            width,
            height,
            m.corner_radius() / 2.0,
            m.theme.colors.highlight,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::Renderer;
    use crate::fonts::FontLibrary;
    use crate::frame::{Frame, Preedit, Row, Tone};
    use crate::layout::Layout;
    use crate::theme::{FontSpec, Theme, ThemeOverrides};

    /// 一行普通译文 + 一行生词译文，竖排时每行都带 annotation。
    fn frame() -> Frame {
        let annotated = |index: usize, text: &str, annotation: &[(&str, Tone)]| Row {
            index: (index + 1).to_string(),
            text: text.to_owned(),
            code: None,
            annotation: annotation
                .iter()
                .map(|(text, tone)| ((*text).to_owned(), *tone))
                .collect(),
            cloud: false,
        };
        Frame {
            preedit: Some(Preedit::plain("ni'hao", 6)),
            rows: vec![
                annotated(0, "你好", &[("int. ", Tone::Faint), ("hello", Tone::Gloss)]),
                annotated(
                    1,
                    "你好像",
                    &[("phr. ", Tone::Faint), ("you seem", Tone::Fresh)],
                ),
                annotated(2, "你好好", &[]),
            ],
            highlighted: Some(0),
            footer: Some("1/3".to_owned()),
            ..Frame::default()
        }
    }

    fn overrides(size: f32) -> ThemeOverrides {
        ThemeOverrides {
            gloss_size: Some(size),
            ..ThemeOverrides::default()
        }
    }

    #[test]
    fn bigger_gloss_grows_vertical_rows() {
        // 没有系统字体的环境（CI 容器）跳过
        let Ok(library) = FontLibrary::system("zh-CN") else {
            return;
        };
        let mut renderer = Renderer::new(library);
        let mut height = |theme: &Theme| {
            renderer
                .render(&frame(), Layout::Vertical, theme, 2.0, None)
                .unwrap()
                .content_size_points()
                .1
        };
        let base = height(&Theme::light());
        // 12 pt 的译文比 16 pt 的候选词矮，行高只看候选词；调大后译文更高的那一行必须把行高撑起来
        let big = height(&Theme::light().with_overrides(&overrides(24.0)));
        assert!(big > base, "译文调大后竖排高度该跟着长：{base} → {big}");
    }

    #[test]
    fn bigger_gloss_grows_horizontal_highlight_row() {
        let Ok(library) = FontLibrary::system("zh-CN") else {
            return;
        };
        let mut renderer = Renderer::new(library);
        let mut height = |theme: &Theme| {
            renderer
                .render(&frame(), Layout::Horizontal, theme, 2.0, None)
                .unwrap()
                .content_size_points()
                .1
        };
        let base = height(&Theme::light());
        let big = height(&Theme::light().with_overrides(&overrides(24.0)));
        assert!(
            big > base,
            "译文调大后横排高亮行的译文行该跟着长：{base} → {big}"
        );
    }

    #[test]
    fn gloss_override_keeps_annotation_font_untouched() {
        // 拼音行 / 词性用的是 annotation_font，不该被译文字号覆盖带着一起放大
        let theme = Theme::light().with_overrides(&overrides(24.0));
        assert_eq!(theme.annotation_font, FontSpec::new(12.0, 15.0));
        assert_eq!(theme.gloss_font.size, 24.0);
        assert!(theme.gloss_font.line_height > 24.0);
    }
}
