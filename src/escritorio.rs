//! Integración con el escritorio, para que la barra de tareas muestre el
//! logo de Solar.
//!
//! En X11 basta con el ícono de la ventana (_NET_WM_ICON), pero en Wayland
//! (Niri, Hyprland, GNOME, KDE…) la barra busca un lanzador `.desktop` cuyo
//! nombre coincida con el app_id de la ventana ("solar") y usa su `Icon=`.
//! Si Solar no está instalado con `make instalar` (por ejemplo, lo corres
//! desde `target/release`), aquí se deja el ícono y un lanzador en
//! ~/.local/share para tu usuario. Si después lo instalas en el sistema,
//! el lanzador generado se quita solo.

use crate::nucleo;
use std::fs;
use std::path::{Path, PathBuf};

pub const ICONOS: [(u32, &[u8]); 8] = [
    (16, include_bytes!("../recursos/iconos/solar-16.png")),
    (24, include_bytes!("../recursos/iconos/solar-24.png")),
    (32, include_bytes!("../recursos/iconos/solar-32.png")),
    (48, include_bytes!("../recursos/iconos/solar-48.png")),
    (64, include_bytes!("../recursos/iconos/solar-64.png")),
    (128, include_bytes!("../recursos/iconos/solar-128.png")),
    (256, include_bytes!("../recursos/iconos/solar-256.png")),
    (512, include_bytes!("../recursos/iconos/solar-512.png")),
];
pub const SVG: &[u8] = include_bytes!("../recursos/solar.svg");
const MARCA: &str = "X-Solar-Generado=true";

/// ~/.local/share
fn datos_usuario() -> PathBuf {
    nucleo::dir_datos().parent().map(Path::to_path_buf).unwrap_or_else(|| nucleo::dir_hogar().join(".local/share"))
}

/// Carpetas de datos del sistema ($XDG_DATA_DIRS)
fn datos_sistema() -> Vec<PathBuf> {
    let v = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
    let v = if v.trim().is_empty() { "/usr/local/share:/usr/share".to_string() } else { v };
    v.split(':').filter(|d| !d.is_empty()).map(PathBuf::from).collect()
}

fn en_sistema(relativa: &str) -> bool {
    datos_sistema().iter().any(|d| d.join(relativa).exists())
}

fn escribir_si_cambia(ruta: &Path, datos: &[u8]) -> std::io::Result<bool> {
    if fs::read(ruta).is_ok_and(|v| v == datos) {
        return Ok(false);
    }
    if let Some(dir) = ruta.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = ruta.with_extension("tmp-solar");
    fs::write(&tmp, datos)?;
    fs::rename(&tmp, ruta)?;
    Ok(true)
}

fn lanzador(exe: &Path) -> String {
    // Exec necesita comillas si la ruta tiene espacios
    let ruta = exe.to_string_lossy();
    let exec = if ruta.contains(' ') { format!("\"{}\"", ruta.replace('"', "\\\"")) } else { ruta.into_owned() };
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Solar\n\
         GenericName=Gestor de archivos\n\
         Comment=Explora y organiza tus archivos, con carpetas de colores y logotipos\n\
         Exec={exec} %U\n\
         Icon=solar\n\
         Terminal=false\n\
         StartupWMClass=solar\n\
         Categories=System;FileTools;FileManager;Utility;\n\
         MimeType=inode/directory;\n\
         Keywords=archivos;carpetas;explorador;gestor;files;folders;\n\
         {MARCA}\n"
    )
}

/// Deja el ícono y el lanzador (en un hilo aparte; no estorba al arrancar).
pub fn integrar() {
    std::thread::Builder::new()
        .name("solar-escritorio".into())
        .spawn(|| {
            if let Err(e) = integrar_ya() {
                eprintln!("solar: no pude dejar el ícono para la barra de tareas: {e}");
            }
        })
        .ok();
}

fn integrar_ya() -> std::io::Result<()> {
    let usuario = datos_usuario();
    let mut cambio = false;

    // Íconos (si el sistema no los tiene ya)
    if !en_sistema("icons/hicolor/scalable/apps/solar.svg") {
        cambio |= escribir_si_cambia(&usuario.join("icons/hicolor/scalable/apps/solar.svg"), SVG)?;
    }
    for (tam, png) in ICONOS {
        let rel = format!("icons/hicolor/{tam}x{tam}/apps/solar.png");
        if !en_sistema(&rel) {
            cambio |= escribir_si_cambia(&usuario.join(&rel), png)?;
        }
    }

    // Lanzador
    let propio = usuario.join("applications/solar.desktop");
    let generado = fs::read_to_string(&propio).ok().filter(|t| t.contains(MARCA));
    if en_sistema("applications/solar.desktop") {
        // Ya está instalado: el lanzador generado solo estorbaría
        if generado.is_some() {
            let _ = fs::remove_file(&propio);
            cambio = true;
        }
    } else if generado.is_some() || !propio.exists() {
        // (si tú hiciste tu propio solar.desktop, no lo tocamos)
        if let Ok(exe) = std::env::current_exe() {
            let exe = PathBuf::from(exe.to_string_lossy().trim_end_matches(" (deleted)").to_string());
            cambio |= escribir_si_cambia(&propio, lanzador(&exe).as_bytes())?;
        }
    }

    if cambio {
        // Avisar a los lanzadores que cachean (si la herramienta existe). A la
        // carpeta de íconos del usuario no le hacemos caché: una caché vieja
        // ahí escondería los íconos de otros programas.
        let _ = std::process::Command::new("update-desktop-database")
            .arg("-q")
            .arg(usuario.join("applications"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    Ok(())
}
