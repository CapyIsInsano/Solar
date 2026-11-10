//! ☀ Solar — gestor de archivos
//!
//! Núcleo en C (nucleo/), interfaz en Rust (src/) y configuración en Lua
//! (~/.config/solar/config.lua y ~/.config/solar/temas/*.lua).
//!
//! Por ahora Solar es exclusivo de Linux.
//!
//! Licencia: GPL-3.0-or-later

#[cfg(not(target_os = "linux"))]
compile_error!("Solar por ahora es exclusivo de Linux (el núcleo usa inotify, /proc y /sys).");

mod actualizar;
mod ajustes;
mod app;
mod arrastre_os;
mod atajos;
mod barras;
mod dialogos;
mod escritorio;
mod ffi;
mod formato;
mod fuentes;
mod iconos;
mod iconos_programas;
mod memoria;
mod nucleo;
mod perfil;
mod sonido;
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
  ~/.cache/solar/                 Descargas de actualizaciones (se puede borrar)

Solar por ahora es exclusivo de Linux.
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
                println!("Solar {} (núcleo {})", actualizar::version_completa(), nucleo::version());
                return Ok(());
            }
            Some("--config") => {
                println!("config: {}", nucleo::dir_config().join("config.lua").display());
                println!("temas:  {}", nucleo::dir_config().join("temas").display());
                println!("datos:  {}", nucleo::dir_datos().display());
                println!("caché:  {}", actualizar::dir_cache().display());
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

    memoria::ajustar();
    let arranque = std::time::Instant::now();

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
        avisos.push(format!("No existe el tema «{}»; uso «sol».", config.tema));
    }
    let carpetas = match nucleo::Carpetas::abrir(&nucleo::dir_datos().join("carpetas.tsv")) {
        Some(c) => c,
        None => {
            eprintln!("solar: no se pudo abrir la base de carpetas personalizadas");
            std::process::exit(1);
        }
    };

    perfil::reportar("config, temas y carpetas", arranque);
    let integrada = config.barra_titulo_integrada;
    let mut ventana = egui::ViewportBuilder::default()
        .with_title("Solar")
        .with_app_id("solar")
        .with_inner_size([1180.0, 740.0])
        .with_min_inner_size([560.0, 380.0])
        .with_transparent(true)
        .with_decorations(!integrada);
    if let Ok(icono) = eframe::icon_data::from_png_bytes(escritorio::ICONOS[6].1) {
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
        Box::new(move |cc| {
            perfil::reportar("ventana y OpenGL listos", arranque);
            let t = std::time::Instant::now();
            let app = app::Solar::nuevo(cc, inicio);
            perfil::reportar("Solar::nuevo", t);
            Ok(Box::new(app))
        }),
    )
}
