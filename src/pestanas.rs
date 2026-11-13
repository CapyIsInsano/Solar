//! Pestañas: varias carpetas abiertas en la misma ventana, cada una con su
//! historial, su selección y su vista. Solo la pestaña que se ve está «viva»
//! (sus archivos están en `Solar`); las demás guardan lo que tenían y, si la
//! carpeta no es enorme, también su lista de archivos para volver al
//! instante (y se refresca por detrás).
//!
//! La barra de pestañas va arriba de todo, como en Windows 11: a la
//! izquierda el espacio de trabajo, luego las pestañas, el botón «+» y, si
//! Solar dibuja su propia barra de título, los botones de la ventana.

use crate::app::{Accion, Arrastre, Solar, Ubicacion};
use crate::iconos::{self, FUENTE_RELLENA};
use crate::nucleo::{self, Entrada, Espacio, Vista};
use crate::tareas::{self, ComandoVigia};
use crate::tema::{self, Paleta, con_alfa};
use crate::ui_util::boton_icono;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontFamily, FontId, Frame, Id, Margin, Rect, RichText, Sense, Stroke,
    Ui, ViewportCommand, pos2, vec2,
};
use egui_phosphor::regular as ph;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const ALTO_PESTANAS: f32 = 40.0;
/// Carpetas más grandes que esto no se guardan al cambiar de pestaña
/// (se vuelven a leer al regresar).
const GUARDAR_HASTA: usize = 5000;

/// Lo que una pestaña que no se ve recuerda de su carpeta.
pub struct Listado {
    entradas: Vec<Arc<Entrada>>,
    indice: HashMap<PathBuf, usize>,
    espacio: Option<Espacio>,
    error: Option<String>,
    originales_papelera: Vec<PathBuf>,
}

pub struct Guardado {
    seleccion: HashSet<PathBuf>,
    foco: Option<PathBuf>,
    ancla: Option<PathBuf>,
    busqueda: Option<String>,
    listado: Option<Listado>,
}

pub struct Pestana {
    pub id: u64,
    pub vista: Vista,
    /// Solo están al día si no es la pestaña activa (la activa vive en Solar)
    pub ubicacion: Ubicacion,
    pub historial: Vec<Ubicacion>,
    pub pos_historial: usize,
    pub guardado: Option<Box<Guardado>>,
    pub creada: Instant,
}

impl Pestana {
    pub fn nueva(id: u64, ubicacion: Ubicacion, vista: Vista) -> Self {
        Self {
            id,
            vista,
            historial: vec![ubicacion.clone()],
            ubicacion,
            pos_historial: 0,
            guardado: None,
            creada: Instant::now(),
        }
    }
}

/// Para volver a abrir una pestaña cerrada (Ctrl+Shift+T).
pub struct Cerrada {
    ubicacion: Ubicacion,
    vista: Vista,
    posicion: usize,
}

/// Arrastrar una pestaña para cambiarla de lugar.
#[derive(Clone, Copy)]
pub struct ArrastrePestana(pub u64);

/// Nombre e ícono de una ubicación (para la pestaña).
pub fn nombre_y_glifo(app: &Solar, u: &Ubicacion) -> (String, &'static str, Color32) {
    match u {
        Ubicacion::Papelera => ("Papelera".into(), ph::TRASH, app.paleta.texto_tenue),
        Ubicacion::Ruta(r) => {
            if *r == nucleo::dir_hogar() {
                return ("Carpeta personal".into(), ph::HOUSE, app.paleta.carpeta);
            }
            if r.as_os_str() == "/" {
                return ("Sistema".into(), ph::HARD_DRIVES, app.paleta.texto_tenue);
            }
            if let Some(d) = app.dispositivos.iter().find(|d| d.punto_montaje.as_deref() == Some(r.as_path())) {
                return (d.nombre.clone(), if d.extraible { ph::USB } else { ph::HARD_DRIVE }, app.paleta.acento);
            }
            let estilo = app.estilo_de(r).unwrap_or_default();
            let glifo = estilo.icono.as_deref().and_then(iconos::emblema).map(|e| e.glifo).unwrap_or(ph::FOLDER);
            (tareas::nombre(r), glifo, estilo.color.unwrap_or(app.paleta.carpeta))
        }
    }
}

