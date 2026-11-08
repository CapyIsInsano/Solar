//! Íconos vectoriales: carpetas con la misma forma que el logo de Solar
//! (de cualquier color y con logotipo), archivos por tipo y el sol del logo.
//!
//! Todo se dibuja con mallas de egui, así se ve nítido a cualquier tamaño.

use crate::nucleo::Categoria;
use crate::tema::{self, con_alfa};
use eframe::egui::{
    Align2, Color32, FontFamily, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, epaint::Vertex, pos2, vec2,
};
use egui_phosphor::regular as ph;

pub const FUENTE_RELLENA: &str = "phosphor-fill";

// ======================================================================
//  Logotipos para carpetas
// ======================================================================

pub struct Emblema {
    pub id: &'static str,
    pub nombre: &'static str,
    pub glifo: &'static str,
}

macro_rules! emblemas {
    ($($id:literal, $nombre:literal, $glifo:ident;)*) => {
        pub const EMBLEMAS: &[Emblema] = &[$(Emblema { id: $id, nombre: $nombre, glifo: ph::$glifo }),*];
    };
}

emblemas! {
    "mando", "Mando", GAME_CONTROLLER;
    "musica", "Música", MUSIC_NOTES;
    "codigo", "Código", CODE;
    "terminal", "Terminal", TERMINAL_WINDOW;
    "imagen", "Imágenes", IMAGE;
    "camara", "Fotos", CAMERA;
    "video", "Videos", FILM_STRIP;
    "documento", "Documentos", FILE_TEXT;
    "descarga", "Descargas", DOWNLOAD_SIMPLE;
    "casa", "Casa", HOUSE;
    "escritorio", "Escritorio", DESKTOP;
    "estrella", "Estrella", STAR;
    "corazon", "Corazón", HEART;
    "nube", "Nube", CLOUD;
    "candado", "Privado", LOCK;
    "trabajo", "Trabajo", BRIEFCASE;
    "escuela", "Escuela", GRADUATION_CAP;
    "libro", "Libros", BOOK_OPEN;
    "arte", "Arte", PALETTE;
    "pincel", "Diseño", PAINT_BRUSH;
    "audifonos", "Audífonos", HEADPHONES;
    "microfono", "Micrófono", MICROPHONE;
    "chip", "Electrónica", CPU;
    "herramienta", "Herramientas", WRENCH;
    "cohete", "Cohete", ROCKET;
    "planeta", "Planeta", PLANET;
    "sol", "Sol", SUN;
    "luna", "Luna", MOON;
    "rayo", "Rayo", LIGHTNING;
    "fuego", "Fuego", FIRE;
    "globo", "Web", GLOBE;
    "dinero", "Dinero", CURRENCY_DOLLAR;
    "cartera", "Cartera", WALLET;
    "paquete", "Paquetes", PACKAGE;
    "datos", "Datos", DATABASE;
    "usb", "USB", USB;
    "huella", "Mascotas", PAW_PRINT;
    "gato", "Gato", CAT;
    "perro", "Perro", DOG;
    "flor", "Flor", FLOWER;
    "arbol", "Naturaleza", TREE;
    "cafe", "Café", COFFEE;
    "pizza", "Comida", PIZZA;
    "avion", "Viajes", AIRPLANE;
    "auto", "Autos", CAR;
    "trofeo", "Trofeo", TROPHY;
    "bandera", "Bandera", FLAG;
    "ciencia", "Ciencia", FLASK;
    "atomo", "Átomo", ATOM;
    "calavera", "Calavera", SKULL;
    "escudo", "Seguridad", SHIELD;
    "llave", "Llaves", KEY;
    "calendario", "Calendario", CALENDAR;
    "notas", "Notas", PENCIL;
}

pub fn emblema(id: &str) -> Option<&'static Emblema> {
    EMBLEMAS.iter().find(|e| e.id == id)
}

