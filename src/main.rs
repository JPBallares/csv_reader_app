use csv::Reader;
use eframe::egui::{self, Color32, FontFamily, FontId};
use egui_extras::{Column, TableBuilder};
use rfd::FileDialog;
use std::error::Error;

fn read_csv_with_header(file_path: &str) -> Result<(Vec<String>, Vec<Vec<String>>), Box<dyn Error>> {
    let mut rdr = Reader::from_path(file_path)?;
    let header = rdr.headers()?.iter().map(|s| s.to_string()).collect();
    let mut records = Vec::new();
    for result in rdr.records() {
        let record = result?;
        records.push(record.iter().map(|s| s.to_string()).collect());
    }
    Ok((header, records))
}

#[derive(Default)]
struct MyApp {
    csv_header: Vec<String>,
    csv_data: Vec<Vec<String>>,
    current_page: usize,
    rows_per_page: usize,
    search_query: String,
    search_results: Option<Vec<Vec<String>>>,
    row_number_input: String,
    selected_row: Option<Vec<String>>,
}

impl MyApp {
    fn total_pages(&self) -> usize {
        if self.csv_data.is_empty() {
            1
        } else {
            (self.csv_data.len() + self.rows_per_page - 1) / self.rows_per_page
        }
    }

    fn perform_search(&self) -> Vec<Vec<String>> {
        let query = self.search_query.to_lowercase();
        self.csv_data
            .iter()
            .filter(|row| row.iter().any(|cell| cell.to_lowercase().contains(&query)))
            .cloned()
            .collect()
    }

    fn get_row_by_number(&self, row_num: usize) -> Option<Vec<String>> {
        if row_num == 1 {
            Some(self.csv_header.clone())
        } else if row_num > 1 && row_num - 2 < self.csv_data.len() {
            Some(self.csv_data[row_num - 2].clone())
        } else {
            None
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Load CSV file
            if ui.button("Load CSV").clicked() {
                if let Some(path) = FileDialog::new().add_filter("CSV", &["csv"]).pick_file() {
                    if let Some(path_str) = path.to_str() {
                        if let Ok((header, data)) = read_csv_with_header(path_str) {
                            self.csv_header = header;
                            self.csv_data = data;
                            self.current_page = 0;
                            self.search_query.clear();
                            self.search_results = None;
                            self.row_number_input.clear();
                            self.selected_row = None;
                        }
                    } else {
                        eprintln!("Selected file path is not valid UTF-8");
                    }
                }
            }
            ui.separator();

            // Search by text:
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);
                if ui.button("Search").clicked() {
                    self.search_results = Some(self.perform_search());
                    self.selected_row = None;
                }
                if ui.button("Clear Search").clicked() {
                    self.search_query.clear();
                    self.search_results = None;
                }
            });
            ui.separator();

            // Row lookup:
            ui.horizontal(|ui| {
                ui.label("Go to row:");
                ui.text_edit_singleline(&mut self.row_number_input);
                if ui.button("Go").clicked() {
                    if let Ok(row_num) = self.row_number_input.trim().parse::<usize>() {
                        if row_num == 1 {
                            self.selected_row = Some(self.csv_header.clone());
                        } else if let Some(row) = self.get_row_by_number(row_num) {
                            self.selected_row = Some(row);
                        } else {
                            self.selected_row = None;
                        }
                        self.search_results = None;
                    }
                }
            });
            ui.separator();

            let rows_to_display: Vec<Vec<String>> = if !self.csv_header.is_empty() {
                if let Some(ref selected) = self.selected_row {
                    if selected != &self.csv_header {
                        vec![self.csv_header.clone(), selected.clone()]
                    } else {
                        vec![self.csv_header.clone()]
                    }
                } else if let Some(ref results) = self.search_results {
                    let mut rows = vec![self.csv_header.clone()];
                    rows.extend(results.clone());
                    rows
                } else {
                    let start = self.current_page * self.rows_per_page;
                    let end = ((self.current_page + 1) * self.rows_per_page).min(self.csv_data.len());
                    let mut rows = vec![self.csv_header.clone()];
                    rows.extend(self.csv_data[start..end].iter().cloned());
                    rows
                }
            } else {
                vec![]
            };

            if !rows_to_display.is_empty() {
                egui::ScrollArea::both().show(ui, |ui| {
                    let ctx = ui.ctx().clone(); // clone the context early
                    let num_columns = self.csv_header.len();
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .columns(Column::initial(150.0), num_columns)
                        .header(25.0, |mut header| {
                            for header_cell in &self.csv_header {
                                header.col(|ui| {
                                    ui.label(egui::RichText::new(header_cell).text());
                                });
                            }
                        })
                        .body(|mut body| {
                            let rows = if rows_to_display.len() > 1 && rows_to_display[0] == self.csv_header {
                                &rows_to_display[1..]
                            } else {
                                &rows_to_display[..]
                            };
                            for row in rows {
                                // Assume each column starts with 150.0 width.
                                let row_height = row.iter().fold(20.0f32, |mut max_height, cell| {
                                    let available_width = 150.0;
                                    let galley = ctx.fonts(|f| {
                                        f.layout(
                                            cell.clone(),
                                            FontId::new(14.0, FontFamily::Proportional),
                                            Color32::WHITE,
                                            available_width,
                                        )
                                    });
                                    max_height = max_height.max(galley.size().y as f32);
                                    max_height
                                });
                                body.row(row_height, |mut row_ui| {
                                    for cell in row {
                                        row_ui.col(|ui| {
                                            ui.add(egui::Label::new(cell).wrap(true));
                                        });
                                    }
                                });
                            }
                        });
                });
            }

            if self.search_results.is_none() && self.selected_row.is_none() {
                ui.horizontal(|ui| {
                    if ui.button("Previous").clicked() && self.current_page > 0 {
                        self.current_page -= 1;
                    }
                    ui.label(format!("Page {} of {}", self.current_page + 1, self.total_pages()));
                    if ui.button("Next").clicked() && self.current_page + 1 < self.total_pages() {
                        self.current_page += 1;
                    }
                });
            }
        });
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "CSV Reader",
        options,
        Box::new(|_cc| Box::new(MyApp {
            rows_per_page: 100,
            ..Default::default()
        })),
    )?;
    Ok(())
}