impl Solar {
    /// La vista de la pestaña que se ve.
    pub fn vista(&self) -> Vista {
        self.pestanas.get(self.activa).map(|p| p.vista).unwrap_or(self.config.vista)
    }

    pub fn id_pestana(&self) -> u64 {
        self.pestanas.get(self.activa).map(|p| p.id).unwrap_or(0)
    }

    pub fn ubicacion_de(&self, i: usize) -> &Ubicacion {
        if i == self.activa { &self.ubicacion } else { &self.pestanas[i].ubicacion }
    }

    pub(crate) fn nuevo_id_pestana(&mut self) -> u64 {
        self.siguiente_pestana += 1;
        self.siguiente_pestana
    }

    /// Guarda en la pestaña activa lo que hoy vive en Solar (antes de irse a otra).
    fn guardar_activa(&mut self) {
        let entradas = std::mem::take(&mut self.entradas);
        let indice = std::mem::take(&mut self.indice);
        let listado = if entradas.len() <= GUARDAR_HASTA && self.cargando_desde.is_none() {
            Some(Listado {
                entradas,
                indice,
                espacio: self.espacio.take(),
                error: self.error_listado.take(),
                originales_papelera: std::mem::take(&mut self.originales_papelera),
            })
        } else {
            crate::memoria::soltar_aparte((entradas, indice), true);
            None
        };
        let guardado = Guardado {
            seleccion: self.seleccion.tomar(),
            foco: self.foco.take(),
            ancla: self.ancla.take(),
            busqueda: self.busqueda.take(),
            listado,
        };
        let p = &mut self.pestanas[self.activa];
        p.ubicacion = self.ubicacion.clone();
        p.historial = self.historial.clone();
        p.pos_historial = self.pos_historial;
        p.guardado = Some(Box::new(guardado));
        self.visibles.clear();
    }

    /// Suelta lo que tiene la pestaña activa (porque se cierra).
    fn soltar_activa(&mut self) {
        let entradas = std::mem::take(&mut self.entradas);
        let grande = entradas.len() > GUARDAR_HASTA;
        crate::memoria::soltar_aparte((entradas, std::mem::take(&mut self.indice)), grande);
        self.visibles.clear();
    }

    /// Pone en Solar lo de la pestaña `i`.
    pub(crate) fn cargar_pestana(&mut self, ctx: &egui::Context, i: usize) {
        self.activa = i;
        let p = &mut self.pestanas[i];
        self.ubicacion = p.ubicacion.clone();
        self.historial = std::mem::take(&mut p.historial);
        if self.historial.is_empty() {
            self.historial.push(self.ubicacion.clone());
        }
        self.pos_historial = p.pos_historial.min(self.historial.len() - 1);
        let guardado = p.guardado.take();
        self.banda = None;
        self.editando_ruta = None;
        self.seleccionar_al_cargar = None;
        self.generacion += 1;
        self.visibles.clear();
        self.visibles_sucios = true;
        self.disposicion = None;
        let mut con_listado = false;
        if let Some(g) = guardado {
            self.seleccion.reemplazar(g.seleccion);
            self.foco = g.foco;
            self.ancla = g.ancla;
            self.busqueda = g.busqueda;
            if let Some(l) = g.listado {
                self.entradas = l.entradas;
                self.indice = l.indice;
                self.espacio = l.espacio;
                self.error_listado = l.error;
                self.originales_papelera = l.originales_papelera;
                con_listado = true;
            }
        } else {
            self.seleccion.clear();
            self.foco = None;
            self.ancla = None;
            self.busqueda = None;
        }
        if con_listado {
            // Se ve al instante y se refresca por detrás
            self.desplazar_a_foco = self.foco.is_some();
            self.cargando_desde = None;
            self.vigia.enviar(ComandoVigia::Observar(self.ubicacion.ruta().map(|r| r.to_path_buf())));
            self.cargar(ctx, false);
        } else {
            self.entradas.clear();
            self.indice.clear();
            self.animar_carga = true;
            self.cargar(ctx, true);
        }
        self.marcar_sesion();
    }

