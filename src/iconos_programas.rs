//! Íconos de los programas instalados (para «Abrir con…»).
//!
//! Busca el `Icon=` del lanzador en tu tema de íconos (el de GTK), en
//! hicolor y en /usr/share/pixmaps. Lee PNG y SVG (con resvg). Si no lo
//! encuentra, la interfaz dibuja un glifo según el tipo de programa.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const TAM: u32 = 64;

fn dirs_datos() -> Vec<PathBuf> {
    let mut v = vec![crate::nucleo::dir_datos().parent().map(Path::to_path_buf).unwrap_or_default()];
    let sis = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
    let sis = if sis.trim().is_empty() { "/usr/local/share:/usr/share".to_string() } else { sis };
    v.extend(sis.split(':').filter(|d| !d.is_empty()).map(PathBuf::from));
    v.push(crate::nucleo::dir_hogar().join(".local/share/flatpak/exports/share"));
    v.push(PathBuf::from("/var/lib/flatpak/exports/share"));
    v
}

/// Temas de íconos a revisar, en orden: el tuyo, sus "padres" comunes y hicolor.
fn temas() -> &'static Vec<String> {
    static T: OnceLock<Vec<String>> = OnceLock::new();
    T.get_or_init(|| {
        let mut v = Vec::new();
        let hogar = crate::nucleo::dir_hogar();
        for archivo in [".config/gtk-4.0/settings.ini", ".config/gtk-3.0/settings.ini"] {
            if let Ok(t) = std::fs::read_to_string(hogar.join(archivo)) {
                for l in t.lines() {
                    if let Some(valor) = l.trim().strip_prefix("gtk-icon-theme-name") {
                        let nombre = valor.trim_start_matches([' ', '=']).trim().trim_matches('"').to_string();
                        if !nombre.is_empty() && !v.contains(&nombre) {
                            v.push(nombre);
                        }
                    }
                }
            }
        }
        for t in ["Papirus", "Adwaita", "breeze", "hicolor"] {
            if !v.iter().any(|x| x == t) {
                v.push(t.to_string());
            }
        }
        v
    })
}

fn buscar(nombre: &str) -> Option<PathBuf> {
    if nombre.is_empty() {
        return None;
    }
    let p = Path::new(nombre);
    if p.is_absolute() {
        return p.is_file().then(|| p.to_path_buf());
    }
    let tamanos = ["64x64", "48x48", "128x128", "96x96", "256x256", "32x32", "scalable", "512x512"];
    for tema in temas() {
        for base in dirs_datos().iter().map(|d| d.join("icons").join(tema)).chain(std::iter::once(crate::nucleo::dir_hogar().join(".icons").join(tema))) {
            if !base.is_dir() {
                continue;
            }
            for t in tamanos {
                for sub in ["apps", "applications"] {
                    for ext in ["png", "svg"] {
                        // hicolor/64x64/apps y también Papirus/64x64/apps
                        let c = base.join(t).join(sub).join(format!("{nombre}.{ext}"));
                        if c.is_file() {
                            return Some(c);
                        }
                        // breeze/apps/48
                        let c = base.join(sub).join(t.split('x').next().unwrap_or(t)).join(format!("{nombre}.{ext}"));
                        if c.is_file() {
                            return Some(c);
                        }
                    }
                }
            }
        }
    }
    for d in dirs_datos() {
        for ext in ["png", "svg"] {
            let c = d.join("pixmaps").join(format!("{nombre}.{ext}"));
            if c.is_file() {
                return Some(c);
            }
        }
    }
    None
}

fn cargar(ruta: &Path) -> Option<ColorImage> {
    let datos = std::fs::read(ruta).ok()?;
    let es_svg = ruta.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    if es_svg {
        let arbol = resvg::usvg::Tree::from_data(&datos, &resvg::usvg::Options::default()).ok()?;
        let tam = arbol.size();
        let escala = TAM as f32 / tam.width().max(tam.height()).max(1.0);
        let mut lienzo = resvg::tiny_skia::Pixmap::new(TAM, TAM)?;
        let dx = (TAM as f32 - tam.width() * escala) / 2.0;
        let dy = (TAM as f32 - tam.height() * escala) / 2.0;
        resvg::render(&arbol, resvg::tiny_skia::Transform::from_row(escala, 0.0, 0.0, escala, dx, dy), &mut lienzo.as_mut());
        // tiny-skia da RGBA premultiplicado
        Some(ColorImage::from_rgba_premultiplied([TAM as usize, TAM as usize], lienzo.data()))
    } else {
        let icono = eframe::icon_data::from_png_bytes(&datos).ok()?;
        Some(ColorImage::from_rgba_unmultiplied([icono.width as usize, icono.height as usize], &icono.rgba))
    }
}

enum Estado {
    /// Buscándose/leyéndose en otro hilo
    Cargando,
    Listo(Option<TextureHandle>),
}

/// Texturas de los íconos. Buscar un ícono en los temas y dibujar un SVG
/// toma unos milisegundos, así que se hace en otro hilo: mientras tanto se
/// ve el glifo de respaldo y, al llegar, el ícono real.
pub struct Cache {
    texturas: HashMap<String, Estado>,
    /// Pedidos al hilo que carga (uno solo, aunque pidas 300 íconos)
    pedidos: Option<std::sync::mpsc::Sender<String>>,
    tx: std::sync::mpsc::Sender<(String, Option<ColorImage>)>,
    rx: std::sync::mpsc::Receiver<(String, Option<ColorImage>)>,
}

impl Default for Cache {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self { texturas: HashMap::new(), pedidos: None, tx, rx }
    }
}

impl Cache {
    /// La textura del ícono, si ya está lista (None mientras carga o si no existe).
    pub fn textura(&mut self, ctx: &egui::Context, icono: &str) -> Option<TextureHandle> {
        // Recoger lo que ya terminó
        while let Ok((nombre, img)) = self.rx.try_recv() {
            let t = img.map(|img| ctx.load_texture(format!("programa:{nombre}"), img, TextureOptions::LINEAR));
            self.texturas.insert(nombre, Estado::Listo(t));
        }
        match self.texturas.get(icono) {
            Some(Estado::Listo(t)) => return t.clone(),
            Some(Estado::Cargando) => return None,
            None => {}
        }
        if self.pedidos.is_none() {
            let (tx_p, rx_p) = std::sync::mpsc::channel::<String>();
            let (tx, ctx) = (self.tx.clone(), ctx.clone());
            let lanzado = std::thread::Builder::new().name("solar-iconos".into()).spawn(move || {
                // Termina solo cuando se suelta la caché
                for nombre in rx_p {
                    let img = buscar(&nombre).and_then(|r| cargar(&r));
                    if tx.send((nombre, img)).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            });
            if lanzado.is_ok() {
                self.pedidos = Some(tx_p);
            }
        }
        let enviado = self.pedidos.as_ref().is_some_and(|p| p.send(icono.to_string()).is_ok());
        self.texturas
            .insert(icono.to_string(), if enviado { Estado::Cargando } else { Estado::Listo(None) });
        None
    }

    pub fn vaciar(&mut self) {
        self.texturas.clear();
    }
}
