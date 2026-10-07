//! 候选窗口的输出端。

use qingjian_platform::protocol::{Frame, ScreenRect};
use qingjian_platform::{CandidateRenderer, ThemeConfig};

/// 候选窗口 / 状态条的画法。
// 没有 `Eq`：[`ThemeConfig`] 里 `gloss_size` 是浮点，浮点不是 `Eq`；比较只用 `PartialEq`。
#[derive(Debug, Clone, PartialEq)]
pub struct RenderSettings {
    /// 由谁画（`[general] renderer`）。
    pub renderer: CandidateRenderer,

    /// 字族名（`[general] font`），空为系统字体。
    pub font: String,

    /// 译文小字的字号与颜色（`[theme]`）。只对候选窗口生效，状态条不受影响。
    pub theme: ThemeConfig,
}

/// Router 只产出帧，画交给它；Windows 上由 UI 线程实现。
pub trait CandidateSink: Send {
    /// 把候选窗口摆到 `rect`（组句范围的屏幕矩形）下方并按 `frame` 重绘。
    fn show(&self, frame: Frame, rect: ScreenRect);

    fn hide(&self);

    /// 换画法：装上时与配置热加载后调，只在设置变了时调。
    fn configure(&self, settings: RenderSettings);
}

/// 不画候选窗口的空实现。
pub struct NoopSink;

impl CandidateSink for NoopSink {
    fn show(&self, _frame: Frame, _rect: ScreenRect) {}

    fn hide(&self) {}

    fn configure(&self, _settings: RenderSettings) {}
}
