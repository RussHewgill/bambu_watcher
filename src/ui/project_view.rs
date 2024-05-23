use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::{Color32, Sense};
use egui_extras::Column;
use tracing::{debug, error, info, trace, warn};

use crate::cloud::projects::ProjectData;

use super::ui_types::App;

impl App {
    pub fn show_project_view(&mut self, ctx: &egui::Context) {
        egui::panel::SidePanel::left("printer_list")
            .min_width(200.)
            // .max_width(printer_list_size)
            .resizable(true)
            .show(ctx, |ui| {
                self.project_list(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Sync Projects").clicked() {
                self.cmd_tx
                    .as_ref()
                    .unwrap()
                    .send(crate::conn_manager::PrinterConnCmd::SyncProjects)
                    .unwrap();
            }
            //
        });

        //
    }

    fn project_list(&mut self, ui: &mut egui::Ui) {
        let row_height = 80.0;
        let thumbnail_size = 80.0;

        /// filter
        ui.horizontal(|ui| {
            //
        });

        // egui_data_table::DataTable<MyRowData>
        ui.add(
            egui_data_table::Renderer::new(&mut self.projects.projects, &mut { ProjectRowViewer })
                .with_table_row_height(row_height),
        );

        //
    }

    #[cfg(feature = "nope")]
    fn project_list(&mut self, ui: &mut egui::Ui) {
        let row_height = 80.0;
        let thumbnail_size = 80.0;

        /// filter
        ui.horizontal(|ui| {
            //
        });

        let mut table = egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(thumbnail_size))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            // .column(Column::initial(100.0).range(40.0..=300.0))
            // .column(Column::initial(100.0).at_least(40.0).clip(true))
            // .column(Column::remainder())
            // .min_scrolled_height(0.0)
            // .max_scroll_height(available_height)
            .sense(Sense::click());

        /// Columns:
        /// Thumbnail
        /// // Printer
        /// Name
        /// Date
        /// Status ?
        /// Time
        /// Material
        /// Plate
        table
            .header(40., |mut header| {
                header.col(|ui| {
                    ui.strong("Thumbnail");
                });
                // header.col(|ui| {
                //     ui.strong("Printer");
                // });
                header.col(|ui| {
                    ui.strong("Name");
                    //  debug!("sort by name");
                });

                header.col(|ui| {
                    let but = match self.projects.sorted() {
                        Some((super::ui_types::SortType::Date, false)) => {
                            egui::Button::image_and_text(super::icon_sort_down(), "Date")
                        }
                        Some((super::ui_types::SortType::Date, true)) => {
                            egui::Button::image_and_text(super::icon_sort_up(), "Date")
                        }
                        _ => egui::Button::new("Date"),
                    };
                    if ui.add(but).clicked() {
                        debug!("sort by date");
                        self.projects.sort_date();
                    }
                });

                #[cfg(feature = "nope")]
                if header
                    .col(|ui| {
                        ui.strong("Date");
                        match self.projects.sorted() {
                            Some((super::ui_types::SortType::Date, false)) => {
                                ui.add(super::icon_sort_down());
                            }
                            Some((super::ui_types::SortType::Date, true)) => {
                                ui.add(super::icon_sort_up());
                            }
                            _ => {}
                        }
                    })
                    .1
                    .clicked()
                {
                    debug!("sort by date");
                    self.projects.sort_date();
                };
                header.col(|ui| {
                    ui.strong("Status");
                });
                header.col(|ui| {
                    ui.strong("Time");
                });
                header.col(|ui| {
                    ui.strong("Material");
                });
                header.col(|ui| {
                    ui.strong("Plate");
                });
                header.col(|ui| {
                    ui.strong("Time");
                });
            })
            .body(|mut body| {
                body.rows(row_height, self.projects.projects.len(), |mut row| {
                    let row_index = row.index();
                    let p = &self.projects.projects[row_index];

                    let Some(plate) = p.plates.iter().find(|p| p.filaments.len() > 0) else {
                        return;
                    };

                    row.col(|ui| {
                        ui.add(
                            egui::Image::new(&plate.thumbnail.url)
                                .bg_fill(ui.visuals().panel_fill)
                                .rounding(5.),
                        );
                    });
                    row.col(|ui| {
                        ui.label(&p.name);
                    });
                    row.col(|ui| {
                        ui.label(&format!("{}", p.create_time.format("%Y-%m-%d %H:%M:%S")));
                    });
                    row.col(|ui| {
                        // ui.label(&p.status);
                    });
                    row.col(|ui| {
                        // ui.label(&p.status);
                    });
                    row.col(|ui| {
                        ui.label(&format!("{:.1}", plate.weight));
                    });
                    row.col(|ui| {
                        // ui.label(&plate.name);
                    });

                    //
                });
            });
    }
}

struct ProjectRowViewer;

impl ProjectRowViewer {
    /// Title, sortable
    const ROWS: [(&'static str, bool); 7] = [
        ("Thumbnail", false),
        ("Name", true),
        ("Date", true),
        ("Status", true),
        ("Time", true),
        ("Material", true),
        ("Plate", true),
    ];
}

// #[cfg(feature = "nope")]
impl egui_data_table::RowViewer<ProjectData> for ProjectRowViewer {
    fn num_columns(&mut self) -> usize {
        Self::ROWS.len()
    }

        #[rustfmt::skip]
    fn column_name(&mut self, column: usize) -> std::borrow::Cow<'static, str> {
            // [
            //     "Preview",
            //     "Name",
            //     "Status",
            //     "Time",
            //     "Material",
            // ][column]
            //     .into()
            Self::ROWS[column].0.into()
        }

    fn is_sortable_column(&mut self, column: usize) -> bool {
        Self::ROWS[column].1
    }

    fn column_render_config(&mut self, column: usize) -> Column {
        match column {
            0 => Column::exact(80.),
            1 => Column::auto(),
            2 => Column::auto(),
            3 => Column::auto(),
            4 => Column::auto(),
            5 => Column::auto(),
            6 => Column::auto(),
            _ => unreachable!(),
        }
    }

    fn show_cell_view(&mut self, ui: &mut egui::Ui, row: &ProjectData, column: usize) {
        let Some(plate) = row.plates.iter().find(|p| p.filaments.len() > 0) else {
            return;
        };

        match column {
            0 => {
                ui.add(
                    egui::Image::new(&plate.thumbnail.url)
                        .bg_fill(ui.visuals().panel_fill)
                        .rounding(5.),
                );
            }
            1 => {}
            2 => {}
            3 => {}
            4 => {}
            5 => {}
            6 => {}
            _ => unreachable!(),
        }
    }

    fn show_cell_editor(
        &mut self,
        ui: &mut egui::Ui,
        row: &mut ProjectData,
        column: usize,
    ) -> Option<egui::Response> {
        unimplemented!()
    }

    fn set_cell_value(&mut self, src: &ProjectData, dst: &mut ProjectData, column: usize) {
        unimplemented!()
    }

    fn new_empty_row(&mut self) -> ProjectData {
        ProjectData::default()
    }
}
