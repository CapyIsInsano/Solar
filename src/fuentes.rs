//! Fuentes: la integrada de egui, la del sistema (vía fontconfig) o un
//! archivo .ttf/.otf, más los íconos Phosphor (normales y rellenos).

use crate::iconos::FUENTE_RELLENA;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

pub fn configurar(ctx: &egui::Context, fuente: &str) -> Result<(), String> {
    let mut defs = FontDefinitions::default();
    let mut error = None;
    let archivo = match fuente.trim() {
        "" => None,
        "sistema" => match fuente_del_sistema() {
            Some(a) => Some(a),
            None => {
                error = Some("No se pudo encontrar la fuente del sistema (¿está fontconfig?). Uso la integrada.".to_string());
                None
            }
        },
        ruta => Some(crate::nucleo::expandir(ruta).to_string_lossy().into_owned()),
    };
    if let Some(a) = archivo {
        match std::fs::read(&a) {
            Ok(bytes) => {
                defs.font_data.insert("personal".into(), Arc::new(FontData::from_owned(bytes)));
                defs.families.entry(FontFamily::Proportional).or_default().insert(0, "personal".into());
            }
            Err(e) => error = Some(format!("No se pudo leer la fuente «{a}»: {e}. Uso la integrada.")),
        }
    }
    egui_phosphor::add_to_fonts(&mut defs, egui_phosphor::Variant::Regular);
    egui_phosphor::add_font_bytes_as_family(&mut defs, FUENTE_RELLENA, egui_phosphor::Variant::Fill.font_bytes());
    ctx.set_fonts(defs);
    error.map_or(Ok(()), Err)
}

fn fuente_del_sistema() -> Option<String> {
    let salida = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", "sans-serif:style=Regular"])
        .output()
        .ok()?;
    let ruta = String::from_utf8(salida.stdout).ok()?.trim().to_string();
    let valida = [".ttf", ".otf", ".TTF", ".OTF"].iter().any(|e| ruta.ends_with(e));
    (valida && std::path::Path::new(&ruta).is_file()).then_some(ruta)
}
