//! `[theme]` 分节：候选行里译文小字的外观（字号与颜色）。
//!
//! 只覆盖译文那一列，其余（候选词、拼音行、序号、背景）仍是内置主题。颜色用十六进制写法
//! （`#RRGGBB` / `#RRGGBBAA`），留空跟随内置色，写错记一条警告再退回内置色。
//!
//! 这里只做配置写法与校验，返回平台无关的 [`ThemeColor`]；各壳自己换成渲染器的
//! `qingjian_render::ThemeOverrides`——本 crate 也被 TSF DLL 依赖，不能拉进渲染器的字体依赖树。

use serde::{Deserialize, Serialize};

/// 译文字号的最小值（点）。与渲染器侧 `qingjian_render::MIN_GLOSS_SIZE` 保持一致。
pub const MIN_GLOSS_SIZE: f32 = 8.0;

/// 译文字号的最大值（点）。太小看不清、太大把候选窗撑坏。
pub const MAX_GLOSS_SIZE: f32 = 48.0;

/// 主题内置的译文字号（点），也是「恢复默认」回的值。
pub const DEFAULT_GLOSS_SIZE: f32 = 12.0;

/// 缺省的生词译文颜色（浅色外观的内置橙）。深色外观内置 `#ff9230`，两者几乎一样，
/// 界面上的色块与「恢复默认」都用这个值。
pub const DEFAULT_FRESH_COLOR: &str = "#ff8d28";

/// 缺省的普通译文颜色（浅色外观的内置灰）；深色外观另有更亮的一套。
pub const DEFAULT_GLOSS_COLOR: &str = "#6b6b70";

/// 平台无关的 sRGB 颜色；壳转成渲染器的 `Color` 或 GDI 的 `COLORREF`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeColor {
    pub r: u8,

    pub g: u8,

    pub b: u8,

    /// 不透明度，255 为完全不透明。
    pub a: u8,
}

impl ThemeColor {
    /// 解析 `#RRGGBB` / `#RRGGBBAA`（`#` 可省，大小写不敏感）；写法不对返回 `None`。
    /// 设置界面把配置里的颜色串还原成控件值时也走这里。
    pub fn parse_hex(text: &str) -> Option<Self> {
        parse_hex(text)
    }
}

/// 译文外观的配置；各壳据此构造渲染器的覆盖项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// 候选行里译文的字号（点），缺省 12；合法范围见 [`MIN_GLOSS_SIZE`] / [`MAX_GLOSS_SIZE`]。
    /// 只影响译文那一列（读音、词性、译文、辅码），拼音行与候选词不变。
    pub gloss_size: f32,

    /// 普通译文颜色，写法 `#RRGGBB` 或 `#RRGGBBAA`；空串跟随内置色。
    pub gloss_color: String,

    /// 生词译文颜色（还没见过几轮、加粗强调的那个橙字），写法同上；空串跟随内置色。
    pub fresh_color: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            gloss_size: DEFAULT_GLOSS_SIZE,
            gloss_color: String::new(),
            fresh_color: String::new(),
        }
    }
}

impl ThemeConfig {
    /// 夹到合法范围的译文字号。
    pub fn gloss_size(&self) -> f32 {
        if self.gloss_size.is_finite() {
            self.gloss_size.clamp(MIN_GLOSS_SIZE, MAX_GLOSS_SIZE)
        } else {
            DEFAULT_GLOSS_SIZE
        }
    }

    /// 普通译文颜色；留空跟随内置色，写错警告后退回内置色。
    pub fn gloss_color(&self) -> Option<ThemeColor> {
        parse_color(&self.gloss_color, "译文")
    }

    /// 生词译文颜色；留空跟随内置色，写错警告后退回内置色。
    pub fn fresh_color(&self) -> Option<ThemeColor> {
        parse_color(&self.fresh_color, "生词译文")
    }

    /// 字号是否被改过（不等于缺省）；壳据此决定要不要套覆盖。
    pub fn gloss_size_changed(&self) -> bool {
        self.gloss_size() != DEFAULT_GLOSS_SIZE
    }

