//! Temas: aplica los colores del tema (que vienen del núcleo/Lua) a egui,
//! y la matemática de colores para dibujar carpetas de cualquier color.

use crate::nucleo::{ColorTema, Tema};
use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Theme, ThemePreference, Vec2,
};
use std::cell::RefCell;
use std::collections::HashMap;

/// Colores ya listos para pintar (con la transparencia aplicada donde va).
#[derive(Clone, Debug)]
pub struct Paleta {
    pub oscuro: bool,
    pub fondo: Color32,
    pub superficie: Color32,
    pub elevado: Color32,
    pub borde: Color32,
    pub texto: Color32,
    pub texto_tenue: Color32,
    pub acento: Color32,
    pub texto_acento: Color32,
    pub seleccion: Color32,
    pub seleccion_borde: Color32,
    pub hover: Color32,
    pub peligro: Color32,
    pub exito: Color32,
    pub aviso: Color32,
    pub carpeta: Color32,
}

pub fn con_alfa(c: Color32, alfa: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (alfa.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Mezcla en sRGB (t = 0 -> a, t = 1 -> b).
pub fn mezclar(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()), m(a.a(), b.a()))
}

pub fn paleta(tema: &Tema, transparencia: bool, opacidad: f32) -> Paleta {
    let alfa = if transparencia { opacidad.clamp(0.3, 1.0) } else { 1.0 };
    let acento = tema.c(ColorTema::Acento);
    let elevado = tema.c(ColorTema::Elevado);
    Paleta {
        oscuro: tema.oscuro,
        fondo: con_alfa(tema.c(ColorTema::Fondo), alfa),
        // Las barras un poco más opacas que el fondo para que se lean bien
        superficie: con_alfa(tema.c(ColorTema::Superficie), (alfa + 0.08).min(1.0)),
        elevado,
        borde: tema.c(ColorTema::Borde),
        texto: tema.c(ColorTema::Texto),
        texto_tenue: tema.c(ColorTema::TextoTenue),
        acento,
        texto_acento: tema.c(ColorTema::TextoAcento),
        seleccion: con_alfa(tema.c(ColorTema::Seleccion), if tema.oscuro { 0.24 } else { 0.18 }),
        seleccion_borde: con_alfa(tema.c(ColorTema::Seleccion), 0.75),
        hover: con_alfa(tema.c(ColorTema::Texto), if tema.oscuro { 0.06 } else { 0.05 }),
        peligro: tema.c(ColorTema::Peligro),
        exito: tema.c(ColorTema::Exito),
        aviso: tema.c(ColorTema::Aviso),
        carpeta: tema.c(ColorTema::Carpeta),
    }
}

