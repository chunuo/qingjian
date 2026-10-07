//! 「候选窗口」页：外观、排布、渲染引擎、字体、译文小字、拼音显示位置、悬浮状态条。

use qingjian_platform::{
    CandidateRenderer, DEFAULT_FRESH_COLOR, DEFAULT_GLOSS_COLOR, LayoutMode, MAX_GLOSS_SIZE,
    MIN_GLOSS_SIZE, PreeditMode, ThemeColor, ThemeMode,
};
use windows_reactor::*;

use crate::panel::controls::{field, page};
use crate::panel::{Message, Settings};

/// 配置里的十六进制写法 → 颜色控件要的值；留空（跟随内置色）时用给定的内置色显示。
fn picker_color(value: &str, fallback: &str) -> Color {
    let hex = if value.trim().is_empty() {
        fallback
    } else {
        value.trim()
    };
    match ThemeColor::parse_hex(hex) {
        Some(ThemeColor { r, g, b, a }) => Color::argb(a, r, g, b),
        None => Color::rgb(0, 0, 0),
    }
}

/// 枚举下拉：按 `label()` 列项，选中 `current`（找不到取 0）。
fn mode_combo<T: PartialEq + Copy>(
    all: &'static [T],
    current: T,
    label: fn(T) -> &'static str,
    callback: Callback<Option<usize>>,
) -> ComboBox {
    ComboBox::new()
        .items_source(all.iter().map(|mode| label(*mode)))
        .selected_index(all.iter().position(|mode| *mode == current).unwrap_or(0))
        .on_selection_changed(callback)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let g = &settings.config.general;
    let t = &settings.config.theme;
    let font_text = settings
        .font_query
        .clone()
        .unwrap_or_else(|| g.font.clone());
    let query = font_text.to_lowercase();
    let suggestions: Vec<String> = settings
        .families
        .iter()
        .filter(|family| family.to_lowercase().contains(&query))
        .cloned()
        .collect();
    let rows = [
        field(
            "外观",
            "",
            mode_combo(
                &ThemeMode::ALL,
                g.theme,
                ThemeMode::label,
                context.callback(Message::Theme),
            ),
        ),
        field(
            "排布",
            "横排时只给高亮的候选显示译词。",
            mode_combo(
                &LayoutMode::ALL,
                g.layout,
                LayoutMode::label,
                context.callback(Message::Layout),
            ),
        ),
        field(
            "渲染引擎",
            "青简渲染器让候选窗口在各平台一致。",
            mode_combo(
                &CandidateRenderer::ALL,
                g.renderer,
                CandidateRenderer::label,
                context.callback(Message::Renderer),
            ),
        ),
        field(
            "字体",
            "只对青简渲染器生效；留空用系统字体，没装的字体自动回到系统字体。",
            AutoSuggestBox::new()
                .width(260.0)
                .text(font_text)
                .placeholder_text("系统字体")
                .items_source(suggestions)
                .on_text_changed(context.callback(Message::FontQuery))
                .on_suggestion_chosen(context.callback(Message::Font)),
        ),
        field(
            "译文字号",
            "候选词右侧那列小字（读音、词性、译文、辅码）的大小，8–48 点，缺省 12；\
             生词译文（浅色外观里那个橙色的词）也按这个大小画。候选词本身与顶部拼音行不变。",
            NumberBox::new()
                .minimum(MIN_GLOSS_SIZE as f64)
                .maximum(MAX_GLOSS_SIZE as f64)
                .value(t.gloss_size() as f64)
                .on_value_changed(context.callback(Message::GlossSize)),
        ),
        field(
            "普通译文颜色",
            "见过几轮、不再强调的译文用这个颜色；留空跟随内置色（浅色外观是灰的，深色外观更亮）。",
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .children([
                    ColorPicker::new()
                        .color(picker_color(&t.gloss_color, DEFAULT_GLOSS_COLOR))
                        .is_alpha_enabled(true)
                        .is_hex_input_visible(true)
                        .on_color_changed(context.callback(Message::GlossColor))
                        .into(),
                    Button::new()
                        .on_click(context.message(Message::ClearGlossColor))
                        .content("跟随内置色"),
                ]),
        ),
        field(
            "生词译文颜色",
            "还没见过几轮、需要强调的译文用这个颜色，缺省是橙的；留空跟随内置色。\
             想更醒目的就往深里调，看着晃眼就调浅。",
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .children([
                    ColorPicker::new()
                        .color(picker_color(&t.fresh_color, DEFAULT_FRESH_COLOR))
                        .is_alpha_enabled(true)
                        .is_hex_input_visible(true)
                        .on_color_changed(context.callback(Message::FreshColor))
                        .into(),
                    Button::new()
                        .on_click(context.message(Message::ClearFreshColor))
                        .content("跟随内置色"),
                ]),
        ),
        field(
            "译文外观",
            "字号与两个颜色一起回到内置值。",
            Button::new()
                .on_click(context.message(Message::ResetGloss))
                .content("恢复默认"),
        ),
        field(
            "拼音显示",
            "「只在候选窗口」时正在敲的拼音不显示在应用里，终端或行内拼音不正常的应用可以选它。",
            mode_combo(
                &PreeditMode::ALL,
                g.preedit,
                PreeditMode::label,
                context.callback(Message::Preedit),
            ),
        ),
        field(
            "悬浮状态条",
            "桌面上常驻、可拖动的小条：点「中 / 英」切换模式（开着双拼时还显示方案名），点「，。」切全角 / 半角标点，点齿轮打开设置。只在当前输入法是青简时显示，拖到哪下次还在哪。",
            ToggleSwitch::new()
                .is_on(settings.config.status_bar.enabled)
                .on_toggled(context.callback(Message::StatusBar)),
        ),
    ];
    page("候选窗口", StackPanel::new().spacing(16.0).children(rows))
}
