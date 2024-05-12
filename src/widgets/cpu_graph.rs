use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::{prelude::*, widgets::*};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub struct CpuGraph<'a> {
    block: Option<Block<'a>>,
    data: VecDeque<u64>,
}

impl<'a> Default for CpuGraph<'a> {
    fn default() -> CpuGraph<'a> {
        CpuGraph {
            block: None,
            data: VecDeque::from(vec![0_u64, 25]),
        }
    }
}

impl<'a> CpuGraph<'a> {
    /// Surrounds the `CpuGraph` with a [`Block`].
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn update(mut self, point: u64) -> Self {
        self.data.push_back(point);
        self.data.pop_front();
        self
    }
}

impl Widget for CpuGraph<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_ref(area, buf);
    }
}

impl WidgetRef for CpuGraph<'_> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        self.block.render_ref(area, buf);
        let inner = self.block.inner_if_some(area);
        self.render_cpu_graph(inner, buf);
    }
}

impl CpuGraph<'_> {
    fn render_cpu_graph(&self, gpu_graph_area: Rect, buf: &mut Buffer) {
        if gpu_graph_area.is_empty() {
            return;
        }
        let label = Span::raw(format!("{}", self.data.len()));
        buf.set_span(0, 0, &label, 1);
    }
}