/// Qué se dibuja encima de una carpeta.
#[derive(Clone, Copy)]
pub enum Logotipo<'a> {
    Ninguno,
    Glifo(&'static str),
    Texto(&'a str),
    Sol,
}

impl<'a> Logotipo<'a> {
    pub fn de(icono: Option<&'a str>) -> Self {
        match icono {
            None | Some("") => Self::Ninguno,
            Some("sol") => Self::Sol,
            Some(s) if s.starts_with("txt:") => {
                let t = &s[4..];
                if t.is_empty() { Self::Ninguno } else { Self::Texto(t) }
            }
            Some(s) => emblema(s).map(|e| Self::Glifo(e.glifo)).unwrap_or(Self::Ninguno),
        }
    }
}

// ======================================================================
//  Geometría
// ======================================================================

/// Transforma coordenadas del lienzo de 512×512 del logo a la pantalla.
#[derive(Clone, Copy)]
struct Lienzo {
    origen: Pos2,
    escala: f32,
}

impl Lienzo {
    /// Encaja la caja (x0, y0)-(x1, y1) del logo, centrada, en `rect`.
    fn encajar(rect: Rect, x0: f32, y0: f32, x1: f32, y1: f32, margen: f32) -> Self {
        let disponible = rect.size() * (1.0 - margen);
        let escala = (disponible.x / (x1 - x0)).min(disponible.y / (y1 - y0));
        let centro_logo = pos2((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let centro = rect.center();
        Self { origen: centro - centro_logo.to_vec2() * escala, escala }
    }

    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origen + vec2(x, y) * self.escala
    }

    fn y(&self, y: f32) -> f32 {
        self.origen.y + y * self.escala
    }
}

/// Constructor de contornos con curvas cuadráticas (como en el SVG).
struct Trazo {
    puntos: Vec<Pos2>,
}

impl Trazo {
    fn en(p: Pos2) -> Self {
        Self { puntos: vec![p] }
    }

    fn linea(mut self, p: Pos2) -> Self {
        self.puntos.push(p);
        self
    }

    fn curva(mut self, control: Pos2, fin: Pos2) -> Self {
        let inicio = *self.puntos.last().unwrap_or(&control);
        let pasos = 6;
        for i in 1..=pasos {
            let t = i as f32 / pasos as f32;
            let u = 1.0 - t;
            let p = inicio.to_vec2() * (u * u) + control.to_vec2() * (2.0 * u * t) + fin.to_vec2() * (t * t);
            self.puntos.push(p.to_pos2());
        }
        self
    }

    fn cerrar(mut self) -> Vec<Pos2> {
        if self.puntos.len() > 1 && self.puntos.first() == self.puntos.last() {
            self.puntos.pop();
        }
        self.puntos
    }
}

fn degradado(arriba: Color32, abajo: Color32, y0: f32, y1: f32, y: f32) -> Color32 {
    let t = if (y1 - y0).abs() < 0.001 { 0.0 } else { ((y - y0) / (y1 - y0)).clamp(0.0, 1.0) };
    tema::mezclar(arriba, abajo, t)
}

/// Rellena un polígono convexo con un degradado vertical y bordes suaves.
fn poligono(painter: &Painter, puntos: &[Pos2], arriba: Color32, abajo: Color32, y0: f32, y1: f32, alfa: f32) {
    let n = puntos.len();
    if n < 3 {
        return;
    }
    let ppp = painter.ctx().pixels_per_point();
    let suave = 1.0 / ppp;
    let mut area = 0.0;
    for i in 0..n {
        let (a, b) = (puntos[i], puntos[(i + 1) % n]);
        area += a.x * b.y - b.x * a.y;
    }
    let signo = if area >= 0.0 { 1.0 } else { -1.0 };

    let mut mesh = Mesh::default();
    mesh.reserve_vertices(n * 2);
    mesh.reserve_triangles(n * 3);
    for i in 0..n {
        let anterior = puntos[(i + n - 1) % n];
        let actual = puntos[i];
        let siguiente = puntos[(i + 1) % n];
        let e0 = (actual - anterior).normalized();
        let e1 = (siguiente - actual).normalized();
        let n0 = vec2(e0.y, -e0.x) * signo;
        let n1 = vec2(e1.y, -e1.x) * signo;
        let mut normal = n0 + n1;
        let largo = normal.length();
        normal = if largo > 1e-4 { normal / largo } else { n0 };
        let coseno = normal.dot(n0).max(0.35);
        let normal = normal / coseno;
        let color = con_alfa(degradado(arriba, abajo, y0, y1, actual.y), alfa * (arriba.a() as f32 / 255.0));
        mesh.vertices.push(Vertex { pos: actual - normal * (suave * 0.5), uv: Default::default(), color });
        mesh.vertices.push(Vertex { pos: actual + normal * (suave * 0.5), uv: Default::default(), color: Color32::TRANSPARENT });
    }
    for i in 1..n - 1 {
        mesh.add_triangle(0, (2 * i) as u32, (2 * (i + 1)) as u32);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (ai, ao, bi, bo) = ((2 * i) as u32, (2 * i + 1) as u32, (2 * j) as u32, (2 * j + 1) as u32);
        mesh.add_triangle(ai, ao, bi);
        mesh.add_triangle(ao, bo, bi);
    }
    painter.add(Shape::mesh(mesh));
}

// ======================================================================
//  Carpetas
// ======================================================================

/// Dibuja una carpeta como la del logo de Solar dentro de `rect`.
pub fn carpeta(
    painter: &Painter,
    rect: Rect,
    color: Color32,
    logotipo: Logotipo,
    color_logotipo: Option<Color32>,
    alfa: f32,
) {
    let pc = tema::paleta_carpeta(color);
    let l = Lienzo::encajar(rect, 62.0, 88.0, 448.0, 424.0, 0.10);

    // Carpeta trasera: cuerpo + pestaña (dos piezas convexas del mismo color)
    let cuerpo = Trazo::en(l.p(64.0, 144.0))
        .curva(l.p(64.0, 120.0), l.p(88.0, 120.0))
        .linea(l.p(448.0, 120.0))
        .linea(l.p(448.0, 400.0))
        .curva(l.p(448.0, 424.0), l.p(424.0, 424.0))
        .linea(l.p(88.0, 424.0))
        .curva(l.p(64.0, 424.0), l.p(64.0, 400.0))
        .cerrar();
    let pestana = Trazo::en(l.p(284.0, 120.0))
        .linea(l.p(316.0, 88.0))
        .linea(l.p(424.0, 88.0))
        .curva(l.p(448.0, 88.0), l.p(448.0, 112.0))
        .linea(l.p(448.0, 150.0))
        .linea(l.p(284.0, 150.0))
        .cerrar();
    let (ya, yb) = (l.y(88.0), l.y(424.0));
    poligono(painter, &cuerpo, pc.trasera_arriba, pc.trasera_abajo, ya, yb, alfa);
    poligono(painter, &pestana, pc.trasera_arriba, pc.trasera_abajo, ya, yb, alfa);

    // Carpeta frontal (inclinada)
    let frente = Trazo::en(l.p(144.0, 176.0))
        .linea(l.p(424.0, 176.0))
        .curva(l.p(448.0, 176.0), l.p(448.0, 200.0))
        .linea(l.p(448.0, 400.0))
        .curva(l.p(448.0, 424.0), l.p(424.0, 424.0))
        .linea(l.p(90.0, 424.0))
        .curva(l.p(62.0, 424.0), l.p(68.0, 398.0))
        .linea(l.p(120.0, 194.0))
        .curva(l.p(125.0, 176.0), l.p(144.0, 176.0))
        .cerrar();
    poligono(painter, &frente, pc.frontal_arriba, pc.frontal_abajo, l.y(176.0), l.y(424.0), alfa);

    // Logotipo (no se dibuja en íconos diminutos)
    let lado = rect.width().min(rect.height());
    if lado < 26.0 {
        return;
    }
    let color_logo = con_alfa(color_logotipo.unwrap_or(pc.logotipo), alfa);
    let centro = l.p(268.0, 302.0);
    match logotipo {
        Logotipo::Ninguno => {}
        Logotipo::Sol => sol(painter, l, color_logo),
        Logotipo::Glifo(g) => {
            let tam = 168.0 * l.escala;
            painter.text(centro, Align2::CENTER_CENTER, g, FontId::new(tam, FontFamily::Name(FUENTE_RELLENA.into())), color_logo);
        }
        Logotipo::Texto(t) => {
            let n = t.chars().count().max(1) as f32;
            let tam = (150.0 / n.sqrt()) * l.escala;
            painter.text(centro, Align2::CENTER_CENTER, t, FontId::proportional(tam), color_logo);
        }
    }
}

/// El sol del logo (inclinado -14°), en coordenadas del lienzo.
fn sol(painter: &Painter, l: Lienzo, color: Color32) {
    let inclinar = |x: f32, y: f32| l.p(x - (14.0f32).to_radians().tan() * (y - 302.0), y);
    let (cx, cy) = (270.0, 302.0);
    let grosor = 14.0 * l.escala;
    for i in 0..8 {
        let a = (i as f32) * std::f32::consts::FRAC_PI_4;
        let (s, c) = a.sin_cos();
        let p0 = inclinar(cx + s * 60.0, cy - c * 60.0);
        let p1 = inclinar(cx + s * 84.0, cy - c * 84.0);
        painter.line_segment([p0, p1], Stroke::new(grosor, color));
        painter.circle_filled(p0, grosor / 2.0, color);
        painter.circle_filled(p1, grosor / 2.0, color);
    }
    let nucleo: Vec<Pos2> = (0..36)
        .map(|i| {
            let a = i as f32 / 36.0 * std::f32::consts::TAU;
            inclinar(cx + a.cos() * 40.0, cy + a.sin() * 40.0)
        })
        .collect();
    poligono(painter, &nucleo, color, color, 0.0, 1.0, 1.0);
}

/// El logo completo de Solar (carpeta naranja con el sol).
pub fn logo(painter: &Painter, rect: Rect) {
    carpeta(painter, rect, Color32::from_rgb(0xF9, 0x73, 0x16), Logotipo::Sol, None, 1.0);
}

// ======================================================================
//  Archivos
// ======================================================================

pub fn color_categoria(c: Categoria) -> Color32 {
    use Categoria::*;
    let rgb = |r, g, b| Color32::from_rgb(r, g, b);
    match c {
        Carpeta => rgb(0xF9, 0x73, 0x16),
        Texto => rgb(0x94, 0xA3, 0xB8),
        Codigo => rgb(0x3B, 0x82, 0xF6),
        Imagen => rgb(0xA8, 0x55, 0xF7),
        Audio => rgb(0xEC, 0x48, 0x99),
        Video => rgb(0xEF, 0x44, 0x44),
        Comprimido => rgb(0xF5, 0x9E, 0x0B),
        Pdf => rgb(0xDC, 0x26, 0x26),
        Documento => rgb(0x25, 0x63, 0xEB),
        HojaCalculo => rgb(0x16, 0xA3, 0x4A),
        Presentacion => rgb(0xEA, 0x58, 0x0C),
        Ejecutable => rgb(0x10, 0xB9, 0x81),
        Fuente => rgb(0x8B, 0x5C, 0xF6),
        Disco => rgb(0x64, 0x74, 0x8B),
        Paquete => rgb(0xD9, 0x77, 0x06),
        Generico => rgb(0x9C, 0xA3, 0xAF),
    }
}

pub fn glifo_categoria(c: Categoria) -> Option<&'static str> {
    use Categoria::*;
    Some(match c {
        Texto | Documento => ph::TEXT_ALIGN_LEFT,
        Codigo => ph::CODE,
        Imagen => ph::IMAGE,
        Audio => ph::MUSIC_NOTE,
        Video => ph::PLAY,
        Comprimido => ph::ARCHIVE,
        HojaCalculo => ph::TABLE,
        Presentacion => ph::PRESENTATION_CHART,
        Ejecutable => ph::GEAR_SIX,
        Fuente => ph::TEXT_AA,
        Disco => ph::DISC,
        Paquete => ph::PACKAGE,
        Carpeta => ph::FOLDER,
        Pdf | Generico => return None,
    })
}

/// Dibuja un archivo (hoja con esquina doblada) del color de su tipo.
pub fn archivo(painter: &Painter, rect: Rect, categoria: Categoria, nombre: &str, enlace_roto: bool, alfa: f32) {
    let base = if enlace_roto { Color32::from_rgb(0x6B, 0x72, 0x80) } else { color_categoria(categoria) };
    let arriba = tema::tono(base, 0.10, 0.85);
    let abajo = tema::tono(base, -0.02, 1.0);
    let doblez = tema::tono(base, 0.26, 0.45);
    let l = Lienzo::encajar(rect, 112.0, 56.0, 400.0, 456.0, 0.10);

    let hoja = Trazo::en(l.p(112.0, 80.0))
        .curva(l.p(112.0, 56.0), l.p(136.0, 56.0))
        .linea(l.p(300.0, 56.0))
        .linea(l.p(400.0, 156.0))
        .linea(l.p(400.0, 432.0))
        .curva(l.p(400.0, 456.0), l.p(376.0, 456.0))
        .linea(l.p(136.0, 456.0))
        .curva(l.p(112.0, 456.0), l.p(112.0, 432.0))
        .cerrar();
    poligono(painter, &hoja, arriba, abajo, l.y(56.0), l.y(456.0), alfa);
    let esquina = Trazo::en(l.p(300.0, 56.0))
        .linea(l.p(400.0, 156.0))
        .linea(l.p(326.0, 156.0))
        .curva(l.p(300.0, 156.0), l.p(300.0, 130.0))
        .cerrar();
    poligono(painter, &esquina, doblez, doblez, 0.0, 1.0, alfa);

    let lado = rect.width().min(rect.height());
    if lado < 22.0 {
        return;
    }
    let tinta = con_alfa(Color32::from_white_alpha(235), alfa);
    let centro = l.p(256.0, 300.0);
    if enlace_roto {
        painter.text(centro, Align2::CENTER_CENTER, ph::LINK_BREAK, FontId::new(150.0 * l.escala, FontFamily::Name(FUENTE_RELLENA.into())), tinta);
        return;
    }
    match glifo_categoria(categoria) {
        Some(g) => {
            painter.text(centro, Align2::CENTER_CENTER, g, FontId::new(150.0 * l.escala, FontFamily::Name(FUENTE_RELLENA.into())), tinta);
        }
        None => {
            let etiqueta = if categoria == Categoria::Pdf {
                "PDF".to_string()
            } else {
                nombre
                    .rsplit_once('.')
                    .filter(|(a, e)| !a.is_empty() && !e.is_empty() && e.len() <= 4)
                    .map(|(_, e)| e.to_uppercase())
                    .unwrap_or_default()
            };
            if !etiqueta.is_empty() && lado >= 30.0 {
                let tam = if etiqueta.chars().count() > 3 { 78.0 } else { 96.0 } * l.escala;
                painter.text(centro, Align2::CENTER_CENTER, etiqueta, FontId::proportional(tam), tinta);
            }
        }
    }
}

/// Indicador de enlace simbólico (flechita en la esquina).
pub fn marca_enlace(painter: &Painter, rect: Rect, paleta_fondo: Color32, tinta: Color32) {
    let lado = rect.width().min(rect.height());
    let r = (lado * 0.16).max(6.0);
    let c = rect.left_bottom() + vec2(r + lado * 0.08, -r - lado * 0.06);
    painter.circle_filled(c, r, paleta_fondo);
    painter.text(c, Align2::CENTER_CENTER, ph::ARROW_BEND_UP_RIGHT, FontId::proportional(r * 1.3), tinta);
}

/// Un círculo de color (para elegir colores en menús).
pub fn muestra_color(painter: &Painter, centro: Pos2, radio: f32, color: Color32, seleccionado: bool, borde: Color32) {
    painter.circle_filled(centro, radio, color);
    painter.circle_stroke(centro, radio, Stroke::new(1.0, con_alfa(borde, 0.6)));
    if seleccionado {
        painter.circle_stroke(centro, radio + 3.0, Stroke::new(2.0, borde));
        painter.text(centro, Align2::CENTER_CENTER, ph::CHECK, FontId::proportional(radio * 1.1), tema::texto_sobre(color));
    }
}