    pub fn activar_pestana(&mut self, ctx: &egui::Context, i: usize) {
        if i == self.activa || i >= self.pestanas.len() {
            return;
        }
        self.guardar_activa();
        self.cargar_pestana(ctx, i);
    }

    /// Abre `ubicacion` en una pestaña nueva (al lado de la activa).
    pub fn abrir_pestana(&mut self, ctx: &egui::Context, ubicacion: Ubicacion, activar: bool) {
        if let Ubicacion::Ruta(r) = &ubicacion
            && !r.is_dir()
        {
            self.avisar(format!("No se puede abrir «{}»: no existe o no es una carpeta.", r.display()), crate::app::TipoAviso::Error);
            return;
        }
        let id = self.nuevo_id_pestana();
        let pestana = Pestana::nueva(id, ubicacion, self.vista());
        let pos = (self.activa + 1).min(self.pestanas.len());
        // Varias seguidas (clic con la rueda) van una tras otra, como en el navegador
        let pos = if activar { pos } else { self.posicion_siguiente.filter(|&p| p <= self.pestanas.len()).unwrap_or(pos) };
        self.pestanas.insert(pos, pestana);
        if !activar {
            self.posicion_siguiente = Some(pos + 1);
        }
        if pos <= self.activa {
            self.activa += 1;
        }
        if activar {
            self.activar_pestana(ctx, pos);
        }
        self.marcar_sesion();
    }

    pub fn cerrar_pestana(&mut self, ctx: &egui::Context, i: usize) {
        if i >= self.pestanas.len() {
            return;
        }
        if self.pestanas.len() == 1 {
            // Como en Windows: cerrar la última pestaña cierra la ventana
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }
        self.cerradas.push(Cerrada { ubicacion: self.ubicacion_de(i).clone(), vista: self.pestanas[i].vista, posicion: i });
        if self.cerradas.len() > 25 {
            self.cerradas.remove(0);
        }
        self.posicion_siguiente = None;
        if i == self.activa {
            self.soltar_activa();
            let vecina = if i + 1 < self.pestanas.len() { i + 1 } else { i - 1 };
            self.pestanas.remove(i);
            let vecina = if vecina > i { vecina - 1 } else { vecina };
            self.cargar_pestana(ctx, vecina);
        } else {
            let p = self.pestanas.remove(i);
            if let Some(l) = p.guardado.and_then(|g| g.listado) {
                crate::memoria::soltar_aparte((l.entradas, l.indice), false);
            }
            if i < self.activa {
                self.activa -= 1;
            }
        }
        crate::sonido::pedir(crate::sonido::Sonido::Retraccion);
        self.marcar_sesion();
    }

    pub fn reabrir_pestana(&mut self, ctx: &egui::Context) {
        let Some(c) = self.cerradas.pop() else {
            self.avisar("No hay pestañas cerradas para reabrir.", crate::app::TipoAviso::Info);
            return;
        };
        let id = self.nuevo_id_pestana();
        let pos = c.posicion.min(self.pestanas.len());
        self.pestanas.insert(pos, Pestana::nueva(id, c.ubicacion, c.vista));
        if pos <= self.activa {
            self.activa += 1;
        }
        self.activar_pestana(ctx, pos);
    }

    pub fn mover_pestana(&mut self, de: usize, a: usize) {
        if de >= self.pestanas.len() || a >= self.pestanas.len() || de == a {
            return;
        }
        let activa = self.pestanas[self.activa].id;
        let p = self.pestanas.remove(de);
        self.pestanas.insert(a, p);
        self.activa = self.pestanas.iter().position(|p| p.id == activa).unwrap_or(0);
        self.marcar_sesion();
    }

    pub fn indice_pestana(&self, id: u64) -> Option<usize> {
        self.pestanas.iter().position(|p| p.id == id)
    }

