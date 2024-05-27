use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::Vec2b;
use egui_plot::{AxisHints, GridMark, Legend, Plot};
use tracing::{debug, error, info, trace, warn};

use dashmap::DashMap;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Instant,
};

use crate::{conn_manager::PrinterId, ui::ui_types::App};

#[derive(Debug, Clone)]
pub struct Graphs {
    pub printer_graphs: Arc<DashMap<PrinterId, PrinterGraphData>>,
}

/// new
impl Graphs {
    pub fn new() -> Self {
        Self {
            printer_graphs: Arc::new(DashMap::new()),
        }
    }

    pub fn debug_new(id0: PrinterId, id1: PrinterId) -> Self {
        let mut map: DashMap<PrinterId, PrinterGraphData> = DashMap::new();

        let t0 = chrono::Local::now();

        let mut entry0 = map.entry(id0.clone()).or_default();
        let mut entry1 = map.entry(id1.clone()).or_default();

        for i in 0..20 {
            let t = t0 - chrono::Duration::seconds(i * 2);

            entry0
                .temp_nozzle
                .vals
                .push_back((t, 180.0 + i as f64 * 5.));
            entry1
                .temp_nozzle
                .vals
                .push_back((t, 100.0 + i as f64 * 5.));
        }

        drop(entry0);
        drop(entry1);

        Self {
            printer_graphs: Arc::new(map),
        }
    }

    #[cfg(feature = "nope")]
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let data = (*self.printer_graphs).clone();
        let data = serde_json::to_string_pretty(&data)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    #[cfg(feature = "nope")]
    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let data: HashMap<PrinterId, PrinterGraphData> = serde_json::from_str(&data)?;
        let printer_graphs = Arc::new(DashMap::from(data));
        Ok(Self { printer_graphs })
    }

    pub fn update_printer(&self, id: &PrinterId, data: &crate::mqtt::message::PrintData) {
        let t = chrono::Local::now();
        let mut entry = self.printer_graphs.entry(id.clone()).or_default();

        if let Some(temp) = data.nozzle_temper {
            entry.temp_nozzle.vals.push_back((t, temp));
        }
        if let Some(temp) = data.nozzle_target_temper {
            entry.temp_nozzle_tgt.vals.push_back((t, temp));
        }
        if let Some(temp) = data.bed_temper {
            entry.temp_bed.vals.push_back((t, temp));
        }
        if let Some(temp) = data.bed_target_temper {
            entry.temp_bed_tgt.vals.push_back((t, temp));
        }
        if let Some(temp) = data.chamber_temper {
            entry.temp_chamber.vals.push_back((t, temp));
        }

        // self.printer_graphs.insert(id.clone(), data);
    }
}

#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct PrinterGraphData {
    temp_nozzle: GraphValues<f64>,
    temp_nozzle_tgt: GraphValues<f64>,
    temp_bed: GraphValues<f64>,
    temp_bed_tgt: GraphValues<f64>,
    temp_chamber: GraphValues<f64>,
    temp_chamber_tgt: GraphValues<f64>,
}

#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct GraphValues<T> {
    vals: VecDeque<(chrono::DateTime<chrono::Local>, T)>,
}

impl App {
    pub fn show_graphs(&mut self, ui: &mut egui::Ui) {
        ui.label("TODO: Graphs");

        const MINS_PER_DAY: f64 = 24.0 * 60.0;
        const MINS_PER_H: f64 = 60.0;

        fn day(x: f64) -> f64 {
            (x / MINS_PER_DAY).floor()
        }

        fn hour(x: f64) -> f64 {
            (x.rem_euclid(MINS_PER_DAY) / MINS_PER_H).floor()
        }

        fn minute(x: f64) -> f64 {
            x.rem_euclid(MINS_PER_H).floor()
        }

        fn is_approx_integer(val: f64) -> bool {
            val.fract().abs() < 1e-6
        }

        let time_formatter = |mark: GridMark, _digits, _range: &std::ops::RangeInclusive<f64>| {
            let minutes = mark.value;
            if minutes < 0.0 || 5.0 * MINS_PER_DAY <= minutes {
                // No labels outside value bounds
                String::new()
            } else if is_approx_integer(minutes / MINS_PER_DAY) {
                // Days
                format!("Day {}", day(minutes))
            } else {
                // Hours and minutes
                format!("{h}:{m:02}", h = hour(minutes), m = minute(minutes))
            }
        };

        let x_axes = vec![
            AxisHints::new_x().label("Time").formatter(time_formatter),
            AxisHints::new_x().label("Value"),
        ];

        let plot = Plot::new("Printer Plot")
            .auto_bounds(Vec2b::new(false, false))
            .view_aspect(16. / 9.)
            .width(600.)
            .legend(Legend::default())
            .custom_x_axes(x_axes)
            // .legend(Legend::default())
            ;

        let inner = plot.show(ui, |plot_ui| {
            for id in self.config.printer_ids() {
                let data = self
                    .graphs
                    .as_ref()
                    .unwrap()
                    .printer_graphs
                    .get(&id)
                    .unwrap();

                // let graph = data.temp_nozzle.vals.iter().map(|(t, v)| [*t, *v]);
                // plot_ui.line(Line::new(PlotPoints::from(graph)).name("curve"));
            }
        });
    }
}
