//! Piezas de interfaz reutilizables: botones de ícono, interruptores,
//! rejillas de colores y de logotipos.

use crate::iconos::{self, EMBLEMAS, FUENTE_RELLENA};
use crate::tema::{self, Paleta, con_alfa};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontFamily, FontId, Response, RichText, Sense, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo, WidgetType, pos2, vec2,
};

/// Como `Response::dnd_release_payload`, pero sin tirar una carga de otro
/// tipo: egui la saca aunque no sea la que se pide, y entonces el que sí la
/// esperaba (en el mismo widget o en otro) ya no la encuentra.
pub fn soltado<T: std::any::Any + Send + Sync>(resp: &Response) -> Option<std::sync::Arc<T>> {
    if egui::DragAndDrop::has_payload_of_type::<T>(&resp.ctx) { resp.dnd_release_payload::<T>() } else { None }
}

/// Botón cuadrado con un ícono de Phosphor, sin marco hasta que pasas el mouse.
pub fn boton_icono(ui: &mut Ui, p: &Paleta, glifo: &str, ayuda: &str, habilitado: bool) -> Response {
    boton_icono_estado(ui, p, glifo, ayuda, habilitado, false)
}

pub fn boton_icono_estado(ui: &mut Ui, p: &Paleta, glifo: &str, ayuda: &str, habilitado: bool, activo: bool) -> Response {
    let tam = vec2(32.0, 30.0);
    let sentido = if habilitado { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(tam, sentido);
    if habilitado && resp.clicked() {
        crate::sonido::pedir(crate::sonido::Sonido::Clic);
    }
    if ui.is_rect_visible(rect) {
        // El fondo aparece y se va suave; al presionar se encoge un poquito
        let h = ui.ctx().animate_bool_with_time(resp.id.with("h"), habilitado && resp.hovered(), 0.12);
        let presionado = ui.ctx().animate_bool_with_time(resp.id.with("p"), habilitado && resp.is_pointer_button_down_on(), 0.08);
        let caja = rect.shrink(presionado * 1.5);
        let fondo = if activo {
            con_alfa(p.acento, 0.22)
        } else if presionado > 0.0 {
            tema::mezclar(p.hover, con_alfa(p.texto, 0.14), presionado)
        } else {
            con_alfa(p.hover, h * p.hover.a() as f32 / 255.0)
        };
        ui.painter().rect_filled(caja, CornerRadius::same(8), fondo);
        let color = if !habilitado {
            con_alfa(p.texto_tenue, 0.45)
        } else if activo {
            p.acento
        } else {
            p.texto
        };
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, glifo, FontId::proportional(17.0), color);
    }
    let resp = if habilitado { resp.on_hover_cursor(CursorIcon::PointingHand) } else { resp };
    if ayuda.is_empty() { resp } else { resp.on_hover_text(ayuda) }
}

/// Botón principal (con el color de acento).
pub fn boton_acento(ui: &mut Ui, p: &Paleta, texto: &str) -> Response {
    ui.add(egui::Button::new(RichText::new(texto).color(p.texto_acento).strong()).fill(p.acento).min_size(vec2(120.0, 32.0)))
        .on_hover_cursor(CursorIcon::PointingHand)
}

/// Interruptor tipo celular.
pub fn interruptor(ui: &mut Ui, p: &Paleta, encendido: &mut bool) -> Response {
    let tam = vec2(42.0, 24.0);
    let (rect, mut resp) = ui.allocate_exact_size(tam, Sense::click());
    if resp.clicked() {
        *encendido = !*encendido;
        resp.mark_changed();
        crate::sonido::pedir(crate::sonido::Sonido::Clic);
    }
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, ui.is_enabled(), *encendido, ""));
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_responsive(resp.id, *encendido);
        let apagado = tema::mezclar(p.elevado, p.texto, 0.22);
        let fondo = tema::mezclar(apagado, p.acento, t);
        let radio = rect.height() / 2.0;
        ui.painter().rect(rect, CornerRadius::same(radio as u8), fondo, Stroke::NONE, StrokeKind::Inside);
        let x = egui::lerp((rect.left() + radio)..=(rect.right() - radio), t);
        ui.painter().circle_filled(pos2(x, rect.center().y), radio * 0.72, Color32::WHITE);
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Una fila de ajustes: título y explicación a la izquierda, control a la derecha.
pub fn fila<R>(ui: &mut Ui, p: &Paleta, titulo: &str, explicacion: &str, control: impl FnOnce(&mut Ui) -> R) -> R {
    let mut salida = None;
    let ancho_texto = (ui.available_width() - 230.0).max(160.0);
    // Medimos el texto antes de colocar nada: así el texto y el control
    // quedan centrados en la misma línea, sin importar cuál sea más alto
    let g_titulo = ui.painter().layout(titulo.to_owned(), egui::TextStyle::Body.resolve(ui.style()), p.texto, ancho_texto);
    let g_expl = (!explicacion.is_empty()).then(|| {
        ui.painter().layout(explicacion.to_owned(), egui::TextStyle::Small.resolve(ui.style()), p.texto_tenue, ancho_texto)
    });
    let alto_texto = g_titulo.size().y + g_expl.as_ref().map_or(0.0, |g| 3.0 + g.size().y);
    let alto = (alto_texto + 12.0).max(32.0);
    ui.horizontal(|ui| {
        ui.set_min_height(alto);
        let (rect, resp) = ui.allocate_exact_size(vec2(ancho_texto, alto), Sense::hover());
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, titulo));
        let y = rect.center().y - alto_texto / 2.0;
        let alto_titulo = g_titulo.size().y;
        ui.painter().galley(pos2(rect.left(), y), g_titulo, p.texto);
        if let Some(g) = g_expl {
            ui.painter().galley(pos2(rect.left(), y + alto_titulo + 3.0), g, p.texto_tenue);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            salida = Some(control(ui));
        });
    });
    ui.add_space(2.0);
    salida.expect("el control siempre se dibuja")
}

