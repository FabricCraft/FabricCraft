#![cfg_attr(all(target_os = "windows", not(test)), windows_subsystem = "windows")]

#[path = "studio/app.rs"]
mod studio;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([800.0, 580.0]),
        ..Default::default()
    };
    eframe::run_native(
        "FabricCraft",
        options,
        Box::new(|context| Ok(Box::new(studio::Studio::new(&context.egui_ctx)))),
    )
}
