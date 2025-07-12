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

fn save_csv(
    path: &str,
    header: &Vec<String>,
    data: &Vec<Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(header)?;
    for row in data {
        wtr.write_record(row)?;
    }
    wtr.flush()?;
    Ok(())
}

#[derive(Default)]
struct MyApp {
    csv_header: Vec<String>,
    csv_data: Vec<Vec<String>>,
    current_page: usize,
    rows_per_page: usize,
    search_query: String,
    search_header: u8,
    search_results: Option<Vec<Vec<String>>>,
    row_number_input: String,
    selected_row: Option<Vec<String>>,
    visible_columns: Vec<bool>, // Track which columns are visible
    show_column_controls: bool, // Toggle for showing/hiding column controls
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
            .filter(|row| {
                row.iter()
                    .enumerate()
                    .any(|(idx, cell)| idx as u8 == self.search_header && cell.to_lowercase().contains(&query))
            })
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

    // Filter columns based on visibility
    fn filter_visible_columns(&self, row: &Vec<String>) -> Vec<String> {
        row.iter()
            .enumerate()
            .filter(|(idx, _)| *idx < self.visible_columns.len() && self.visible_columns[*idx])
            .map(|(_, cell)| cell.clone())
            .collect()
    }

    // Get visible headers
    fn get_visible_headers(&self) -> Vec<String> {
        self.filter_visible_columns(&self.csv_header)
    }

    // Initialize visible columns when CSV is loaded
    fn initialize_visible_columns(&mut self) {
        self.visible_columns = vec![true; self.csv_header.len()];
    }

    // Toggle all columns on/off
    fn toggle_all_columns(&mut self, visible: bool) {
        self.visible_columns = vec![visible; self.csv_header.len()];
    }

    // Count visible columns
    fn visible_column_count(&self) -> usize {
        self.visible_columns.iter().filter(|&&v| v).count()
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
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
                                self.initialize_visible_columns();
                            }
                        } else {
                            eprintln!("Selected file path is not valid UTF-8");
                        }
                    }
                }
                // Save CSV file
                if ui.button("Save CSV").clicked() {
                    if let Some(path) = FileDialog::new().save_file() {
                        if let Some(path_str) = path.to_str() {
                            if let Err(err) = save_csv(path_str, &self.csv_header, &self.csv_data) {
                                eprintln!("Error saving CSV: {}", err);
                            }
                        }
                    }
                }

                // Column visibility controls
                if !self.csv_header.is_empty() {
                    ui.separator();
                    if ui.button(if self.show_column_controls { "Hide Column Controls" } else { "Show Column Controls" }).clicked() {
                        self.show_column_controls = !self.show_column_controls;
                    }

                    ui.label(format!("Visible: {}/{}", self.visible_column_count(), self.csv_header.len()));
                }
            });

            // Column visibility controls
            if self.show_column_controls && !self.csv_header.is_empty() {
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Show All").clicked() {
                        self.toggle_all_columns(true);
                    }
                    if ui.button("Hide All").clicked() {
                        self.toggle_all_columns(false);
                    }
                });

                ui.label("Column Visibility:");
                ui.push_id("column_visibility_scroll", |ui| {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (idx, header) in self.csv_header.iter().enumerate() {
                                if idx < self.visible_columns.len() {
                                    ui.push_id(idx, |ui| {
                                        ui.checkbox(&mut self.visible_columns[idx], header)
                                            .on_hover_text(format!("Toggle visibility for column: {}", header));
                                    });
                                }
                            }
                        });
                    });
                });
            }

            ui.separator();

            // Search by text:
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);

                ui.label("Column Index:");
                let mut search_header = self.search_header.to_string();
                ui.text_edit_singleline(&mut search_header);
                self.search_header = match search_header.parse() { Ok(n) => n, Err(_) => 0};

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

            if self.search_results.is_none() && self.selected_row.is_none() {
                ui.separator();
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

            if !rows_to_display.is_empty() && self.visible_column_count() > 0 {
                egui::ScrollArea::both().show(ui, |ui| {
                    let ctx = ui.ctx().clone();
                    let visible_headers = self.get_visible_headers();
                    let num_visible_columns = visible_headers.len();

                    if num_visible_columns > 0 {
                        TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::TOP))
                            .columns(Column::initial(150.0), num_visible_columns)
                            .header(25.0, |mut header| {
                                for header_cell in &visible_headers {
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
                                    let visible_row = self.filter_visible_columns(row);
                                    let row_height = visible_row.iter().fold(20.0f32, |mut max_height, cell| {
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
                                        for cell in &visible_row {
                                            row_ui.col(|ui| {
                                                ui.add(egui::Label::new(cell).wrap(true));
                                            });
                                        }
                                    });
                                }
                            });
                    }
                });
            } else if !rows_to_display.is_empty() && self.visible_column_count() == 0 {
                ui.label("No columns are visible. Use the column controls to show columns.");
            }
        });
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut options = eframe::NativeOptions::default();
    options.maximized = true;
    eframe::run_native(
        "CSV Reader",
        options,
        Box::new(|_cc| Box::new(MyApp {
            rows_per_page: 100,
            show_column_controls: false,
            ..Default::default()
        })),
    )?;
    Ok(())
}