    /// Cierra varias pestañas de un jalón (las demás, las de la derecha).
    pub fn cerrar_pestanas(&mut self, ctx: &egui::Context, conservar: impl Fn(usize) -> bool) {
        let ids: Vec<u64> = self.pestanas.iter().enumerate().filter(|(i, _)| !conservar(*i)).map(|(_, p)| p.id).collect();
        for id in ids {
            if self.pestanas.len() > 1
                && let Some(i) = self.indice_pestana(id)
            {
                self.cerrar_pestana(ctx, i);
            }
        }
    }

    // ------------------------------------------------------------------
    //  La barra de pestañas
    // ------------------------------------------------------------------

    pub(crate) fn barra_pestanas(&mut self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        let p = self.paleta.clone();
        let integrada = self.config.barra_titulo_integrada;
        let tira = tema::mezclar(p.fondo, p.superficie, 0.25);
        egui::Panel::top("pestanas")
            .exact_size(ALTO_PESTANAS)
            .frame(Frame::new().fill(tira).inner_margin(Margin { left: 8, right: 8, top: 0, bottom: 0 }))
            .show_separator_line(false)
            .show(ui, |ui| {
                let rect = ui.max_rect();
                // Arrastrar la ventana desde los huecos
                let fondo = ui.interact(rect, Id::new("arrastrar_ventana_pestanas"), Sense::click_and_drag());
                if integrada {
                    if fondo.drag_started_by(egui::PointerButton::Primary) {
                        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                    }
                    if fondo.double_clicked() {
                        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!self.maximizada));
                    }
                }
                if fondo.middle_clicked() {
                    acciones.push(Accion::NuevaPestana);
                }
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    self.chip_espacio(ui, &p, acciones);
                    ui.add_space(4.0);
                    let ancho_ventana = if integrada { 3.0 * 36.0 + 8.0 } else { 0.0 };
                    let ancho_tabs = (ui.available_width() - ancho_ventana - 40.0).max(80.0);
                    self.pestanas_ui(ui, &p, ancho_tabs, acciones);
                    let mas = boton_icono(ui, &p, ph::PLUS, &format!("Pestaña nueva ({})", self.atajos.texto("nueva_pestana")), true);
                    if mas.clicked() {
                        acciones.push(Accion::NuevaPestana);
                    }
                    if integrada {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            self.botones_ventana(ui, &p, acciones);
                        });
                    }
                });
            });
    }

    /// Minimizar, maximizar y cerrar (cuando Solar dibuja su barra de título).
    pub(crate) fn botones_ventana(&self, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        let cerrar = boton_icono(ui, p, ph::X, "Cerrar", true);
        if cerrar.hovered() {
            ui.painter().rect_filled(cerrar.rect, CornerRadius::same(8), con_alfa(p.peligro, 0.85));
            ui.painter().text(cerrar.rect.center(), Align2::CENTER_CENTER, ph::X, FontId::proportional(17.0), Color32::WHITE);
        }
        if cerrar.clicked() {
            acciones.push(Accion::Cerrar);
        }
        let (glifo, ayuda) = if self.maximizada { (ph::CORNERS_IN, "Restaurar") } else { (ph::CORNERS_OUT, "Maximizar") };
        if boton_icono(ui, p, glifo, ayuda, true).clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!self.maximizada));
        }
        if boton_icono(ui, p, ph::MINUS, "Minimizar", true).clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
        }
    }

    fn pestanas_ui(&mut self, ui: &mut Ui, p: &Paleta, ancho_total: f32, acciones: &mut Vec<Accion>) {
        let n = self.pestanas.len();
        let ancho = (ancho_total / n as f32).clamp(72.0, 220.0);
        let alto = ALTO_PESTANAS - 8.0;
        let desplazar = ancho * n as f32 > ancho_total + 1.0;
        let mut dibujar = |ui: &mut Ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for i in 0..n {
                    // La pestaña recién abierta crece de 0 a su ancho
                    let t = self.progreso_animacion(self.pestanas[i].creada, 0.18);
                    if t < 1.0 {
                        ui.ctx().request_repaint();
                    }
                    self.pestana(ui, p, i, vec2(ancho * t, alto), acciones);
                }
            });
        };
        if desplazar {
            egui::ScrollArea::horizontal()
                .id_salt("pestanas")
                .max_width(ancho_total)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| dibujar(ui));
        } else {
            dibujar(ui);
        }
    }

    fn pestana(&mut self, ui: &mut Ui, p: &Paleta, i: usize, tam: egui::Vec2, acciones: &mut Vec<Accion>) {
        let id = self.pestanas[i].id;
        let activa = i == self.activa;
        let (rect, resp) = ui.allocate_exact_size(tam, Sense::click_and_drag());
        let rect = rect.translate(vec2(0.0, 4.0));
        if tam.x < 24.0 {
            return;
        }
        let ubicacion = self.ubicacion_de(i).clone();
        let (nombre, glifo, color) = nombre_y_glifo(self, &ubicacion);
        let ctx = ui.ctx().clone();
        let h = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered() && !activa, 0.12);
        let radio = CornerRadius { nw: 9, ne: 9, sw: 0, se: 0 };
        let painter = ui.painter();

        // Archivos arrastrados encima: la pestaña se abre sola tras un momento
        let con_archivos = resp.dnd_hover_payload::<Arrastre>().is_some();
        let pestana_encima = resp.dnd_hover_payload::<ArrastrePestana>().filter(|a| a.0 != id).is_some();
        if activa {
            painter.rect_filled(rect, radio, p.superficie);
            // Une la pestaña con la barra de abajo
            painter.rect_filled(Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), pos2(rect.right(), rect.bottom() + 1.0)), 0.0, p.superficie);
        } else if con_archivos {
            painter.rect(rect, radio, con_alfa(p.acento, 0.16), Stroke::new(1.5, p.acento), egui::StrokeKind::Inside);
        } else if h > 0.0 {
            painter.rect_filled(rect.shrink2(vec2(2.0, 0.0)), radio, con_alfa(p.texto, 0.07 * h));
        }
        // Separador entre pestañas que no se tocan con la activa
        if !activa && i + 1 != self.activa && i + 1 < self.pestanas.len() && h < 0.5 {
            painter.line_segment(
                [pos2(rect.right() - 0.5, rect.top() + 9.0), pos2(rect.right() - 0.5, rect.bottom() - 9.0)],
                Stroke::new(1.0, con_alfa(p.borde, 0.9)),
            );
        }
        if pestana_encima {
            painter.line_segment([rect.left_top(), rect.left_bottom()], Stroke::new(2.5, p.acento));
        }
        let mut x = rect.left() + 12.0;
        if tam.x >= 60.0 || activa {
            painter.text(
                pos2(x + 8.0, rect.center().y),
                Align2::CENTER_CENTER,
                glifo,
                FontId::new(15.0, FontFamily::Name(FUENTE_RELLENA.into())),
                color,
            );
            x += 22.0;
        }
        let ver_cerrar = activa || resp.hovered() || h > 0.0;
        let derecha = rect.right() - if ver_cerrar { 30.0 } else { 10.0 };
        let color_texto = if activa { p.texto } else { p.texto_tenue };
        if derecha > x + 8.0 {
            let g = crate::vista::texto_ajustado(painter, &nombre, derecha - x, 1, 13.0, color_texto, false);
            painter.galley(pos2(x, rect.center().y - g.size().y / 2.0), g, color_texto);
        }
        let mut cerrar_clic = false;
        if ver_cerrar && tam.x >= 48.0 {
            let caja = Rect::from_center_size(pos2(rect.right() - 17.0, rect.center().y), vec2(20.0, 20.0));
            let r = ui.interact(caja, Id::new(("cerrar_pestana", id)), Sense::click());
            if r.hovered() {
                ui.painter().rect_filled(caja, CornerRadius::same(5), con_alfa(p.texto, 0.12));
            }
            ui.painter().text(caja.center(), Align2::CENTER_CENTER, ph::X, FontId::proportional(13.0), color_texto);
            if r.on_hover_text(format!("Cerrar pestaña ({})", self.atajos.texto("cerrar_pestana"))).clicked() {
                cerrar_clic = true;
                acciones.push(Accion::CerrarPestana(id));
            }
        }

        let resp = resp.on_hover_text(match &ubicacion {
            Ubicacion::Ruta(r) => r.display().to_string(),
            Ubicacion::Papelera => "Papelera".into(),
        });
        if resp.clicked() && !cerrar_clic {
            acciones.push(Accion::ActivarPestana(id));
        }
        if resp.middle_clicked() {
            acciones.push(Accion::CerrarPestana(id));
        }
        if resp.drag_started_by(egui::PointerButton::Primary) {
            resp.dnd_set_drag_payload(ArrastrePestana(id));
        }
        if resp.dragged() {
            ctx.set_cursor_icon(CursorIcon::Grabbing);
        }
        if let Some(a) = crate::ui_util::soltado::<ArrastrePestana>(&resp)
            && a.0 != id
            && let Some(de) = self.indice_pestana(a.0)
        {
            acciones.push(Accion::MoverPestana { de, a: i });
        }
        // Archivos: esperar un poquito encima abre la pestaña; soltar los mueve ahí
        if con_archivos && !activa {
            match self.espera_pestana {
                Some((otra, desde)) if otra == id => {
                    if desde.elapsed() > Duration::from_millis(650) {
                        acciones.push(Accion::ActivarPestana(id));
                        self.espera_pestana = None;
                    } else {
                        ctx.request_repaint_after(Duration::from_millis(60));
                    }
                }
                _ => self.espera_pestana = Some((id, Instant::now())),
            }
        }
        if let Some(a) = crate::ui_util::soltado::<Arrastre>(&resp) {
            match &ubicacion {
                Ubicacion::Ruta(destino) => {
                    let copiar = ui.input(|i| i.modifiers.ctrl);
                    acciones.push(Accion::Soltar { destino: destino.clone(), rutas: a.0.clone(), copiar });
                }
                Ubicacion::Papelera => acciones.push(Accion::Papelera(a.0.clone())),
            }
        }
        let n = self.pestanas.len();
        resp.context_menu(|ui| {
            ui.set_min_width(230.0);
            let a = |id: &str| self.atajos.texto(id);
            if self.opcion(ui, ph::PLUS, "Pestaña nueva", &a("nueva_pestana")) {
                acciones.push(Accion::NuevaPestana);
            }
            if self.opcion(ui, ph::COPY_SIMPLE, "Duplicar pestaña", "") {
                acciones.push(Accion::DuplicarPestana(id));
            }
            if let Ubicacion::Ruta(r) = &ubicacion
                && self.opcion(ui, ph::APP_WINDOW, "Abrir en ventana nueva", "")
            {
                acciones.push(Accion::AbrirEnVentana(r.clone()));
            }
            ui.separator();
            if self.opcion(ui, ph::X, "Cerrar pestaña", &a("cerrar_pestana")) {
                acciones.push(Accion::CerrarPestana(id));
            }
            if n > 1 && self.opcion(ui, ph::X_SQUARE, "Cerrar las demás", "") {
                acciones.push(Accion::CerrarOtras(id));
            }
            if i + 1 < n && self.opcion(ui, ph::ARROW_LINE_RIGHT, "Cerrar las de la derecha", "") {
                acciones.push(Accion::CerrarDerecha(id));
            }
            if !self.cerradas.is_empty() && self.opcion(ui, ph::ARROW_COUNTER_CLOCKWISE, "Reabrir pestaña cerrada", &a("reabrir_pestana")) {
                acciones.push(Accion::ReabrirPestana);
            }
            let otros: Vec<(String, String)> = self
                .espacios
                .iter()
                .filter(|e| e.id != self.espacio_trabajo.datos.id)
                .map(|e| (e.id.clone(), e.nombre.clone()))
                .collect();
            if !otros.is_empty() {
                ui.separator();
                ui.menu_button(format!("{}   Mandar a otro espacio", ph::PLANET), |ui| {
                    for (eid, nombre) in otros {
                        if ui.button(RichText::new(nombre)).clicked() {
                            acciones.push(Accion::PestanaAEspacio { pestana: id, espacio: eid });
                            ui.close();
                        }
                    }
                });
            }
        });
    }
}
