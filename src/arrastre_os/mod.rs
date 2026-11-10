//! Arrastrar y soltar con el resto del escritorio: sacar archivos de Solar
//! hacia otra ventana de Solar, una terminal (que escribe la ruta) o
//! cualquier programa que acepte archivos, y recibir los que vienen de afuera.
//!
//! winit todavía no sabe *empezar* un arrastre, y en Wayland tampoco sabe
//! recibirlos, así que aquí se habla directo con el servidor gráfico:
//!
//! * Wayland: un hilo con su propia cola sobre la misma conexión de la
//!   ventana (wl_data_device).
//! * X11: protocolo XDND con una conexión aparte. Recibir ya lo hace winit;
//!   aquí solo se averigua dónde está el puntero al soltar.

use eframe::egui;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};

mod icono;
mod wayland;
mod x11;

/// Lo que pasó desde el último cuadro.
#[derive(Debug)]
pub enum Evento {
    /// Un arrastre de afuera va pasando por la ventana (en puntos de egui)
    Encima(egui::Pos2),
    /// Se fue sin soltar nada
    Salio,
    /// Soltaron archivos en la ventana
    Soltado { rutas: Vec<PathBuf>, pos: Option<egui::Pos2>, copiar: bool },
    /// Terminó el arrastre que empezamos nosotros
    PropioTermino,
}

/// Mensajes de los hilos (las posiciones aún en unidades de la ventana).
enum Crudo {
    Encima(f64, f64),
    Salio,
    Soltado { rutas: Vec<PathBuf>, pos: (f64, f64), copiar: bool },
    PropioTermino,
}

enum Motor {
    Wayland(wayland::Wayland),
    X11(x11::X11),
}

pub struct ArrastreOs {
    motor: Motor,
    rx: mpsc::Receiver<Crudo>,
    arrastrando: bool,
    /// X11: winit avisa que hay archivos encima; nosotros seguimos el puntero
    x11_encima: bool,
}

impl ArrastreOs {
    pub fn nuevo(cc: &eframe::CreationContext<'_>) -> Option<Self> {
        let display = cc.display_handle().ok()?.as_raw();
        let ventana = cc.window_handle().ok()?.as_raw();
        let (tx, rx) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();
        let motor = match (display, ventana) {
            (RawDisplayHandle::Wayland(d), RawWindowHandle::Wayland(v)) => {
                Motor::Wayland(wayland::Wayland::nuevo(d.display.as_ptr(), v.surface.as_ptr(), tx, ctx)?)
            }
            (RawDisplayHandle::Xlib(d), RawWindowHandle::Xlib(v)) => {
                Motor::X11(x11::X11::nuevo(d.display.map(|p| p.as_ptr()), v.window as u32, tx, ctx)?)
            }
            (RawDisplayHandle::Xcb(_), RawWindowHandle::Xcb(v)) => Motor::X11(x11::X11::nuevo(None, v.window.get(), tx, ctx)?),
            _ => return None,
        };
        Some(Self { motor, rx, arrastrando: false, x11_encima: false })
    }

    /// ¿Hay un arrastre nuestro en curso fuera de la ventana?
    pub fn arrastrando(&self) -> bool {
        self.arrastrando
    }

    /// Pasa el arrastre al sistema (el puntero ya salió de la ventana con el
    /// botón apretado). Devuelve `false` si no se pudo.
    /// `color` es el del tema, para el dibujito que acompaña al puntero.
    pub fn iniciar(&mut self, rutas: Vec<PathBuf>, color: egui::Color32) -> bool {
        if self.arrastrando || rutas.is_empty() {
            return false;
        }
        let icono = icono::Icono { cuantos: rutas.len(), carpeta: rutas[0].is_dir(), color: [color.r(), color.g(), color.b()] };
        let ok = match &mut self.motor {
            Motor::Wayland(w) => w.iniciar(rutas, icono),
            Motor::X11(x) => x.iniciar(rutas),
        };
        self.arrastrando = ok;
        ok
    }