pub fn encabezado(ui: &mut Ui, p: &Paleta, texto: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(texto.to_uppercase()).small().strong().color(p.texto_tenue));
    ui.add_space(2.0);
}

pub fn titulo(ui: &mut Ui, p: &Paleta, texto: &str) {
    ui.label(RichText::new(texto).size(18.0).strong().color(p.texto));
    ui.add_space(8.0);
}

/// Rejilla de colores listos. Devuelve el color elegido.
pub fn rejilla_colores(ui: &mut Ui, p: &Paleta, actual: Option<Color32>, por_fila: usize, radio: f32) -> Option<Color32> {
    let mut elegido = None;
    let paso = radio * 2.0 + 8.0;
    for fila in tema::COLORES.chunks(por_fila) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (nombre, color) in fila {
                let (rect, resp) = ui.allocate_exact_size(vec2(paso, paso), Sense::click());
                let sel = actual == Some(*color);
                let r = if resp.hovered() { radio + 1.5 } else { radio };
                iconos::muestra_color(ui.painter(), rect.center(), r, *color, sel, p.texto);
                if resp.on_hover_text(*nombre).on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    elegido = Some(*color);
                }
            }
        });
    }
    elegido
}

/// Rejilla de logotipos (filtrable). Devuelve el id elegido.
pub fn rejilla_logotipos(
    ui: &mut Ui,
    p: &Paleta,
    actual: Option<&str>,
    filtro: &str,
    por_fila: usize,
    tam: f32,
) -> Option<&'static str> {
    let filtro = crate::formato::plegar(filtro.trim());
    let lista: Vec<_> = EMBLEMAS
        .iter()
        .filter(|e| filtro.is_empty() || crate::formato::plegar(e.nombre).contains(&filtro) || e.id.contains(&filtro))
        .collect();
    if lista.is_empty() {
        ui.label(RichText::new("No hay logotipos con ese nombre.").color(p.texto_tenue));
        return None;
    }
    let mut elegido = None;
    for fila in lista.chunks(por_fila) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for e in fila {
                let (rect, resp) = ui.allocate_exact_size(Vec2::splat(tam), Sense::click());
                let sel = actual == Some(e.id);
                let fondo = if sel {
                    con_alfa(p.acento, 0.28)
                } else if resp.hovered() {
                    p.hover
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(rect, CornerRadius::same(8), fondo);
                if sel {
                    ui.painter().rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.5, p.acento), StrokeKind::Inside);
                }
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    e.glifo,
                    FontId::new(tam * 0.55, FontFamily::Name(FUENTE_RELLENA.into())),
                    if sel { p.acento } else { p.texto },
                );
                if resp.on_hover_text(e.nombre).on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    elegido = Some(e.id);
                }
            }
        });
    }
    elegido
}

/// Ícono pequeño de un lugar o logotipo (glifo relleno).
pub fn glifo_relleno(glifo: &str, tam: f32, color: Color32) -> RichText {
    RichText::new(glifo).font(FontId::new(tam, FontFamily::Name(FUENTE_RELLENA.into()))).color(color)
}

/// Barra de capacidad (para memorias USB y discos).
pub fn barra_capacidad(ui: &mut Ui, p: &Paleta, usado: f32, ancho: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ancho, 4.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(2), con_alfa(p.texto, 0.12));
    let mut lleno = rect;
    lleno.set_width(rect.width() * usado.clamp(0.0, 1.0));
    let color = if usado > 0.9 { p.peligro } else if usado > 0.75 { p.aviso } else { p.acento };
    ui.painter().rect_filled(lleno, CornerRadius::same(2), color);
}
