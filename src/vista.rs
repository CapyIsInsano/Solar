//! Las vistas de archivos (cuadrícula, lista, mosaicos y órbita) comparten
//! todo lo que no es acomodar y pintar: los clics y la selección (Ctrl,
//! Shift, banda), arrastrar y soltar, los menús, el teclado, la búsqueda y
//! las animaciones. Cada vista solo dice *dónde* va cada archivo y *cómo* se
//! ve (el trait [`Disposicion`]). Así se puede cambiar de vista sin perder la
//! selección, el foco ni la carpeta, y agregar vistas nuevas es poco código.

mod cuadricula;
mod lista;
pub(crate) mod menus;
mod mosaicos;
pub(crate) mod orbita;

use crate::app::{Accion, Arrastre, Banda, ModoPortapapeles, Solar, Ubicacion};
use crate::iconos::{self, Logotipo};
use crate::nucleo::{Entrada, TipoEntrada, Vista};
use crate::tema::{Paleta, con_alfa};
use crate::formato;
use eframe::egui::{
    self, Align, Align2, CornerRadius, CursorIcon, Event, FontId, Frame, Galley, Key, Margin, Painter, PointerButton, Pos2,
    Rect, Response, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, text::LayoutJob, vec2,
};
use egui_phosphor::regular as ph;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) use orbita::{CambioOrbita, EstadoOrbita};

/// Cómo está un elemento en este momento (para pintarlo).
pub(crate) struct Visual {
    pub seleccionado: bool,
    /// Se puede soltar lo que arrastras aquí (es una carpeta)
    pub soltar_aqui: bool,
    /// 0 → 1 con el mouse encima (animado)
    pub hover: f32,
    /// 0 → 1 mientras se presiona
    pub presion: f32,
    /// 0 → 1 → 0 cuando se le cambia el color o el logotipo
    pub salto: f32,
    /// Más transparente si está cortado u oculto
    pub alfa: f32,
    /// Posición entre los visibles (para alternar el fondo de las filas)
    pub indice: usize,
}

/// Lo único que cambia de una vista a otra.
pub(crate) trait Disposicion {
    /// ¿Se desplaza con la rueda? (la Órbita no: se acerca y se aleja)
    fn desplazable(&self) -> bool {
        true
    }
    /// Tamaño de todo el contenido.
    fn tamano(&self) -> Vec2;
    /// Dónde va el elemento `k` (relativo al origen del contenido).
    fn rect(&self, k: usize) -> Rect;
    /// Los elementos que pueden caer en esta parte visible (en coordenadas
    /// del contenido); los que no se ven ni se tocan.
    fn rango(&self, visible: Rect) -> Range<usize>;
    /// A quién se pasa con una tecla de flecha, Inicio, Fin, RePág o AvPág.
    fn vecino(&self, actual: Option<usize>, tecla: Key) -> Option<usize>;
    /// La parte que responde al mouse.
    fn zona(&self, rect: Rect) -> Rect {
        rect.shrink(4.0)
    }
    /// La caja que cuenta para la selección con banda.
    fn caja_banda(&self, k: usize) -> Rect {
        self.rect(k).shrink(8.0)
    }
    /// Dónde se ve el elemento mientras aparece (t de 0 a 1 al abrir la carpeta).
    fn al_aparecer(&self, rect: Rect, t: f32) -> Rect {
        rect.translate(vec2(0.0, (1.0 - t) * 12.0))
    }
    /// Pinta un elemento.
    fn pintar(&self, app: &Solar, painter: &Painter, e: &Entrada, rect: Rect, v: &Visual);
    /// Algo arriba de la lista (los títulos de las columnas).
    fn encabezado(&self, _app: &Solar, _ui: &mut Ui, _p: &Paleta, _acciones: &mut Vec<Accion>) {}
    /// Lo que va detrás de los elementos (el sol y las órbitas).
    fn decorar(&self, _app: &Solar, _ui: &mut Ui, _area: Rect, _acciones: &mut Vec<Accion>) {}
}

/// Íconos y nombres de las vistas (para menús y botones).
pub(crate) fn info_vista(v: Vista) -> (&'static str, &'static str, &'static str) {
    match v {
        Vista::Cuadricula => (ph::SQUARES_FOUR, "Cuadrícula", "vista_cuadricula"),
        Vista::Lista => (ph::LIST_DASHES, "Lista (detalles)", "vista_lista"),
        Vista::Mosaicos => (ph::SQUARE_SPLIT_HORIZONTAL, "Mosaicos", "vista_mosaicos"),
        Vista::Orbita => (ph::PLANET, "Órbita", "vista_orbita"),
    }
}

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