    /// Lo que pasó desde el último cuadro. En X11 también se queda con los
    /// archivos que soltó winit, para saber en qué parte de la ventana cayeron.
    pub fn eventos(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) -> Vec<Evento> {
        // Wayland da coordenadas lógicas; X11, píxeles
        let escala = match self.motor {
            Motor::Wayland(_) => ctx.zoom_factor(),
            Motor::X11(_) => raw.viewports.get(&raw.viewport_id).and_then(|v| v.native_pixels_per_point).unwrap_or(1.0)
                * ctx.zoom_factor(),
        };
        let punto = |x: f64, y: f64| egui::pos2(x as f32 / escala, y as f32 / escala);
        let mut eventos = Vec::new();
        while let Ok(c) = self.rx.try_recv() {
            eventos.push(match c {
                Crudo::Encima(x, y) => Evento::Encima(punto(x, y)),
                Crudo::Salio => Evento::Salio,
                Crudo::Soltado { rutas, pos, copiar } => Evento::Soltado { rutas, pos: Some(punto(pos.0, pos.1)), copiar },
                Crudo::PropioTermino => {
                    self.arrastrando = false;
                    Evento::PropioTermino
                }
            });
        }
        if let Motor::X11(x) = &self.motor {
            if !raw.dropped_files.is_empty() {
                let rutas: Vec<PathBuf> = std::mem::take(&mut raw.dropped_files)
                    .iter()
                    .map(|f| f.path().to_path_buf())
                    .filter(|p| !p.as_os_str().is_empty())
                    .collect();
                self.x11_encima = false;
                if !rutas.is_empty() {
                    let pos = x.puntero().map(|(a, b)| punto(a, b));
                    eventos.push(Evento::Soltado { rutas, pos, copiar: false });
                }
            } else if !raw.hovered_files.is_empty() {
                if let Some((a, b)) = x.puntero() {
                    eventos.push(Evento::Encima(punto(a, b)));
                }
                self.x11_encima = true;
                // Mientras dura, nadie más avisa que el puntero se movió
                ctx.request_repaint_after(std::time::Duration::from_millis(30));
            } else if self.x11_encima {
                self.x11_encima = false;
                eventos.push(Evento::Salio);
            }
        }
        eventos
    }
}

// ----------------------------------------------------------------------
//  Formatos
// ----------------------------------------------------------------------

/// Tipos que ofrecemos, del más útil al menos.
pub(crate) const TIPOS: [&str; 4] = ["text/uri-list", "text/plain;charset=utf-8", "UTF8_STRING", "text/plain"];

/// Lo que se manda para cada tipo: lista de URIs para los gestores de
/// archivos, rutas listas para pegar en una terminal para el resto.
pub(crate) fn datos_para(tipo: &str, rutas: &[PathBuf]) -> Option<Vec<u8>> {
    match tipo {
        "text/uri-list" | "x-special/gnome-copied-files" => Some(uri_list(rutas).into_bytes()),
        t if t.starts_with("text/plain") || t == "UTF8_STRING" || t == "STRING" || t == "TEXT" => {
            Some(texto_para_terminal(rutas).into_bytes())
        }
        _ => None,
    }
}

/// `file:///ruta%20con%20espacios\r\n` por archivo.
pub(crate) fn uri_list(rutas: &[PathBuf]) -> String {
    let mut s = String::new();
    for r in rutas {
        s.push_str("file://");
        s.push_str(&codificar_ruta(r));
        s.push_str("\r\n");
    }
    s
}