    /// 是否有任何一项被改过；全没改时壳直接用内置主题，省掉一次换算。
    pub fn any_changed(&self) -> bool {
        self.gloss_size_changed() || self.gloss_color().is_some() || self.fresh_color().is_some()
    }
}

/// 十六进制解析：`#RRGGBB` / `#RRGGBBAA`（`#` 可省，大小写不敏感）。
fn parse_hex(text: &str) -> Option<ThemeColor> {
    let hex = text.trim().trim_start_matches('#');
    let byte = |start: usize| u8::from_str_radix(hex.get(start..start + 2)?, 16).ok();
    match hex.len() {
        6 => Some(ThemeColor {
            r: byte(0)?,
            g: byte(2)?,
            b: byte(4)?,
            a: 255,
        }),
        8 => Some(ThemeColor {
            r: byte(0)?,
            g: byte(2)?,
            b: byte(4)?,
            a: byte(6)?,
        }),
        _ => None,
    }
}

/// 空串当作没填（跟随内置色）；非空但解析不了就记警告再跟随内置色。
fn parse_color(text: &str, what: &str) -> Option<ThemeColor> {
    match text.trim() {
        "" => None,
        value => match parse_hex(value) {
            Some(color) => Some(color),
            None => {
                tracing::warn!(value, "不认识的{what}颜色，用内置色");
                None
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_builtin_look() {
        let config = ThemeConfig::default();
        assert_eq!(config.gloss_size(), DEFAULT_GLOSS_SIZE);
        assert!(config.gloss_color().is_none());
        assert!(config.fresh_color().is_none());
        assert!(!config.any_changed());
    }

    #[test]
    fn parses_hex_with_or_without_hash() {
        let config = ThemeConfig {
            gloss_color: "#336699".to_owned(),
            fresh_color: "ff8d28".to_owned(),
            ..ThemeConfig::default()
        };
        assert_eq!(
            config.gloss_color(),
            Some(ThemeColor {
                r: 0x33,
                g: 0x66,
                b: 0x99,
                a: 255
            })
        );
        assert_eq!(
            config.fresh_color(),
            Some(ThemeColor {
                r: 0xff,
                g: 0x8d,
                b: 0x28,
                a: 255
            })
        );
        let alpha = ThemeConfig {
            fresh_color: "#ff8d2880".to_owned(),
            ..ThemeConfig::default()
        };
        assert_eq!(alpha.fresh_color().map(|color| color.a), Some(0x80));
    }

    #[test]
    fn garbage_colors_fall_back_to_the_builtin() {
        let config = ThemeConfig {
            gloss_color: "nope".to_owned(),
            fresh_color: "#12345".to_owned(),
            ..ThemeConfig::default()
        };
        assert!(config.gloss_color().is_none());
        assert!(config.fresh_color().is_none());
    }

    #[test]
    fn gloss_size_is_clamped() {
        let small = ThemeConfig {
            gloss_size: 1.0,
            ..ThemeConfig::default()
        };
        let large = ThemeConfig {
            gloss_size: 400.0,
            ..ThemeConfig::default()
        };
        assert_eq!(small.gloss_size(), MIN_GLOSS_SIZE);
        assert_eq!(large.gloss_size(), MAX_GLOSS_SIZE);
        assert!(small.any_changed() && large.any_changed());
    }

    #[test]
    fn section_parses_and_is_part_of_defaults() {
        use crate::Config;

        let config: Config = toml::from_str(
            "[theme]\ngloss_size = 18.0\ngloss_color = \"#112233\"\nfresh_color = \"#ff0000\"\n",
        )
        .unwrap();
        assert_eq!(config.theme.gloss_size(), 18.0);
        assert_eq!(
            config.theme.gloss_color(),
            Some(ThemeColor {
                r: 0x11,
                g: 0x22,
                b: 0x33,
                a: 255
            })
        );
        assert_eq!(config.theme.fresh_color().map(|color| color.r), Some(0xff));

        // 不写 `[theme]` 时全是缺省值，与 Config::default() 一致
        let empty: Config = toml::from_str("").unwrap();
        assert_eq!(empty.theme, ThemeConfig::default());
    }
}