/// Aplica el tema a egui (widgets, menús, ventanas, texto…).
pub fn aplicar(ctx: &egui::Context, tema: &Tema, p: &Paleta) {
    let mut v = if tema.oscuro { egui::Visuals::dark() } else { egui::Visuals::light() };
    let texto = p.texto;
    let acento = p.acento;
    let elevado = p.elevado;
    let base_boton = mezclar(elevado, texto, if tema.oscuro { 0.07 } else { 0.10 });
    let hover = mezclar(elevado, acento, 0.16);
    let activo = mezclar(elevado, acento, 0.30);
    let radio = CornerRadius::same(8);

    v.dark_mode = tema.oscuro;
    v.panel_fill = p.superficie;
    v.window_fill = elevado;
    v.extreme_bg_color = mezclar(tema.c(ColorTema::Fondo), elevado, 0.35);
    v.text_edit_bg_color = Some(mezclar(tema.c(ColorTema::Fondo), elevado, 0.5));
    v.faint_bg_color = mezclar(tema.c(ColorTema::Fondo), elevado, 0.45);
    v.code_bg_color = elevado;
    v.window_stroke = Stroke::new(1.0, p.borde);
    v.window_corner_radius = CornerRadius::same(14);
    v.menu_corner_radius = CornerRadius::same(10);
    v.window_shadow = Shadow { offset: [0, 10], blur: 28, spread: 0, color: Color32::from_black_alpha(110) };
    v.popup_shadow = Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(90) };
    v.hyperlink_color = acento;
    v.warn_fg_color = p.aviso;
    v.error_fg_color = p.peligro;
    v.weak_text_color = Some(p.texto_tenue);
    v.override_text_color = None;
    v.selection.bg_fill = con_alfa(acento, if tema.oscuro { 0.45 } else { 0.30 });
    v.selection.stroke = Stroke::new(1.0, texto);
    v.text_cursor.stroke = Stroke::new(2.0, acento);
    v.indent_has_left_vline = false;
    v.collapsing_header_frame = false;

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = elevado;
    w.noninteractive.weak_bg_fill = elevado;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.borde);
    w.noninteractive.fg_stroke = Stroke::new(1.0, texto);
    w.noninteractive.corner_radius = radio;

    w.inactive.bg_fill = base_boton;
    w.inactive.weak_bg_fill = base_boton;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0, texto);
    w.inactive.corner_radius = radio;

    w.hovered.bg_fill = hover;
    w.hovered.weak_bg_fill = hover;
    w.hovered.bg_stroke = Stroke::new(1.0, con_alfa(acento, 0.5));
    w.hovered.fg_stroke = Stroke::new(1.5, texto);
    w.hovered.corner_radius = radio;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = activo;
    w.active.weak_bg_fill = activo;
    w.active.bg_stroke = Stroke::new(1.0, acento);
    w.active.fg_stroke = Stroke::new(2.0, texto);
    w.active.corner_radius = radio;
    w.active.expansion = 0.0;

    w.open.bg_fill = hover;
    // También es el fondo del título de las ventanas (Ajustes)
    w.open.weak_bg_fill = mezclar(elevado, texto, if tema.oscuro { 0.035 } else { 0.03 });
    w.open.bg_stroke = Stroke::new(1.0, p.borde);
    w.open.fg_stroke = Stroke::new(1.0, texto);
    w.open.corner_radius = radio;

    let tema_egui = if tema.oscuro { Theme::Dark } else { Theme::Light };
    ctx.set_theme(if tema.oscuro { ThemePreference::Dark } else { ThemePreference::Light });
    ctx.set_visuals_of(tema_egui, v);
    ctx.style_mut_of(tema_egui, |s| {
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        s.spacing.button_padding = Vec2::new(10.0, 5.0);
        s.spacing.menu_margin = Margin::same(6);
        s.spacing.window_margin = Margin::same(16);
        s.spacing.interact_size = Vec2::new(36.0, 26.0);
        s.spacing.slider_width = 180.0;
        s.spacing.combo_width = 160.0;
        s.spacing.scroll = egui::style::ScrollStyle::floating();
        s.spacing.scroll.bar_width = 8.0;
        s.text_styles = [
            (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
        ]
        .into();
        s.interaction.selectable_labels = false;
    });
}

// ======================================================================
//  OKLab: para sacar tonos claros/oscuros que se vean bien
// ======================================================================

fn a_lineal(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn a_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 { 12.92 * c } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}