fn codificar_ruta(r: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut s = String::new();
    for &b in r.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~!$&'()*+,;=:@".contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

/// Rutas entre comillas simples si hace falta, separadas por espacio y con
/// un espacio al final (para seguir escribiendo el comando).
pub(crate) fn texto_para_terminal(rutas: &[PathBuf]) -> String {
    let mut s = String::new();
    for r in rutas {
        s.push_str(&citar(&r.to_string_lossy()));
        s.push(' ');
    }
    s
}

fn citar(t: &str) -> String {
    let seguro = !t.is_empty() && t.bytes().all(|b| b.is_ascii_alphanumeric() || b"/-_.,+:@%=".contains(&b));
    if seguro { t.to_string() } else { format!("'{}'", t.replace('\'', r"'\''")) }
}

/// Saca las rutas de una lista de URIs (o de texto con rutas sueltas).
pub(crate) fn rutas_de(texto: &str) -> Vec<PathBuf> {
    let mut rutas = Vec::new();
    for l in texto.lines() {
        let l = l.trim_matches(|c: char| c == '\r' || c == '\0').trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let ruta = if let Some(resto) = l.strip_prefix("file://") {
            // file:///ruta o file://localhost/ruta
            let resto = match resto.find('/') {
                Some(0) => resto,
                Some(i) if &resto[..i] == "localhost" || resto[..i] == *nombre_equipo() => &resto[i..],
                _ => continue,
            };
            PathBuf::from(decodificar(resto))
        } else if l.starts_with('/') {
            PathBuf::from(l)
        } else {
            continue;
        };
        rutas.push(ruta);
    }
    rutas
}

fn nombre_equipo() -> &'static String {
    static NOMBRE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    NOMBRE.get_or_init(|| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()).unwrap_or_default())
}

pub(crate) fn decodificar(s: &str) -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt;
    let b = s.as_bytes();
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(a), Some(c)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push(a * 16 + c);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    std::ffi::OsString::from_vec(out)
}

// ----------------------------------------------------------------------
//  Tuberías (Wayland pasa los datos por un fd)
// ----------------------------------------------------------------------

fn bloqueante(fd: &OwnedFd) {
    unsafe {
        let f = libc::fcntl(fd.as_raw_fd(), libc::F_GETFL);
        if f >= 0 {
            libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, f & !libc::O_NONBLOCK);
        }
    }
}

/// Escribe en otro hilo: si el que lee somos nosotros mismos, no se traba.
fn escribir_en(fd: OwnedFd, datos: Vec<u8>) {
    std::thread::spawn(move || {
        bloqueante(&fd);
        let mut f = std::fs::File::from(fd);
        let _ = f.write_all(&datos);
    });
}

/// Lee todo (máximo 16 MiB, por si acaso).
fn leer_de(fd: OwnedFd) -> std::io::Result<String> {
    bloqueante(&fd);
    let f = std::fs::File::from(fd);
    let mut v = Vec::new();
    f.take(16 << 20).read_to_end(&mut v)?;
    Ok(String::from_utf8_lossy(&v).into_owned())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn ida_y_vuelta() {
        let rutas = vec![PathBuf::from("/home/capy/Mis cosas/año 2026.txt"), PathBuf::from("/tmp/a#b%c")];
        let u = uri_list(&rutas);
        assert_eq!(u, "file:///home/capy/Mis%20cosas/a%C3%B1o%202026.txt\r\nfile:///tmp/a%23b%25c\r\n");
        assert_eq!(rutas_de(&u), rutas);
        assert_eq!(rutas_de("# comentario\nfile://localhost/etc/hosts\n/usr/bin\nhttp://x/y"), vec![
            PathBuf::from("/etc/hosts"),
            PathBuf::from("/usr/bin")
        ]);
    }

    #[test]
    fn terminal() {
        let rutas = vec![PathBuf::from("/tmp/simple.txt"), PathBuf::from("/tmp/con espacio"), PathBuf::from("/tmp/it's")];
        assert_eq!(texto_para_terminal(&rutas), r"/tmp/simple.txt '/tmp/con espacio' '/tmp/it'\''s' ");
    }
}