pub(crate) fn texto_ajustado(
    painter: &Painter,
    texto: &str,
    ancho: f32,
    filas: usize,
    tam: f32,
    color: egui::Color32,
    centrado: bool,
) -> Arc<Galley> {
    let mut job = LayoutJob::simple(texto.to_string(), FontId::proportional(tam), color, ancho.max(10.0));
    job.wrap.max_rows = filas;
    job.wrap.break_anywhere = false;
    job.wrap.overflow_character = Some('…');
    if centrado {
        job.halign = Align::Center;
    }
    painter.layout_job(job)
}

pub(crate) fn pintar_galley_centrado(painter: &Painter, centro_x: f32, y: f32, galley: Arc<Galley>, color: egui::Color32) {
    let x = centro_x - galley.rect.center().x;
    painter.galley(pos2(x, y), galley, color);
}

impl Solar {
    fn item(&self, indice_visible: usize) -> Item {
        let e = &self.entradas[self.visibles[indice_visible]];
        Item { entrada: e.clone() }
    }

    pub(crate) fn cortado(&self, ruta: &PathBuf) -> bool {
        self.portapapeles.as_ref().is_some_and(|p| p.modo == ModoPortapapeles::Cortar && p.rutas.contains(ruta))
    }

    /// 0 → 1 mientras un elemento aparece al abrir la carpeta (en cascada).
    fn aparicion(&self, orden: usize) -> f32 {
        let Some(desde) = self.carga_animada else { return 1.0 };
        if !self.config.animaciones {
            return 1.0;
        }
        let t = (desde.elapsed().as_secs_f32() - orden.min(30) as f32 * 0.016) / 0.22;
        crate::app::ease(t).max(0.0)
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
    pub(crate) fn pintar_icono(&self, painter: &Painter, rect: Rect, e: &Entrada, alfa: f32) {
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
    //  Panel central: lo mismo para todas las vistas
    // ------------------------------------------------------------------

    pub(crate) fn vista_central(&mut self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        let p = self.paleta.clone();
        egui::CentralPanel::default().frame(Frame::new().fill(p.fondo)).show(ui, |ui| {
            if self.ubicacion == Ubicacion::Papelera {
                self.aviso_papelera(ui, &p, acciones);
            }
            let vista = self.vista();
            if vista == Vista::Orbita {
                let area = ui.available_rect_before_wrap();
                ui.allocate_rect(area, Sense::hover());
                let orbita = self.preparar_orbita(ui, area, acciones);
                let en_contenido = Rect::from_min_size(Pos2::ZERO, area.size());
                self.contenido(ui, &p, &orbita, area, area.min, en_contenido, acciones);
                self.superposicion_orbita(ui, &p, area, acciones);
                self.disposicion = Some(Box::new(orbita));
                return;
            }
            let ancho = ui.available_width();
            let alto = ui.available_height();
            let d: Box<dyn Disposicion> = match vista {
                Vista::Lista => Box::new(lista::Lista::nueva(self.visibles.len(), ancho, alto)),
                Vista::Mosaicos => Box::new(mosaicos::Mosaicos::nuevos(self.visibles.len(), ancho, alto, self.av().tamano_iconos)),
                _ => Box::new(cuadricula::Cuadricula::nueva(self.visibles.len(), ancho, alto, self.av().tamano_iconos)),
            };
            d.encabezado(self, ui, &p, acciones);
            let tamano = d.tamano();
            ScrollArea::vertical()
                .id_salt(("vista", self.id_pestana(), vista as u8))
                .auto_shrink(false)
                .scroll_source(egui::scroll_area::ScrollSource { drag: egui::scroll_area::DragScroll::Never, ..Default::default() })
                .show_viewport(ui, |ui, viewport| {
                    let origen = ui.max_rect().min;
                    ui.set_min_size(vec2(ui.available_width(), tamano.y.max(viewport.height())));
                    let visible = viewport.translate(origen.to_vec2());
                    self.contenido(ui, &p, &*d, visible, origen, viewport, acciones);
                });
            self.disposicion = Some(d);
        });
    }

    /// Fondo, elementos, banda y estado vacío. `visible` está en coordenadas
    /// de pantalla; `en_contenido` es lo mismo en coordenadas del contenido.
    #[allow(clippy::too_many_arguments)]
    fn contenido(
        &mut self,
        ui: &mut Ui,
        p: &Paleta,
        d: &dyn Disposicion,
        visible: Rect,
        origen: Pos2,
        en_contenido: Rect,
        acciones: &mut Vec<Accion>,
    ) {
        let fondo = ui.interact(visible, ui.id().with("fondo"), Sense::click_and_drag());
        d.decorar(self, ui, visible, acciones);
        let rango = d.rango(en_contenido);
        let primero = rango.start;
        let margen = en_contenido.expand(60.0);
        let mut sobre_carpeta = false;
        for k in rango {
            let r = d.rect(k);
            if !margen.intersects(r) {
                continue;
            }
            sobre_carpeta |= self.elemento(ui, p, d, k, k - primero, r.translate(origen.to_vec2()), acciones);
        }
        self.limpiar_animaciones(ui.ctx());
        if self.desplazar_a_foco {
            if d.desplazable()
                && let Some(k) = self.foco.clone().and_then(|f| self.indice_visible(&f))
            {
                ui.scroll_to_rect(d.rect(k).translate(origen.to_vec2()), None);
            }
            self.desplazar_a_foco = false;
        }
        if self.visibles.is_empty() {
            self.estado_vacio(ui, p, visible);
        }
        self.fondo(ui, p, &fondo, origen, &|k| d.caja_banda(k), d.desplazable(), sobre_carpeta, acciones);
    }

    /// Un archivo: se pregunta a la vista cómo pintarlo y el resto es común.
    /// Devuelve si es una carpeta con archivos arrastrados encima.
    #[allow(clippy::too_many_arguments)]
    fn elemento(&mut self, ui: &mut Ui, p: &Paleta, d: &dyn Disposicion, k: usize, orden: usize, rect: Rect, acciones: &mut Vec<Accion>) -> bool {
        let _ = p;
        let it = self.item(k);
        let resp = ui.interact(d.zona(rect), ui.id().with(("elemento", &it.ruta)), Sense::click_and_drag());
        let seleccionado = self.seleccion.contains(&it.ruta);
        let soltar_aqui = it.es_carpeta() && resp.dnd_hover_payload::<Arrastre>().is_some_and(|a| !a.0.contains(&it.ruta));
        let ctx = ui.ctx().clone();
        let visual = Visual {
            seleccionado,
            soltar_aqui,
            hover: ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered() && !seleccionado, 0.14),
            presion: ctx.animate_bool_with_time(resp.id.with("p"), resp.is_pointer_button_down_on(), 0.08),
            salto: self.salto(&it.ruta),
            alfa: if self.cortado(&it.ruta) { 0.45 } else if it.entrada.es_oculto { 0.65 } else { 1.0 },
            indice: k,
        };
        // Al abrir la carpeta, los elementos aparecen en cascada
        let t = self.aparicion(orden);
        let mut pintor = ui.painter().clone();
        pintor.multiply_opacity(t);
        d.pintar(self, &pintor, &it.entrada, d.al_aparecer(rect, t), &visual);
        let resp = resp.on_hover_ui_at_pointer(|ui| self.tooltip(ui, &it.entrada));
        self.interacciones(ui, &resp, &it, acciones);
        soltar_aqui
    }

    fn aviso_papelera(&mut self, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        Frame::new().fill(p.superficie).inner_margin(Margin::symmetric(16, 10)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(ph::TRASH).size(18.0).color(p.texto_tenue));
                ui.label(RichText::new("Lo que está aquí se puede restaurar hasta que vacíes la papelera.").color(p.texto_tenue));
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let hay = !self.entradas.is_empty();
                    let vaciar = egui::Button::new(
                        RichText::new(format!("{}  Vaciar papelera", ph::BROOM)).color(if hay { p.peligro } else { p.texto_tenue }),
                    );
                    if ui.add_enabled(hay, vaciar).clicked() {
                        acciones.push(Accion::VaciarPapelera);
                    }
                    let sel = self.seleccionados();
                    let texto = if sel.is_empty() { "Restaurar todo" } else { "Restaurar selección" };
                    if ui.add_enabled(hay, egui::Button::new(format!("{}  {texto}", ph::ARROW_COUNTER_CLOCKWISE))).clicked() {
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
        // En la órbita el sol ocupa el centro: el aviso va abajo
        let orbita = self.vista() == Vista::Orbita;
        let painter = ui.painter_at(area);
        let c = if orbita { pos2(area.center().x, area.bottom() - 120.0) } else { area.center() - vec2(0.0, 30.0) };
        if !orbita {
            painter.text(c, Align2::CENTER_CENTER, glifo, FontId::proportional(54.0), con_alfa(p.texto_tenue, 0.55));
        }
        painter.text(c + vec2(0.0, 50.0), Align2::CENTER_CENTER, titulo, FontId::proportional(17.0), p.texto);
        if !detalle.is_empty() {
            let g = texto_ajustado(&painter, &detalle, (area.width() - 60.0).max(120.0), 3, 13.0, p.texto_tenue, true);
            pintar_galley_centrado(&painter, c.x, c.y + 66.0, g, p.texto_tenue);
        }
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
    //  Interacción (igual en todas las vistas)
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
        // Clic con la rueda: la carpeta se abre en una pestaña nueva
        if resp.middle_clicked() && it.es_carpeta() {
            acciones.push(Accion::AbrirEnPestana(ruta.clone()));
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
            && let Some(a) = crate::ui_util::soltado::<Arrastre>(resp)
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
    #[allow(clippy::too_many_arguments)]
    fn fondo(
        &mut self,
        ui: &mut Ui,
        p: &Paleta,
        fondo: &Response,
        origen: Pos2,
        caja_de: &dyn Fn(usize) -> Rect,
        desplazable: bool,
        sobre_carpeta: bool,
        acciones: &mut Vec<Accion>,
    ) {
        let m = ui.input(|i| i.modifiers);
        // Archivos de otro lado encima del fondo: se marca toda la vista
        let de_otro_lado = fondo.dnd_hover_payload::<Arrastre>().is_some_and(|a| match &self.ubicacion {
            Ubicacion::Ruta(dir) => a.0.is_empty() || a.0.iter().any(|r| r.parent() != Some(dir.as_path())),
            Ubicacion::Papelera => true,
        });
        if de_otro_lado && !sobre_carpeta {
            ui.painter().rect(
                fondo.rect.shrink(4.0),
                CornerRadius::same(12),
                con_alfa(p.acento, 0.05),
                Stroke::new(2.0, con_alfa(p.acento, 0.65)),
                StrokeKind::Inside,
            );
        }
        // Soltar en el fondo (no en una carpeta): a la carpeta abierta. Sirve
        // al traer archivos de otra pestaña (se abre sola al pasar por encima).
        // Si vienen de aquí mismo, no pasa nada.
        if let Some(a) = crate::ui_util::soltado::<Arrastre>(fondo)
            && !a.0.is_empty()
        {
            match &self.ubicacion {
                Ubicacion::Ruta(dir) => acciones.push(Accion::Soltar { destino: dir.clone(), rutas: a.0.clone(), copiar: m.ctrl }),
                Ubicacion::Papelera => acciones.push(Accion::Papelera(a.0.clone())),
            }
        }
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
            if caja_de(k).intersects(caja) {
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
        if desplazable {
            let clip = ui.clip_rect();
            if puntero.y < clip.top() + 28.0 {
                ui.scroll_with_delta(vec2(0.0, 14.0));
            } else if puntero.y > clip.bottom() - 28.0 {
                ui.scroll_with_delta(vec2(0.0, -14.0));
            }
        }
        ui.ctx().request_repaint();
        if suelto {
            self.banda = None;
        }
    }

    /// Flechas, Enter, Retroceso, Escape y escribir para buscar. A dónde
    /// lleva cada flecha lo decide la vista que se ve.
    pub(crate) fn teclado_vista(&mut self, ctx: &egui::Context, acciones: &mut Vec<Accion>) {
        let eventos = ctx.input(|i| i.events.clone());
        for ev in eventos {
            match ev {
                Event::Key { key, pressed: true, modifiers, .. } => {
                    if modifiers.ctrl || modifiers.alt || modifiers.command {
                        continue;
                    }
                    let es_movimiento = matches!(
                        key,
                        Key::ArrowRight | Key::ArrowLeft | Key::ArrowDown | Key::ArrowUp | Key::PageDown | Key::PageUp | Key::Home | Key::End
                    );
                    if es_movimiento {
                        if self.visibles.is_empty() {
                            continue;
                        }
                        let actual = self.foco.clone().and_then(|f| self.indice_visible(&f));
                        let nuevo = self.disposicion.as_ref().and_then(|d| d.vecino(actual, key));
                        let Some(nuevo) = nuevo.filter(|&n| n < self.visibles.len()) else { continue };
                        let ruta = self.entradas[self.visibles[nuevo]].ruta.clone();
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
}

/// Recorre una cuadrícula de `cols` columnas con las flechas.
pub(crate) fn vecino_rejilla(actual: Option<usize>, tecla: Key, n: usize, cols: usize, por_pagina: usize) -> Option<usize> {
    if n == 0 {
        return None;
    }
    let cols = cols.max(1) as isize;
    let paso = match tecla {
        Key::ArrowRight if cols > 1 => 1,
        Key::ArrowLeft if cols > 1 => -1,
        Key::ArrowDown => cols,
        Key::ArrowUp => -cols,
        Key::PageDown => por_pagina.max(1) as isize * cols,
        Key::PageUp => -(por_pagina.max(1) as isize) * cols,
        Key::Home => isize::MIN / 2,
        Key::End => isize::MAX / 2,
        _ => return None,
    };
    Some(match actual {
        None => 0,
        Some(a) => (a as isize).saturating_add(paso).clamp(0, n as isize - 1) as usize,
    })
}

/// Muestra el ícono de una entrada como widget (para diálogos).
pub fn icono_entrada(app: &Solar, ui: &mut Ui, e: &Entrada, lado: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(lado, lado), Sense::hover());
    app.pintar_icono(ui.painter(), rect, e, 1.0);
}