pub fn a_oklab(c: Color32) -> [f32; 3] {
    let r = a_lineal(c.r() as f32 / 255.0);
    let g = a_lineal(c.g() as f32 / 255.0);
    let b = a_lineal(c.b() as f32 / 255.0);
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

fn oklab_a_rgb(lab: [f32; 3]) -> [f32; 3] {
    let l = lab[0] + 0.396_337_78 * lab[1] + 0.215_803_76 * lab[2];
    let m = lab[0] - 0.105_561_346 * lab[1] - 0.063_854_17 * lab[2];
    let s = lab[0] - 0.089_484_18 * lab[1] - 1.291_485_5 * lab[2];
    let (l, m, s) = (l * l * l, m * m * m, s * s * s);
    [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ]
}

/// De OKLab a sRGB; si el color se sale de la gama, baja la saturación.
pub fn de_oklab(lab: [f32; 3]) -> Color32 {
    let dentro = |c: [f32; 3]| c.iter().all(|v| (-0.0005..=1.0005).contains(v));
    let mut rgb = oklab_a_rgb(lab);
    if !dentro(rgb) {
        let (mut bajo, mut alto) = (0.0f32, 1.0f32);
        for _ in 0..14 {
            let medio = (bajo + alto) / 2.0;
            if dentro(oklab_a_rgb([lab[0], lab[1] * medio, lab[2] * medio])) { bajo = medio } else { alto = medio }
        }
        rgb = oklab_a_rgb([lab[0], lab[1] * bajo, lab[2] * bajo]);
    }
    let a8 = |v: f32| (a_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8;
    Color32::from_rgb(a8(rgb[0]), a8(rgb[1]), a8(rgb[2]))
}

/// Cambia la luminosidad (en OKLab) y escala la saturación.
pub fn tono(base: Color32, delta_l: f32, croma: f32) -> Color32 {
    let lab = a_oklab(base);
    de_oklab([(lab[0] + delta_l).clamp(0.0, 1.0), lab[1] * croma, lab[2] * croma])
}

pub fn luminosidad(c: Color32) -> f32 {
    a_oklab(c)[0]
}

/// Texto legible encima de un color (negro o blanco).
pub fn texto_sobre(c: Color32) -> Color32 {
    if luminosidad(c) > 0.68 { Color32::from_rgb(0x1A, 0x12, 0x0C) } else { Color32::WHITE }
}

/// Tonos de una carpeta de cierto color, sacados del logo de Solar:
/// trasera 600→700, frontal 300→400 y logotipo 700 (escala tipo Tailwind).
#[derive(Clone, Copy, Debug)]
pub struct PaletaCarpeta {
    pub trasera_arriba: Color32,
    pub trasera_abajo: Color32,
    pub frontal_arriba: Color32,
    pub frontal_abajo: Color32,
    pub logotipo: Color32,
}

thread_local! {
    static CACHE_CARPETAS: RefCell<HashMap<Color32, PaletaCarpeta>> = RefCell::new(HashMap::new());
}

pub fn paleta_carpeta(base: Color32) -> PaletaCarpeta {
    CACHE_CARPETAS.with(|c| {
        if let Some(p) = c.borrow().get(&base) {
            return *p;
        }
        // El naranja de la marca usa exactamente los colores del logo
        if base == NARANJA_SOLAR {
            let p = PaletaCarpeta {
                trasera_arriba: Color32::from_rgb(0xEA, 0x58, 0x0C),
                trasera_abajo: Color32::from_rgb(0xC2, 0x41, 0x0C),
                frontal_arriba: Color32::from_rgb(0xFD, 0xBA, 0x74),
                frontal_abajo: Color32::from_rgb(0xFB, 0x92, 0x3C),
                logotipo: Color32::from_rgb(0xC2, 0x41, 0x0C),
            };
            c.borrow_mut().insert(base, p);
            return p;
        }
        let l = luminosidad(base);
        // Carpetas muy oscuras: el logotipo va claro para que se vea
        let logotipo = if l < 0.42 { tono(base, 0.38, 0.55) } else { tono(base, -0.155, 0.92) };
        let p = PaletaCarpeta {
            trasera_arriba: tono(base, -0.058, 1.0),
            trasera_abajo: tono(base, -0.150, 0.92),
            frontal_arriba: tono(base, 0.130, 0.62),
            frontal_abajo: tono(base, 0.045, 0.88),
            logotipo,
        };
        let mut cache = c.borrow_mut();
        if cache.len() > 512 {
            cache.clear();
        }
        cache.insert(base, p);
        p
    })
}

pub const NARANJA_SOLAR: Color32 = Color32::from_rgb(0xF9, 0x73, 0x16);

/// Colores listos para carpetas y logotipos (el usuario también puede
/// elegir uno personalizado).
pub const COLORES: [(&str, Color32); 21] = [
    ("Naranja solar", Color32::from_rgb(0xF9, 0x73, 0x16)),
    ("Ámbar", Color32::from_rgb(0xF5, 0x9E, 0x0B)),
    ("Amarillo", Color32::from_rgb(0xEA, 0xB3, 0x08)),
    ("Lima", Color32::from_rgb(0x84, 0xCC, 0x16)),
    ("Verde", Color32::from_rgb(0x22, 0xC5, 0x5E)),
    ("Verde oscuro", Color32::from_rgb(0x15, 0x80, 0x3D)),
    ("Turquesa", Color32::from_rgb(0x14, 0xB8, 0xA6)),
    ("Cian", Color32::from_rgb(0x06, 0xB6, 0xD4)),
    ("Azul cielo", Color32::from_rgb(0x0E, 0xA5, 0xE9)),
    ("Azul", Color32::from_rgb(0x3B, 0x82, 0xF6)),
    ("Índigo", Color32::from_rgb(0x63, 0x66, 0xF1)),
    ("Violeta", Color32::from_rgb(0x8B, 0x5C, 0xF6)),
    ("Morado", Color32::from_rgb(0xA8, 0x55, 0xF7)),
    ("Rosa", Color32::from_rgb(0xEC, 0x48, 0x99)),
    ("Rojo", Color32::from_rgb(0xEF, 0x44, 0x44)),
    ("Vino", Color32::from_rgb(0x9F, 0x12, 0x39)),
    ("Café", Color32::from_rgb(0x92, 0x40, 0x0E)),
    ("Gris", Color32::from_rgb(0x6B, 0x72, 0x80)),
    ("Grafito", Color32::from_rgb(0x37, 0x41, 0x51)),
    ("Negro", Color32::from_rgb(0x11, 0x18, 0x27)),
    ("Blanco", Color32::from_rgb(0xF3, 0xF4, 0xF6)),
];

pub fn nombre_color(c: Color32) -> String {
    COLORES
        .iter()
        .find(|(_, k)| *k == c)
        .map(|(n, _)| n.to_string())
        .unwrap_or_else(|| hex(c))
}

pub fn hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

pub fn de_hex(s: &str) -> Option<Color32> {
    let s = s.trim().trim_start_matches('#');
    let v = u32::from_str_radix(s, 16).ok()?;
    match s.len() {
        6 => Some(Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)),
        3 => Some(Color32::from_rgb(((v >> 8) & 0xF) as u8 * 17, ((v >> 4) & 0xF) as u8 * 17, (v & 0xF) as u8 * 17)),
        _ => None,
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn cerca(a: Color32, b: Color32) -> bool {
        let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
        d(a.r(), b.r()) <= 24 && d(a.g(), b.g()) <= 24 && d(a.b(), b.b()) <= 24
    }

    #[test]
    fn la_carpeta_naranja_es_la_del_logo() {
        let p = paleta_carpeta(NARANJA_SOLAR);
        assert_eq!(p.frontal_arriba, Color32::from_rgb(0xFD, 0xBA, 0x74));
        assert_eq!(p.trasera_abajo, Color32::from_rgb(0xC2, 0x41, 0x0C));
    }

    #[test]
    fn otros_colores_siguen_la_misma_receta() {
        // Azul de Tailwind: 600/700 atrás, 300/400 adelante (sin el giro de tono)
        let p = paleta_carpeta(Color32::from_rgb(0x3B, 0x82, 0xF6));
        assert!(cerca(p.trasera_arriba, Color32::from_rgb(0x25, 0x63, 0xEB)), "{:?}", p.trasera_arriba);
        assert!(cerca(p.frontal_abajo, Color32::from_rgb(0x60, 0xA5, 0xFA)), "{:?}", p.frontal_abajo);
        assert!(luminosidad(p.frontal_arriba) > luminosidad(p.frontal_abajo));
        assert!(luminosidad(p.trasera_abajo) < luminosidad(p.trasera_arriba));
        // Carpeta negra: logotipo claro
        let n = paleta_carpeta(Color32::from_rgb(0x11, 0x18, 0x27));
        assert!(luminosidad(n.logotipo) > 0.5);
    }

    #[test]
    fn hex_ida_y_vuelta() {
        assert_eq!(de_hex("#F97316"), Some(Color32::from_rgb(0xF9, 0x73, 0x16)));
        assert_eq!(de_hex("fff"), Some(Color32::WHITE));
        assert_eq!(hex(Color32::from_rgb(0x15, 0x80, 0x3D)), "#15803D");
        assert_eq!(de_hex("nada"), None);
    }
}
