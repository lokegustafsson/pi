use crate::show::Show;
use eframe::egui::{TextStyle, Ui};
use egui_plot::{Corner, Legend, Line, Plot, PlotPoints};
use std::ops::RangeInclusive;
use sysinfo::Series;
use util::{HISTORY, TICK_DELAY};

pub struct TimeSeries<'a> {
    pub name: &'a str,
    pub max_y: Option<f64>,
    pub kind: TimeSeriesKind,
    pub value_kind: ValueKind,
}
#[derive(Clone, Copy, PartialEq)]
pub enum TimeSeriesKind {
    Preview,
    Primary,
    GridCell { width: f32 },
}
#[derive(Clone, Copy, PartialEq)]
pub enum ValueKind {
    Percent,
    Bytes,
    Temperature,
}
impl<'a> TimeSeries<'a> {
    pub fn render(&self, ui: &mut Ui, series: &[(&str, &Series<f64>)]) {
        let series_max_y = series
            .iter()
            .map(|(_, series)| series.iter().max_by(f64::total_cmp).unwrap())
            .reduce(f64::max)
            .unwrap();
        let plot_width_pixels = ui.ctx().pixels_per_point() * ui.available_width();
        Plot::new(self.name)
            .with_prop(
                match self.kind {
                    TimeSeriesKind::Preview => None,
                    TimeSeriesKind::Primary => Some(1.6),
                    TimeSeriesKind::GridCell { .. } => Some(1.0),
                },
                |plot, aspect| plot.view_aspect(aspect),
            )
            .with_prop(
                match self.kind {
                    TimeSeriesKind::Primary => {
                        let h = ui.ctx().available_rect().height()
                            - 10.0 * ui.text_style_height(&TextStyle::Body);
                        Some(f32::min(h, ui.available_width() / 1.6))
                    }
                    TimeSeriesKind::Preview => Some(ui.available_height()),
                    _ => None,
                },
                |plot, height| plot.height(height),
            )
            .with_prop(
                match self.kind {
                    TimeSeriesKind::GridCell { width } => Some(width),
                    TimeSeriesKind::Preview => Some(ui.available_width()),
                    _ => None,
                },
                |plot, width| plot.width(width),
            )
            .show_x(false)
            .show_y(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_drag(false)
            .include_x(-((HISTORY - 1) as f64) * TICK_DELAY.as_secs_f64())
            .include_x(0)
            .include_y(0)
            .include_y(self.max_y.unwrap_or(1.2 * series_max_y))
            .sharp_grid_lines(false)
            .y_axis_formatter(match self.value_kind {
                ValueKind::Bytes => |val, _: _, range: &RangeInclusive<f64>| {
                    let maximum = *range.end();
                    Show::size_at_scale(val, maximum)
                },
                ValueKind::Percent => |val, _: _, _: &_| format!("{:.0}%", 100.0 * val),
                ValueKind::Temperature => |val, _: _, _: &_| format!("{val}°C"),
            })
            .with_prop(
                match self.kind {
                    TimeSeriesKind::Preview => Some(()),
                    _ => None,
                },
                |plot, ()| plot.custom_y_axes(Vec::new()),
            )
            .with_prop(
                match self.kind {
                    TimeSeriesKind::Preview => None,
                    TimeSeriesKind::Primary | TimeSeriesKind::GridCell { .. } => Some(()),
                },
                |plot, ()| plot.legend(Legend::default().position(Corner::LeftTop)),
            )
            .show(ui, |ui| {
                if let Some(max_y) = self.max_y {
                    ui.line(
                        Line::new(PlotPoints::from_iter([[-60.0, max_y], [0.0f64, max_y]]))
                            .name("Max"),
                    );
                }
                for (name, series) in series {
                    let chunk_size =
                        (Series::<f64>::capacity() as f32 / plot_width_pixels) as usize;
                    let (first, middle, last) = series.chunks(chunk_size);
                    let mut points = Vec::new();
                    let max = |slice: &'_ [f64]| {
                        slice.iter().copied().max_by(f64::total_cmp).unwrap_or(0.0)
                    };
                    if !first.is_empty() {
                        points.push([-(HISTORY as f64 * TICK_DELAY.as_secs_f64()), max(first)]);
                    }
                    points.extend(middle.enumerate().map(|(i, m)| {
                        [
                            -((HISTORY - first.len() - chunk_size * (i + 1)) as f64)
                                * (TICK_DELAY.as_secs_f64() * HISTORY as f64
                                    / (HISTORY - chunk_size) as f64),
                            max(m),
                        ]
                    }));
                    if !last.is_empty() {
                        points.push([0.0, max(last)]);
                    }
                    ui.line(Line::new(points).name(name));
                }
            });
    }
}

trait BuilderOptional: Sized {
    fn with_prop<T>(self, prop: Option<T>, f: impl Fn(Self, T) -> Self) -> Self {
        match prop {
            Some(t) => f(self, t),
            None => self,
        }
    }
}
impl<T> BuilderOptional for T {}
