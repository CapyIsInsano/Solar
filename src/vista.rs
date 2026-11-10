//! La vista de archivos: cuadrícula y lista, selección (clic, Ctrl, Shift y
//! con banda), arrastrar y soltar, teclado y menús del clic derecho.

use crate::app::{Accion, Arrastre, Banda, CambioEstilo, ModoPortapapeles, Solar, Ubicacion};
use crate::iconos::{self, Logotipo};
use crate::nucleo::{Entrada, Orden, TipoEntrada, Vista};
use crate::tema::{Paleta, con_alfa};
use crate::ui_util;
use crate::dialogos;
use crate::nucleo::{Afinidad, Programa};
use crate::{formato, nucleo};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, Event, FontId, Frame, Galley, Key, Margin, PointerButton, Pos2,
    Rect, Response, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, text::LayoutJob, vec2,
};
use egui_phosphor::regular as ph;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

const ALTO_FILA: f32 = 30.0;

/// Lo que se necesita para dibujar un elemento (copiado para no pelear con el
/// verificador de préstamos mientras se ejecutan acciones).
/// Lo que se dibuja de un archivo: apunta a la entrada compartida (Arc), así
/// que crearlo en cada cuadro no copia nada.
struct Item {
    entrada: Arc<Entrada>,
}

impl std::ops::Deref for Item {
    type Target = Entrada;
    fn deref(&self) -> &Entrada {
        &self.entrada
    }
}


fn texto_ajustado(painter: &egui::Painter, texto: &str, ancho: f32, filas: usize, tam: f32, color: Color32, centrado: bool) -> Arc<Galley> {
    let mut job = LayoutJob::simple(texto.to_string(), FontId::proportional(tam), color, ancho.max(10.0));
    job.wrap.max_rows = filas;
    job.wrap.break_anywhere = false;
    job.wrap.overflow_character = Some('…');
    if centrado {
        job.halign = Align::Center;
    }
    painter.layout_job(job)
}

fn pintar_galley_centrado(painter: &egui::Painter, centro_x: f32, y: f32, galley: Arc<Galley>, color: Color32) {
    let x = centro_x - galley.rect.center().x;
    painter.galley(pos2(x, y), galley, color);
}

impl Solar {
    fn item(&self, indice_visible: usize) -> Item {
        let e = &self.entradas[self.visibles[indice_visible]];
        Item { entrada: e.clone() }
    }

    fn cortado(&self, ruta: &PathBuf) -> bool {
        self.portapapeles.as_ref().is_some_and(|p| p.modo == ModoPortapapeles::Cortar && p.rutas.contains(ruta))
    }

    /// Opacidad y desplazamiento de un elemento al abrir la carpeta
    /// (aparecen en cascada, de arriba hacia abajo).
    fn aparicion(&self, orden: usize) -> (f32, f32) {
        let Some(desde) = self.carga_animada else { return (1.0, 0.0) };
        if !self.config.animaciones {
            return (1.0, 0.0);
        }
        let t = (desde.elapsed().as_secs_f32() - orden.min(30) as f32 * 0.016) / 0.22;
        let e = crate::app::ease(t);
        (e.max(0.0), (1.0 - e) * 12.0)
    }

    /// 0 → 1 → 0 durante el "salto" de una carpeta recién personalizada.
    fn salto(&self, ruta: &PathBuf) -> f32 {
        if !self.config.animaciones {
            return 0.0;
        }
        self.pop
            .get(ruta)
            .map(|t| {
                let x = (t.elapsed().as_secs_f32() / 0.38).clamp(0.0, 1.0);
                (x * std::f32::consts::PI).sin() * (1.0 - x * 0.3)
            })
            .unwrap_or(0.0)
    }

    fn limpiar_animaciones(&mut self, ctx: &egui::Context) {
        let mut sigue = false;
        if let Some(d) = self.carga_animada {
            if d.elapsed().as_secs_f32() > 0.22 + 30.0 * 0.016 || !self.config.animaciones {
                self.carga_animada = None;
            } else {
                sigue = true;
            }
        }
        self.pop.retain(|_, t| t.elapsed().as_secs_f32() < 0.4);
        sigue |= !self.pop.is_empty() && self.config.animaciones;
        if sigue {
            ctx.request_repaint();
        }
    }

    /// Dibuja el ícono de una entrada (carpeta con su estilo o archivo).
    pub(crate) fn pintar_icono(&self, painter: &egui::Painter, rect: Rect, e: &Entrada, alfa: f32) {
        match e.tipo {
            TipoEntrada::Carpeta => {
                // Sin copiar el estilo: esto corre en cada cuadro por cada carpeta visible
                let (color, icono, color_icono) = match &e.estilo {
                    Some(s) => (s.color.unwrap_or(self.paleta.carpeta), s.icono.as_deref(), s.color_icono),
                    None => (self.paleta.carpeta, None, None),
                };
                iconos::carpeta(painter, rect, color, Logotipo::de(icono), color_icono, alfa);
            }
            TipoEntrada::EnlaceRoto => iconos::archivo(painter, rect, e.categoria, &e.nombre, true, alfa),
            _ => iconos::archivo(painter, rect, e.categoria, &e.nombre, false, alfa),
        }
        if e.es_enlace && rect.width() >= 32.0 {
            iconos::marca_enlace(painter, rect, self.paleta.elevado, self.paleta.texto);
        }
    }

    // ------------------------------------------------------------------
    //  Panel central
    // ------------------------------------------------------------------

    pub(crate) fn vista_central(&mut self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        let p = self.paleta.clone();
        egui::CentralPanel::default().frame(Frame::new().fill(p.fondo)).show(ui, |ui| {
            if self.ubicacion == Ubicacion::Papelera {
                self.aviso_papelera(ui, &p, acciones);
            }
            if self.config.vista == Vista::Lista {
                self.encabezado_lista(ui, &p, acciones);
            }
            match self.config.vista {
                Vista::Cuadricula => self.cuadricula(ui, &p, acciones),
                Vista::Lista => self.lista(ui, &p, acciones),
            }
        });
    }

