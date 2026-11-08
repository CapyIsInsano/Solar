//! ☀ Solar — gestor de archivos
//!
//! Núcleo en C (nucleo/), interfaz en Rust (src/) y configuración en Lua
//! (~/.config/solar/config.lua y ~/.config/solar/temas/*.lua).
//!
//! Licencia: GPL-3.0-or-later

mod ajustes;
mod app;
mod atajos;
mod barras;
mod dialogos;
mod ffi;
mod formato;
mod fuentes;
mod iconos;
mod nucleo;
mod tareas;
mod tema;
mod ui_util;
mod vista;

use eframe::egui;
use std::path::PathBuf;

const AYUDA: &str = "☀ Solar — gestor de archivos

Uso:
  solar [CARPETA]       Abre Solar (en CARPETA o en tu carpeta personal)

Opciones:
  -h, --help            Muestra esta ayuda
  -v, --version         Muestra la versión
      --config          Dice dónde están los archivos de configuración

Configuración:
  ~/.config/solar/config.lua      Ajustes (Solar la reescribe al cambiar algo)
  ~/.config/solar/temas/*.lua     Tus temas de colores
  ~/.local/share/solar/           Colores y logotipos de tus carpetas
";

fn main() -> eframe::Result {
    let mut abrir: Option<PathBuf> = None;
    for arg in std::env::args_os().skip(1) {
        match arg.to_str() {
            Some("-h" | "--help") => {
                print!("{AYUDA}");
                return Ok(());
            }
            Some("-v" | "--version") => {
                println!("Solar {} (núcleo {})", env!("CARGO_PKG_VERSION"), nucleo::version());
                return Ok(());
            }
            Some("--config") => {
                println!("config: {}", nucleo::dir_config().join("config.lua").display());
                println!("temas:  {}", nucleo::dir_config().join("temas").display());
                println!("datos:  {}", nucleo::dir_datos().display());
                return Ok(());
            }
            Some(s) if s.starts_with('-') => {
                eprintln!("solar: no conozco la opción «{s}» (prueba --help)");
                std::process::exit(2);
            }
            _ => {
                let ruta = PathBuf::from(&arg);
                let ruta = if ruta.is_absolute() {
                    ruta
                } else {
                    std::env::current_dir().map(|d| d.join(&ruta)).unwrap_or(ruta)
                };
                let ruta = std::fs::canonicalize(&ruta).unwrap_or(ruta);
                if ruta.is_dir() {
                    abrir = Some(ruta);
                } else if let Some(padre) = ruta.parent().filter(|p| p.is_dir()) {
                    abrir = Some(padre.to_path_buf());
                } else {
                    eprintln!("solar: «{}» no existe; abro tu carpeta personal.", ruta.display());
                }
            }
        }
    }

    // Configuración (la lee el núcleo en C con Lua)
    let dir_config = nucleo::dir_config();
    let dir_temas = dir_config.join("temas");
    let _ = std::fs::create_dir_all(&dir_temas);
    let archivo_config = dir_config.join("config.lua");
    let carga = nucleo::Config::cargar(&archivo_config);
    let mut avisos: Vec<String> = carga.avisos.iter().cloned().collect();
    if !carga.existia {
        // Primera vez: dejamos un config.lua bien comentado para editar a mano
        if let Err(e) = carga.config.guardar(&archivo_config) {
            avisos.push(format!("No se pudo crear {}: {e}", archivo_config.display()));
        }
    }
    let config = carga.config;
    let (temas, avisos_temas) = nucleo::cargar_temas(Some(&dir_temas));
    avisos.extend(avisos_temas);
    if !temas.iter().any(|t| t.id == config.tema) {
        avisos.push(format!("No existe el tema «{}»; uso «solar».", config.tema));
    }
    let carpetas = match nucleo::Carpetas::abrir(&nucleo::dir_datos().join("carpetas.tsv")) {
        Some(c) => c,
        None => {
            eprintln!("solar: no se pudo abrir la base de carpetas personalizadas");
            std::process::exit(1);
        }
    };

    let integrada = config.barra_titulo_integrada;
    let mut ventana = egui::ViewportBuilder::default()
        .with_title("Solar")
        .with_app_id("solar")
        .with_inner_size([1180.0, 740.0])
        .with_min_inner_size([560.0, 380.0])
        .with_transparent(true)
        .with_decorations(!integrada);
    if let Ok(icono) = eframe::icon_data::from_png_bytes(include_bytes!("../recursos/solar-256.png")) {
        ventana = ventana.with_icon(icono);
    }
    let opciones = eframe::NativeOptions { viewport: ventana, ..Default::default() };
    let inicio = app::Inicio {
        rutas: app::Rutas { config: archivo_config, temas: dir_temas },
        config,
        temas,
        carpetas,
        avisos,
        abrir,
    };
    eframe::run_native(
        "Solar",
        opciones,
        Box::new(move |cc| Ok(Box::new(app::Solar::nuevo(cc, inicio)))),
    )
}
