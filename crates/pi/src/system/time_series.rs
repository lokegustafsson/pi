use crate::show::Show;
use eframe::egui::{
    plot::{Corner, Legend, Line, Plot, PlotPoints},
    Ui,
};
use ingest::{Series, HISTORY, TICK_DELAY};
use std::ops::RangeInclusive;

pub struct TimeSeries<'a> {
    pub name: &'a str,
    pub max_y: f64,
    pub kind: TimeSeriesKind,
    pub value_kind: ValueKind,
}
#[derive(PartialEq)]
pub enum TimeSeriesKind {
    Preview,
    Primary,
    GridCell { width: f32 },
}
#[derive(PartialEq)]
pub enum ValueKind {
    Percent,
    Bytes,
    Temperature,
}
impl<'a> TimeSeries<'a> {
    pub fn render(&self, ui: &mut Ui, series: &[(&str, &Series<f64>)]) {
        let series_max_y = series
            .iter()
            .map(|(_, series)| series.iter().copied().max_by(f64::total_cmp).unwrap())
            .reduce(f64::max)
            .unwrap();
        Plot::new(self.name)
            .view_aspect(match self.kind {
                TimeSeriesKind::Preview | TimeSeriesKind::Primary => 1.6,
                TimeSeriesKind::GridCell { .. } => 1.0,
            })
            .with_prop(
                match self.kind {
                    TimeSeriesKind::GridCell { width } => Some(width),
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
            .include_y(self.max_y.min(1.2 * series_max_y))
            .sharp_grid_lines(false)
            .y_axis_formatter(match self.value_kind {
                ValueKind::Bytes => |val, range: &RangeInclusive<f64>| {
                    let maximum = *range.end();
                    Show::size_at_scale(val, maximum)
                },
                ValueKind::Percent => |val, _: &_| format!("{:.0}%", 100.0 * val),
                ValueKind::Temperature => |val, _: &_| format!("{val}°C"),
            })
            .with_prop(
                match self.kind {
                    TimeSeriesKind::Preview => None,
                    TimeSeriesKind::Primary | TimeSeriesKind::GridCell { .. } => Some(()),
                },
                |plot, ()| plot.legend(Legend::default().position(Corner::LeftTop)),
            )
            .show(ui, |ui| {
                for (name, series) in series {
                    let points: PlotPoints = series
                        .iter()
                        .enumerate()
                        .map(|(i, &y)| {
                            [
                                (i as f64 - (series.len() - 1) as f64) * TICK_DELAY.as_secs_f64(),
                                y,
                            ]
                        })
                        .collect();
                    ui.line(Line::new(points).name(name));
                }
            })
            .response;
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