    fn aviso_papelera(&mut self, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        Frame::new().fill(p.superficie).inner_margin(Margin::symmetric(16, 10)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(ph::TRASH).size(18.0).color(p.texto_tenue));
                ui.label(
                    RichText::new("Lo que está aquí se puede restaurar hasta que vacíes la papelera.")
                        .color(p.texto_tenue),
                );
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let hay = !self.entradas.is_empty();
                    let vaciar = egui::Button::new(RichText::new(format!("{}  Vaciar papelera", ph::BROOM)).color(
                        if hay { p.peligro } else { p.texto_tenue },
                    ));
                    if ui.add_enabled(hay, vaciar).clicked() {
                        acciones.push(Accion::VaciarPapelera);
                    }
                    let sel = self.seleccionados();
                    let texto = if sel.is_empty() { "Restaurar todo" } else { "Restaurar selección" };
                    if ui
                        .add_enabled(hay, egui::Button::new(format!("{}  {texto}", ph::ARROW_COUNTER_CLOCKWISE)))
                        .clicked()
                    {
                        let rutas = if sel.is_empty() { self.entradas.iter().map(|e| e.ruta.clone()).collect() } else { sel };
                        acciones.push(Accion::Restaurar(rutas));
                    }
                });
            });
        });
    }

    fn estado_vacio(&self, ui: &Ui, p: &Paleta, area: Rect) {
        let cargando = self.cargando_desde.is_some_and(|t| t.elapsed().as_millis() > 150);
        let (glifo, titulo, detalle): (&str, String, String) = if let Some(err) = &self.error_listado {
            (ph::WARNING_CIRCLE, "No se puede mostrar esta carpeta".into(), err.clone())
        } else if cargando {
            (ph::HOURGLASS, "Cargando…".into(), String::new())
        } else if self.cargando_desde.is_some() {
            return;
        } else if let Some(b) = self.busqueda.as_ref().filter(|b| !b.trim().is_empty()) {
            (ph::MAGNIFYING_GLASS, format!("Nada coincide con «{}»", b.trim()), "Prueba con otra palabra.".into())
        } else if self.ubicacion == Ubicacion::Papelera {
            (ph::TRASH, "La papelera está vacía".into(), String::new())
        } else if !self.entradas.is_empty() {
            (ph::EYE_SLASH, "Aquí solo hay archivos ocultos".into(), "Presiona Ctrl+H para verlos.".into())
        } else {
            (ph::FOLDER_OPEN, "Esta carpeta está vacía".into(), "Arrastra archivos aquí o usa el clic derecho para crear algo.".into())
        };
        let painter = ui.painter_at(area);
        let c = area.center() - vec2(0.0, 30.0);
        painter.text(c, Align2::CENTER_CENTER, glifo, FontId::proportional(54.0), con_alfa(p.texto_tenue, 0.55));
        painter.text(c + vec2(0.0, 50.0), Align2::CENTER_CENTER, titulo, FontId::proportional(17.0), p.texto);
        if !detalle.is_empty() {
            let g = texto_ajustado(&painter, &detalle, (area.width() - 60.0).max(120.0), 3, 13.0, p.texto_tenue, true);
            pintar_galley_centrado(&painter, c.x, c.y + 66.0, g, p.texto_tenue);
        }
    }

    // ------------------------------------------------------------------
    //  Cuadrícula
    // ------------------------------------------------------------------

    fn cuadricula(&mut self, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        let icono = self.config.tamano_iconos.round();
        let margen = 12.0;
        let ancho_min = (icono + 46.0).max(96.0);
        let alto = icono + 60.0;
        let disponible = (ui.available_width() - margen * 2.0).max(ancho_min);
        let cols = ((disponible / ancho_min).floor() as usize).max(1);
        let ancho = disponible / cols as f32;
        self.columnas = cols;
        let n = self.visibles.len();
        let filas = n.div_ceil(cols);
        let alto_total = filas as f32 * alto + margen * 2.0;
        let celda = move |k: usize| {
            let (f, c) = (k / cols, k % cols);
            Rect::from_min_size(pos2(margen + c as f32 * ancho, margen + f as f32 * alto), vec2(ancho, alto))
        };

        ScrollArea::vertical()
            .id_salt("cuadricula")
            .auto_shrink(false)
            .scroll_source(egui::scroll_area::ScrollSource { drag: egui::scroll_area::DragScroll::Never, ..Default::default() })
            .show_viewport(ui, |ui, viewport| {
                let origen = ui.max_rect().min;
                ui.set_min_size(vec2(ui.available_width(), alto_total.max(viewport.height())));
                self.filas_visibles = ((viewport.height() / alto) as usize).max(1);
                let visible = viewport.translate(origen.to_vec2());
                let fondo = ui.interact(visible, ui.id().with("fondo"), Sense::click_and_drag());

                let primera = (((viewport.top() - margen) / alto).floor().max(0.0)) as usize;
                let ultima = (((viewport.bottom() - margen) / alto).ceil().max(0.0)) as usize + 1;
                for k in (primera * cols)..(ultima * cols).min(n) {
                    let rect = celda(k).translate(origen.to_vec2());
                    self.celda(ui, p, k, k - primera * cols, rect, icono, acciones);
                }
                self.limpiar_animaciones(ui.ctx());
                if self.desplazar_a_foco {
                    if let Some(k) = self.foco.clone().and_then(|f| self.indice_visible(&f)) {
                        ui.scroll_to_rect(celda(k).translate(origen.to_vec2()), None);
                    }
                    self.desplazar_a_foco = false;
                }
                if n == 0 {
                    self.estado_vacio(ui, p, visible);
                }
                self.fondo(ui, p, &fondo, origen, &celda, acciones);
            });
    }

    #[allow(clippy::too_many_arguments)]
    fn celda(&mut self, ui: &mut Ui, p: &Paleta, k: usize, orden: usize, rect: Rect, icono: f32, acciones: &mut Vec<Accion>) {
        let it = self.item(k);
        let resp = ui.interact(rect.shrink(4.0), ui.id().with(("celda", &it.ruta)), Sense::click_and_drag());
        let seleccionado = self.seleccion.contains(&it.ruta);
        let soltar_aqui = it.es_carpeta()
            && resp.dnd_hover_payload::<Arrastre>().is_some_and(|a| !a.0.contains(&it.ruta));
        // Animaciones: aparecer al abrir la carpeta, levantarse con el mouse,
        // encogerse al presionar y "saltar" al cambiar de color o logotipo
        let (aparicion, dy) = self.aparicion(orden);
        let ctx = ui.ctx().clone();
        let h = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered() && !seleccionado, 0.14);
        let presion = ctx.animate_bool_with_time(resp.id.with("p"), resp.is_pointer_button_down_on(), 0.08);
        let salto = self.salto(&it.ruta);
        let mut pintor = ui.painter().clone();
        pintor.multiply_opacity(aparicion);
        let painter = &pintor;
        let rect = rect.translate(vec2(0.0, dy));
        let caja = rect.shrink(5.0);
        let radio = CornerRadius::same(10);
        if seleccionado {
            painter.rect(caja, radio, p.seleccion, Stroke::new(1.0, p.seleccion_borde), StrokeKind::Inside);
        } else if soltar_aqui {
            painter.rect(caja, radio, con_alfa(p.acento, 0.18), Stroke::new(2.0, p.acento), StrokeKind::Inside);
        } else if h > 0.0 {
            painter.rect_filled(caja, radio, con_alfa(p.hover, h * p.hover.a() as f32 / 255.0));
        }

        let alfa = if self.cortado(&it.ruta) { 0.45 } else if it.entrada.es_oculto { 0.65 } else { 1.0 };
        let escala = 1.0 + 0.05 * h - 0.05 * presion + 0.14 * salto;
        let rect_icono = Rect::from_center_size(
            pos2(rect.center().x, rect.top() + 10.0 + icono / 2.0 - 3.0 * h),
            vec2(icono * escala, icono * escala),
        );
        self.pintar_icono(painter, rect_icono, &it.entrada, alfa);

        let color = if alfa < 1.0 { con_alfa(p.texto, 0.75) } else { p.texto };
        let g = texto_ajustado(painter, &it.nombre, rect.width() - 14.0, 2, 13.0, color, true);
        pintar_galley_centrado(painter, rect.center().x, rect.top() + 16.0 + icono, g, color);

        let resp = resp.on_hover_ui_at_pointer(|ui| self.tooltip(ui, &it.entrada));
        self.interacciones(ui, &resp, &it, acciones);
    }

    fn tooltip(&self, ui: &mut Ui, e: &Entrada) {
        ui.label(RichText::new(&e.nombre).strong());
        let mut detalle = e.descripcion.to_string();
        if e.tipo == TipoEntrada::Archivo {
            detalle.push_str(" · ");
            detalle.push_str(&formato::tamano(e.tamano));
        }
        ui.label(RichText::new(detalle).small());
        ui.label(RichText::new(format!("Modificado: {}", formato::fecha(e.modificado))).small());
    }

    // ------------------------------------------------------------------
    //  Lista
    // ------------------------------------------------------------------

    fn columnas_lista(ancho: f32) -> (f32, Option<f32>, Option<f32>, Option<f32>) {
        let tam = (ancho >= 420.0).then_some(96.0);
        let fecha = (ancho >= 520.0).then_some(150.0);
        let tipo = (ancho >= 700.0).then_some(180.0);
        let nombre = ancho - tam.unwrap_or(0.0) - fecha.unwrap_or(0.0) - tipo.unwrap_or(0.0);
        (nombre, tam, tipo, fecha)
    }

    fn encabezado_lista(&mut self, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        let ancho = ui.available_width() - 24.0;
        let (nombre, tam, tipo, fecha) = Self::columnas_lista(ancho);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
        let painter = ui.painter();
        painter.line_segment(
            [rect.left_bottom() + vec2(12.0, 0.0), rect.right_bottom() - vec2(12.0, 0.0)],
            Stroke::new(1.0, con_alfa(p.borde, 0.8)),
        );
        let mut x = rect.left() + 12.0;
        let columnas = [
            (Some(nombre), "Nombre", Orden::Nombre, false),
            (tam, "Tamaño", Orden::Tamano, true),
            (tipo, "Tipo", Orden::Tipo, false),
            (fecha, "Modificado", Orden::Fecha, false),
        ];
        for (ancho_col, titulo, orden, derecha) in columnas {
            let Some(w) = ancho_col else { continue };
            let caja = Rect::from_min_size(pos2(x, rect.top()), vec2(w, rect.height()));
            let resp = ui.interact(caja, ui.id().with(("columna", titulo)), Sense::click());
            let activa = self.config.orden == orden;
            let flecha = if !activa {
                ""
            } else if self.config.orden_descendente {
                ph::CARET_DOWN
            } else {
                ph::CARET_UP
            };
            let color = if activa || resp.hovered() { p.texto } else { p.texto_tenue };
            let texto = format!("{titulo} {flecha}");
            let (pos, ancla) = if derecha {
                (pos2(caja.right() - 10.0, caja.center().y), Align2::RIGHT_CENTER)
            } else {
                (pos2(caja.left() + if titulo == "Nombre" { 34.0 } else { 8.0 }, caja.center().y), Align2::LEFT_CENTER)
            };
            ui.painter().text(pos, ancla, texto, FontId::proportional(12.5), color);
            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                acciones.push(Accion::Ordenar(orden));
            }
            x += w;
        }
    }

    fn lista(&mut self, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        let n = self.visibles.len();
        let margen = 6.0;
        let alto_total = n as f32 * ALTO_FILA + margen * 2.0;
        self.columnas = 1;
        let ancho_util = ui.available_width() - 24.0;
        let fila = move |k: usize| {
            Rect::from_min_size(pos2(12.0, margen + k as f32 * ALTO_FILA), vec2(ancho_util, ALTO_FILA))
        };
        ScrollArea::vertical()
            .id_salt("lista")
            .auto_shrink(false)
            .scroll_source(egui::scroll_area::ScrollSource { drag: egui::scroll_area::DragScroll::Never, ..Default::default() })
            .show_viewport(ui, |ui, viewport| {
                let origen = ui.max_rect().min;
                ui.set_min_size(vec2(ui.available_width(), alto_total.max(viewport.height())));
                self.filas_visibles = ((viewport.height() / ALTO_FILA) as usize).max(1);
                let visible = viewport.translate(origen.to_vec2());
                let fondo = ui.interact(visible, ui.id().with("fondo"), Sense::click_and_drag());
                let primera = ((viewport.top() - margen) / ALTO_FILA).floor().max(0.0) as usize;
                let ultima = (((viewport.bottom() - margen) / ALTO_FILA).ceil().max(0.0) as usize + 1).min(n);
                for k in primera..ultima {
                    let rect = fila(k).translate(origen.to_vec2());
                    self.fila_lista(ui, p, k, k - primera, rect, acciones);
                }
                self.limpiar_animaciones(ui.ctx());
                if self.desplazar_a_foco {
                    if let Some(k) = self.foco.clone().and_then(|f| self.indice_visible(&f)) {
                        ui.scroll_to_rect(fila(k).translate(origen.to_vec2()), None);
                    }
                    self.desplazar_a_foco = false;
                }
                if n == 0 {
                    self.estado_vacio(ui, p, visible);
                }
                self.fondo(ui, p, &fondo, origen, &fila, acciones);
            });
    }

    fn fila_lista(&mut self, ui: &mut Ui, p: &Paleta, k: usize, orden: usize, rect: Rect, acciones: &mut Vec<Accion>) {
        let it = self.item(k);
        let resp = ui.interact(rect, ui.id().with(("fila", &it.ruta)), Sense::click_and_drag());
        let seleccionado = self.seleccion.contains(&it.ruta);
        let soltar_aqui = it.es_carpeta()
            && resp.dnd_hover_payload::<Arrastre>().is_some_and(|a| !a.0.contains(&it.ruta));
        let (aparicion, dy) = self.aparicion(orden);
        let h = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered() && !seleccionado, 0.12);
        let salto = self.salto(&it.ruta);
        let mut pintor = ui.painter().clone();
        pintor.multiply_opacity(aparicion);
        let painter = &pintor;
        // En la lista, las filas entran desde un poco a la izquierda
        let rect = rect.translate(vec2(-dy * 0.8, 0.0));
        let radio = CornerRadius::same(6);
        if seleccionado {
            painter.rect(rect, radio, p.seleccion, Stroke::new(1.0, p.seleccion_borde), StrokeKind::Inside);
        } else if soltar_aqui {
            painter.rect(rect, radio, con_alfa(p.acento, 0.18), Stroke::new(2.0, p.acento), StrokeKind::Inside);
        } else {
            if k % 2 == 1 {
                painter.rect_filled(rect, radio, con_alfa(p.texto, 0.025));
            }
            if h > 0.0 {
                painter.rect_filled(rect, radio, con_alfa(p.hover, h * p.hover.a() as f32 / 255.0));
            }
        }
        let alfa = if self.cortado(&it.ruta) { 0.45 } else if it.entrada.es_oculto { 0.65 } else { 1.0 };
        let lado = 24.0 * (1.0 + 0.2 * salto);
        let icono = Rect::from_center_size(pos2(rect.left() + 17.0 + 2.0 * h, rect.center().y), vec2(lado, lado));
        self.pintar_icono(painter, icono, &it.entrada, alfa);

        let (nombre, tam, tipo, fecha) = Self::columnas_lista(rect.width());
        let color = if alfa < 1.0 { con_alfa(p.texto, 0.75) } else { p.texto };
        let g = texto_ajustado(painter, &it.nombre, nombre - 44.0, 1, 13.5, color, false);
        painter.galley(pos2(rect.left() + 34.0, rect.center().y - g.size().y / 2.0), g, color);
        let mut x = rect.left() + nombre;
        let e = &it.entrada;
        if let Some(w) = tam {
            let texto = if e.tipo == TipoEntrada::Archivo { formato::tamano(e.tamano) } else { "—".into() };
            painter.text(pos2(x + w - 10.0, rect.center().y), Align2::RIGHT_CENTER, texto, FontId::proportional(12.5), p.texto_tenue);
            x += w;
        }
        if let Some(w) = tipo {
            let g = texto_ajustado(painter, &e.descripcion, w - 16.0, 1, 12.5, p.texto_tenue, false);
            painter.galley(pos2(x + 8.0, rect.center().y - g.size().y / 2.0), g, p.texto_tenue);
            x += w;
        }
        if let Some(_w) = fecha {
            painter.text(pos2(x + 8.0, rect.center().y), Align2::LEFT_CENTER, formato::fecha(e.modificado), FontId::proportional(12.5), p.texto_tenue);
        }
        self.interacciones(ui, &resp, &it, acciones);
    }

    // ------------------------------------------------------------------
    //  Interacción
    // ------------------------------------------------------------------

    fn interacciones(&mut self, ui: &Ui, resp: &Response, it: &Item, acciones: &mut Vec<Accion>) {
        let m = ui.input(|i| i.modifiers);
        let ruta = &it.ruta;
        if resp.clicked() {
            if m.ctrl || m.command {
                if !self.seleccion.remove(ruta) {
                    self.seleccion.insert(ruta.clone());
                }
                self.ancla = Some(ruta.clone());
                self.foco = Some(ruta.clone());
            } else if m.shift {
                self.seleccionar_rango(ruta, false);
            } else {
                self.seleccionar_solo(ruta);
                if self.config.un_clic {
                    acciones.push(Accion::Abrir(ruta.clone()));
                }
            }
        }
        if resp.double_clicked() && !self.config.un_clic && !m.ctrl && !m.shift {
            acciones.push(Accion::Abrir(ruta.clone()));
        }
        if resp.middle_clicked() && it.es_carpeta() {
            acciones.push(Accion::AbrirEnVentana(ruta.clone()));
        }
        if resp.secondary_clicked() && !self.seleccion.contains(ruta) {
            self.seleccionar_solo(ruta);
        }
        // Arrastrar
        if resp.drag_started_by(PointerButton::Primary) && self.ubicacion != Ubicacion::Papelera {
            if !self.seleccion.contains(ruta) {
                self.seleccionar_solo(ruta);
            }
            resp.dnd_set_drag_payload(Arrastre(self.seleccionados()));
        }
        // Soltar sobre una carpeta
        if it.es_carpeta()
            && let Some(a) = resp.dnd_release_payload::<Arrastre>()
            && !a.0.contains(ruta)
        {
            acciones.push(Accion::Soltar { destino: ruta.clone(), rutas: a.0.clone(), copiar: m.ctrl });
        }
        let en_papelera = self.ubicacion == Ubicacion::Papelera;
        let es_carpeta = it.es_carpeta();
        resp.context_menu(|ui| {
            if en_papelera {
                self.menu_papelera(ui, acciones);
            } else {
                self.menu_elemento(ui, ruta, es_carpeta, acciones);
            }
        });
    }

    /// Clic en el fondo, menú del fondo y selección con banda.
    fn fondo(
        &mut self,
        ui: &mut Ui,
        p: &Paleta,
        fondo: &Response,
        origen: Pos2,
        rect_de: &dyn Fn(usize) -> Rect,
        acciones: &mut Vec<Accion>,
    ) {
        let m = ui.input(|i| i.modifiers);
        if (fondo.clicked() || fondo.secondary_clicked()) && !m.ctrl && !m.shift {
            self.seleccion.clear();
            self.foco = None;
        }
        fondo.context_menu(|ui| self.menu_fondo(ui, acciones));

        if fondo.drag_started_by(PointerButton::Primary)
            && let Some(pos) = fondo.interact_pointer_pos()
        {
            self.banda = Some(Banda {
                inicio: pos - origen.to_vec2(),
                base: if m.ctrl || m.shift { self.seleccion.clone() } else { Default::default() },
            });
        }
        let Some(banda) = &self.banda else { return };
        let suelto = !ui.input(|i| i.pointer.primary_down());
        let Some(puntero) = ui.input(|i| i.pointer.latest_pos()) else { return };
        let actual = puntero - origen.to_vec2();
        let caja = Rect::from_two_pos(banda.inicio, actual);
        let mut nueva = banda.base.clone();
        for (k, &i) in self.visibles.iter().enumerate() {
            if rect_de(k).shrink(8.0).intersects(caja) {
                nueva.insert(self.entradas[i].ruta.clone());
            }
        }
        self.seleccion.reemplazar(nueva);
        let pantalla = caja.translate(origen.to_vec2());
        ui.painter().rect(
            pantalla,
            CornerRadius::same(4),
            con_alfa(p.acento, 0.12),
            Stroke::new(1.0, con_alfa(p.acento, 0.8)),
            StrokeKind::Inside,
        );
        let clip = ui.clip_rect();
        if puntero.y < clip.top() + 28.0 {
            ui.scroll_with_delta(vec2(0.0, 14.0));
        } else if puntero.y > clip.bottom() - 28.0 {
            ui.scroll_with_delta(vec2(0.0, -14.0));
        }
        ui.ctx().request_repaint();
        if suelto {
            self.banda = None;
        }
    }

    /// Flechas, Enter, Retroceso, Escape y escribir para buscar.
    pub(crate) fn teclado_vista(&mut self, ctx: &egui::Context, acciones: &mut Vec<Accion>) {
        let eventos = ctx.input(|i| i.events.clone());
        for ev in eventos {
            match ev {
                Event::Key { key, pressed: true, modifiers, .. } => {
                    if modifiers.ctrl || modifiers.alt || modifiers.command {
                        continue;
                    }
                    let n = self.visibles.len() as isize;
                    let cols = if self.config.vista == Vista::Lista { 1 } else { self.columnas.max(1) } as isize;
                    let pagina = (self.filas_visibles.max(1) as isize) * cols;
                    let paso = match key {
                        Key::ArrowRight if cols > 1 => Some(1),
                        Key::ArrowLeft if cols > 1 => Some(-1),
                        Key::ArrowDown => Some(cols),
                        Key::ArrowUp => Some(-cols),
                        Key::PageDown => Some(pagina),
                        Key::PageUp => Some(-pagina),
                        Key::Home => Some(isize::MIN / 2),
                        Key::End => Some(isize::MAX / 2),
                        _ => None,
                    };
                    if let Some(paso) = paso {
                        if n == 0 {
                            continue;
                        }
                        let actual = self.foco.clone().and_then(|f| self.indice_visible(&f)).map(|i| i as isize);
                        let nuevo = match actual {
                            None => 0,
                            Some(a) => a.saturating_add(paso).clamp(0, n - 1),
                        };
                        let ruta = self.entradas[self.visibles[nuevo as usize]].ruta.clone();
                        if modifiers.shift {
                            if self.ancla.is_none() {
                                self.ancla = self.foco.clone().or(Some(ruta.clone()));
                            }
                            self.seleccionar_rango(&ruta, false);
                        } else {
                            self.seleccionar_solo(&ruta);
                        }
                        self.desplazar_a_foco = true;
                        continue;
                    }
                    match key {
                        Key::Enter => acciones.push(Accion::AbrirSeleccion),
                        Key::Backspace => acciones.push(Accion::Subir),
                        Key::Escape => {
                            if self.busqueda.is_some() {
                                self.busqueda = None;
                                self.visibles_sucios = true;
                            } else {
                                self.seleccion.clear();
                                self.foco = None;
                            }
                        }
                        _ => {}
                    }
                }
                Event::Text(t) => {
                    let m = ctx.input(|i| i.modifiers);
                    if !m.ctrl && !m.alt && !t.trim().is_empty() && self.busqueda.is_none() {
                        self.busqueda = Some(t);
                        self.enfocar_busqueda = true;
                        self.visibles_sucios = true;
                    }
                }
                _ => {}
            }
        }
    }

    /// Lo que se arrastra se ve junto al puntero.
    pub(crate) fn dibujar_arrastre(&self, ctx: &egui::Context) {
        let Some(a) = egui::DragAndDrop::payload::<Arrastre>(ctx) else { return };
        let Some(pos) = ctx.pointer_latest_pos() else { return };
        let p = &self.paleta;
        // Vacío: viene de otra ventana y aún no sabemos qué trae
        let externo = a.0.is_empty();
        let texto = match a.0.len() {
            0 => "Soltar aquí".to_string(),
            1 => crate::tareas::nombre(&a.0[0]),
            n => formato::elementos(n),
        };
        let copiar = ctx.input(|i| i.modifiers.ctrl);
        let pintor = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("arrastre")));
        let g = pintor.layout_no_wrap(
            format!("{}  {texto}", if externo { ph::DOWNLOAD_SIMPLE } else if copiar { ph::COPY } else { ph::ARROW_RIGHT }),
            FontId::proportional(13.0),
            p.texto_acento,
        );
        let caja = Rect::from_min_size(pos + vec2(16.0, 12.0), g.size() + vec2(20.0, 12.0));
        pintor.rect_filled(caja, CornerRadius::same(8), p.acento);
        pintor.galley(caja.min + vec2(10.0, 6.0), g, p.texto_acento);
        ctx.set_cursor_icon(if copiar { CursorIcon::Copy } else { CursorIcon::Grabbing });
    }

    // ------------------------------------------------------------------
    //  Menús del clic derecho
    // ------------------------------------------------------------------

    fn opcion(&self, ui: &mut Ui, glifo: &str, texto: &str, atajo: &str) -> bool {
        let r = ui.add(egui::Button::new(format!("{glifo}   {texto}")).shortcut_text(RichText::new(atajo).small().color(self.paleta.texto_tenue)));
        if r.clicked() {
            ui.close();
            true
        } else {
            false
        }
    }

    /// Programas para el submenú «Abrir con» (se calculan una vez por selección).
    fn programas_menu(&self, archivos: &[PathBuf]) -> Rc<Vec<Programa>> {
        if let Some((rutas, lista)) = self.menu_programas.borrow().as_ref()
            && rutas.as_slice() == archivos
        {
            return lista.clone();
        }
        let lista = Rc::new(nucleo::programas_para(archivos, false).map(|l| l.programas).unwrap_or_default());
        self.menu_programas.replace(Some((archivos.to_vec(), lista.clone())));
        lista
    }

    fn submenu_abrir_con(&self, ui: &mut Ui, archivos: Vec<PathBuf>, acciones: &mut Vec<Accion>) {
        if archivos.is_empty() {
            return;
        }
        ui.menu_button(format!("{}   Abrir con", ph::ARROW_SQUARE_OUT), |ui| {
            ui.set_min_width(240.0);
            let lista = self.programas_menu(&archivos);
            let mut hubo = false;
            for prog in lista.iter().filter(|p| p.afinidad > Afinidad::Otro).take(10) {
                hubo = true;
                let mut texto = prog.nombre.clone();
                if prog.afinidad == Afinidad::Predeterminado {
                    texto.push_str("  (predeterminado)");
                }
                let textura = self.iconos_programas.borrow_mut().textura(ui.ctx(), &prog.icono);
                let boton = match &textura {
                    Some(t) => egui::Button::image_and_text(egui::Image::new(t).fit_to_exact_size(vec2(18.0, 18.0)), texto),
                    None => egui::Button::new(format!("{}   {texto}", dialogos::glifo_programa(prog))),
                };
                let mut r = ui.add(boton);
                if prog.terminal {
                    r = r.on_hover_text("Se abre en la terminal");
                }
                if r.clicked() {
                    acciones.push(Accion::AbrirConPrograma { programa: Box::new(prog.clone()), rutas: archivos.clone() });
                    ui.close();
                }
            }
            if !hubo {
                ui.label(RichText::new("Ningún programa dice abrir esto").color(self.paleta.texto_tenue));
            }
            ui.separator();
            if self.opcion(ui, ph::DOTS_THREE_CIRCLE, "Otro programa…", "") {
                acciones.push(Accion::AbrirCon(archivos.clone()));
            }
        });
    }

    /// «Añadir a Lugares» o, si la carpeta ya está ahí, «Quitar de Lugares».
    fn opcion_lugar(&self, ui: &mut Ui, ruta: &std::path::Path, atajo: &str, acciones: &mut Vec<Accion>) {
        match self.indice_lugar(ruta) {
            Some(i) => {
                if self.opcion(ui, ph::STAR_HALF, "Quitar de Lugares", "") {
                    acciones.push(Accion::QuitarLugar(i));
                }
            }
            None => {
                if self.opcion(ui, ph::STAR, "Añadir a Lugares", atajo) {
                    acciones.push(Accion::AgregarLugar(ruta.to_path_buf()));
                }
            }
        }
    }

    fn opcion_peligro(&self, ui: &mut Ui, glifo: &str, texto: &str, atajo: &str) -> bool {
        let r = ui.add(
            egui::Button::new(RichText::new(format!("{glifo}   {texto}")).color(self.paleta.peligro))
                .shortcut_text(RichText::new(atajo).small().color(self.paleta.texto_tenue)),
        );
        if r.clicked() {
            ui.close();
            true
        } else {
            false
        }
    }

    fn menu_elemento(&self, ui: &mut Ui, ruta: &std::path::Path, es_carpeta: bool, acciones: &mut Vec<Accion>) {
        ui.set_min_width(250.0);
        let sel = {
            let s = self.seleccionados();
            if s.is_empty() { vec![ruta.to_path_buf()] } else { s }
        };
        let n = sel.len();
        let carpetas: Vec<PathBuf> = sel.iter().filter(|r| self.entrada(r).is_some_and(|e| e.es_carpeta())).cloned().collect();
        let todas_carpetas = carpetas.len() == n;
        let a = |id: &str| self.atajos.texto(id);

        if n == 1 {
            if self.opcion(ui, ph::FOLDER_OPEN, "Abrir", "Enter") {
                acciones.push(Accion::Abrir(ruta.to_path_buf()));
            }
            if es_carpeta {
                if self.opcion(ui, ph::APP_WINDOW, "Abrir en ventana nueva", "") {
                    acciones.push(Accion::AbrirEnVentana(ruta.to_path_buf()));
                }
                if self.opcion(ui, ph::TERMINAL_WINDOW, "Abrir terminal aquí", "") {
                    acciones.push(Accion::AbrirTerminal(ruta.to_path_buf()));
                }
            } else {
                self.submenu_abrir_con(ui, sel.clone(), acciones);
            }
        } else {
            if self.opcion(ui, ph::FOLDER_OPEN, &format!("Abrir {}", formato::elementos(n)), "Enter") {
                acciones.push(Accion::AbrirSeleccion);
            }
            if !todas_carpetas {
                self.submenu_abrir_con(ui, sel.iter().filter(|r| !carpetas.contains(r)).cloned().collect(), acciones);
            }
        }
        ui.separator();
        if self.opcion(ui, ph::SCISSORS, "Cortar", &a("cortar")) {
            acciones.push(Accion::Cortar(sel.clone()));
        }
        if self.opcion(ui, ph::COPY, "Copiar", &a("copiar")) {
            acciones.push(Accion::Copiar(sel.clone()));
        }
        if n == 1 && es_carpeta && self.portapapeles.is_some() && self.opcion(ui, ph::CLIPBOARD, "Pegar dentro", "") {
            acciones.push(Accion::Pegar(ruta.to_path_buf()));
        }
        if self.opcion(ui, ph::COPY_SIMPLE, "Duplicar", &a("duplicar")) {
            acciones.push(Accion::Duplicar(sel.clone()));
        }
        if n == 1 && self.opcion(ui, ph::PENCIL_SIMPLE, "Renombrar…", &a("renombrar")) {
            acciones.push(Accion::Renombrar(ruta.to_path_buf()));
        }

        if todas_carpetas {
            ui.separator();
            self.menus_estilo(ui, &carpetas, acciones);
        }

        ui.separator();
        if n == 1 && es_carpeta {
            self.opcion_lugar(ui, ruta, &a("agregar_lugar"), acciones);
        }
        if self.opcion(ui, ph::LINK, if n == 1 { "Copiar ruta" } else { "Copiar rutas" }, "") {
            acciones.push(Accion::CopiarRuta(sel.clone()));
        }
        ui.separator();
        if self.opcion(ui, ph::TRASH, "Mover a la papelera", &a("papelera")) {
            acciones.push(Accion::Papelera(sel.clone()));
        }
        if self.opcion_peligro(ui, ph::X_CIRCLE, "Eliminar para siempre…", &a("eliminar")) {
            acciones.push(Accion::Eliminar(sel.clone()));
        }
        ui.separator();
        if self.opcion(ui, ph::INFO, "Propiedades", &a("propiedades")) {
            acciones.push(Accion::Propiedades(sel));
        }
    }

    /// Submenús "Color de carpeta" y "Logotipo" (y el diálogo completo).
    pub(crate) fn menus_estilo(&self, ui: &mut Ui, carpetas: &[PathBuf], acciones: &mut Vec<Accion>) {
        let p = &self.paleta;
        let actual = carpetas.first().and_then(|r| self.estilo_de(r)).unwrap_or_default();
        ui.menu_button(format!("{}   Color de carpeta", ph::PALETTE), |ui| {
            ui.set_min_width(250.0);
            ui.label(RichText::new("Colores listos").small().color(p.texto_tenue));
            if let Some(c) = ui_util::rejilla_colores(ui, p, actual.color, 7, 11.0) {
                acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::Color(Some(c))));
                ui.close();
            }
            ui.separator();
            if self.opcion(ui, ph::EYEDROPPER, "Color personalizado…", "") {
                acciones.push(Accion::Personalizar(carpetas.to_vec()));
            }
            if actual.color.is_some() && self.opcion(ui, ph::ARROW_COUNTER_CLOCKWISE, "Usar el color del tema", "") {
                acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::Color(None)));
            }
        });
        ui.menu_button(format!("{}   Logotipo", ph::SMILEY), |ui| {
            ui.set_min_width(300.0);
            ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                if let Some(id) = ui_util::rejilla_logotipos(ui, p, actual.icono.as_deref(), "", 8, 32.0) {
                    acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::Logotipo(Some(id.to_string()))));
                    ui.close();
                }
            });
            ui.separator();
            ui.menu_button(format!("{}   Color del logotipo", ph::DROP), |ui| {
                ui.set_min_width(250.0);
                if let Some(c) = ui_util::rejilla_colores(ui, p, actual.color_icono, 7, 11.0) {
                    acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::ColorLogotipo(Some(c))));
                    ui.close();
                }
                ui.separator();
                if self.opcion(ui, ph::MAGIC_WAND, "Automático (según la carpeta)", "") {
                    acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::ColorLogotipo(None)));
                }
            });
            if self.opcion(ui, ph::TEXT_AA, "Texto o emoji…", "") {
                acciones.push(Accion::Personalizar(carpetas.to_vec()));
            }
            if actual.icono.is_some() && self.opcion(ui, ph::PROHIBIT, "Quitar logotipo", "") {
                acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::Logotipo(None)));
            }
        });
        if self.opcion(ui, ph::SPARKLE, "Personalizar carpeta…", &self.atajos.texto("personalizar")) {
            acciones.push(Accion::Personalizar(carpetas.to_vec()));
        }
        let alguna = carpetas.iter().any(|r| self.estilo_de(r).is_some_and(|e| !e.vacio()));
        if alguna && self.opcion(ui, ph::ARROW_COUNTER_CLOCKWISE, "Quitar personalización", "") {
            acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::Quitar));
        }
    }

    fn menu_fondo(&self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        ui.set_min_width(250.0);
        let a = |id: &str| self.atajos.texto(id);
        if self.ubicacion == Ubicacion::Papelera {
            if self.opcion(ui, ph::SELECTION_ALL, "Seleccionar todo", &a("seleccionar_todo")) {
                acciones.push(Accion::SeleccionarTodo);
            }
            if !self.entradas.is_empty() && self.opcion_peligro(ui, ph::BROOM, "Vaciar papelera…", "") {
                acciones.push(Accion::VaciarPapelera);
            }
            return;
        }
        let Some(dir) = self.ubicacion.ruta().map(PathBuf::from) else { return };
        if self.opcion(ui, ph::FOLDER_PLUS, "Nueva carpeta…", &a("nueva_carpeta")) {
            acciones.push(Accion::NuevaCarpeta);
        }
        if self.opcion(ui, ph::FILE_PLUS, "Nuevo archivo…", &a("nuevo_archivo")) {
            acciones.push(Accion::NuevoArchivo);
        }
        if self.portapapeles.is_some() && self.opcion(ui, ph::CLIPBOARD, "Pegar", &a("pegar")) {
            acciones.push(Accion::Pegar(dir.clone()));
        }
        ui.separator();
        if self.opcion(ui, ph::SELECTION_ALL, "Seleccionar todo", &a("seleccionar_todo")) {
            acciones.push(Accion::SeleccionarTodo);
        }
        ui.menu_button(format!("{}   Ordenar por", ph::SORT_ASCENDING), |ui| {
            for o in Orden::TODOS {
                let marca = if self.config.orden == o { ph::CHECK } else { " " };
                if self.opcion(ui, marca, o.nombre(), "") {
                    acciones.push(Accion::Ordenar(o));
                }
            }
            ui.separator();
            let desc = self.config.orden_descendente;
            if self.opcion(ui, if desc { ph::CHECK } else { " " }, "Al revés", "") {
                acciones.push(Accion::OrdenDescendente(!desc));
            }
            let cp = self.config.carpetas_primero;
            if self.opcion(ui, if cp { ph::CHECK } else { " " }, "Carpetas primero", "") {
                acciones.push(Accion::CarpetasPrimero(!cp));
            }
        });
        ui.menu_button(format!("{}   Vista", ph::SQUARES_FOUR), |ui| {
            let v = self.config.vista;
            if self.opcion(ui, if v == Vista::Cuadricula { ph::CHECK } else { " " }, "Cuadrícula", &a("vista_cuadricula")) {
                acciones.push(Accion::CambiarVista(Vista::Cuadricula));
            }
            if self.opcion(ui, if v == Vista::Lista { ph::CHECK } else { " " }, "Lista", &a("vista_lista")) {
                acciones.push(Accion::CambiarVista(Vista::Lista));
            }
        });
        let ocultos = self.config.mostrar_ocultos;
        if self.opcion(ui, if ocultos { ph::EYE } else { ph::EYE_SLASH }, if ocultos { "Esconder ocultos" } else { "Mostrar ocultos" }, &a("ocultos")) {
            acciones.push(Accion::AlternarOcultos);
        }
        ui.separator();
        self.menus_estilo(ui, std::slice::from_ref(&dir), acciones);
        ui.separator();
        if self.opcion(ui, ph::TERMINAL_WINDOW, "Abrir terminal aquí", &a("terminal")) {
            acciones.push(Accion::AbrirTerminal(dir.clone()));
        }
        self.opcion_lugar(ui, &dir, &a("agregar_lugar"), acciones);
        if self.opcion(ui, ph::INFO, "Propiedades de esta carpeta", "") {
            acciones.push(Accion::Propiedades(vec![dir]));
        }
    }

    fn menu_papelera(&self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        ui.set_min_width(230.0);
        let sel = self.seleccionados();
        if sel.is_empty() {
            return;
        }
        if self.opcion(ui, ph::ARROW_COUNTER_CLOCKWISE, "Restaurar", "") {
            acciones.push(Accion::Restaurar(sel.clone()));
        }
        if self.opcion_peligro(ui, ph::X_CIRCLE, "Eliminar para siempre…", &self.atajos.texto("eliminar")) {
            acciones.push(Accion::BorrarDePapelera(sel));
        }
    }
}

/// Muestra el ícono de una entrada como widget (para diálogos).
pub fn icono_entrada(app: &Solar, ui: &mut Ui, e: &Entrada, lado: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(lado, lado), Sense::hover());
    app.pintar_icono(ui.painter(), rect, e, 1.0);
}

#[allow(dead_code)]
fn _usar(_: &nucleo::Vista) {}
