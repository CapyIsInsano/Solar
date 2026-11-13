//! Barras: superior (navegación, ubicación, búsqueda y botones de ventana),
//! barra de ubicación abajo (opcional), barra de estado y barra lateral
//! (lugares, papelera y memorias USB).

use crate::app::{Accion, Arrastre, Solar, Ubicacion};
use crate::iconos;
use crate::nucleo::{self, Dispositivo, Posicion};
use crate::tema::{Paleta, con_alfa};
use crate::ui_util::{self, boton_icono, boton_icono_estado};
use crate::{formato, tareas};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Layout, Margin, Rect, ResizeDirection,
    RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, ViewportCommand, pos2, vec2,
};
use egui_phosphor::regular as ph;
use std::path::{Path, PathBuf};

const ALTO_BARRA: f32 = 46.0;

/// Un pedazo de la barra de ubicación.
struct Segmento {
    texto: String,
    glifo: Option<&'static str>,
    ruta: Option<PathBuf>,
}

impl Solar {
    fn segmentos(&self) -> Vec<Segmento> {
        let ruta = match &self.ubicacion {
            Ubicacion::Papelera => {
                return vec![Segmento { texto: "Papelera".into(), glifo: Some(ph::TRASH), ruta: None }];
            }
            Ubicacion::Ruta(r) => r.clone(),
        };
        let hogar = nucleo::dir_hogar();
        // Raíz: memoria USB, carpeta personal o el sistema
        let (raiz, mut segs) = if let Some(d) = self
            .dispositivos
            .iter()
            .filter(|d| d.punto_montaje.as_ref().is_some_and(|p| ruta.starts_with(p)))
            .max_by_key(|d| d.punto_montaje.as_ref().map(|p| p.as_os_str().len()))
        {
            let p = d.punto_montaje.clone().unwrap_or_default();
            let glifo = if d.extraible { ph::USB } else { ph::HARD_DRIVE };
            (p.clone(), vec![Segmento { texto: d.nombre.clone(), glifo: Some(glifo), ruta: Some(p) }])
        } else if ruta.starts_with(&hogar) && hogar != Path::new("/") {
            (hogar.clone(), vec![Segmento { texto: "Carpeta personal".into(), glifo: Some(ph::HOUSE), ruta: Some(hogar.clone()) }])
        } else {
            (PathBuf::from("/"), vec![Segmento { texto: "Sistema".into(), glifo: Some(ph::HARD_DRIVES), ruta: Some("/".into()) }])
        };
        let mut actual = raiz.clone();
        if let Ok(resto) = ruta.strip_prefix(&raiz) {
            for c in resto.components() {
                actual.push(c);
                segs.push(Segmento { texto: c.as_os_str().to_string_lossy().into_owned(), glifo: None, ruta: Some(actual.clone()) });
            }
        }
        segs
    }

    /// La barra de ubicación ("donde te dice el árbol de carpetas").
    fn barra_ruta(&mut self, ui: &mut Ui, p: &Paleta, ancho: f32, acciones: &mut Vec<Accion>) {
        let alto = 32.0;
        let (rect, fondo) = ui.allocate_exact_size(vec2(ancho, alto), Sense::click());
        ui.painter().rect(
            rect,
            CornerRadius::same(9),
            con_alfa(p.texto, if p.oscuro { 0.06 } else { 0.05 }),
            Stroke::new(1.0, con_alfa(p.borde, 0.7)),
            StrokeKind::Inside,
        );
        let interior = rect.shrink2(vec2(6.0, 3.0));
        let mut hijo = ui.new_child(egui::UiBuilder::new().max_rect(interior).layout(Layout::left_to_right(Align::Center)));

        if let Some(texto) = self.editando_ruta.as_mut() {
            let resp = hijo.add(
                egui::TextEdit::singleline(texto)
                    .frame(Frame::NONE)
                    .desired_width(interior.width() - 8.0)
                    .font(FontId::proportional(14.0))
                    .hint_text("Escribe una ruta, por ejemplo ~/Juegos"),
            );
            if self.enfocar_ruta {
                // Todo seleccionado: escribir reemplaza la ruta (como en Nautilus)
                resp.request_focus();
                if let Some(mut estado) = egui::TextEdit::load_state(hijo.ctx(), resp.id) {
                    let fin = egui::text::CCursor::new(texto.chars().count());
                    estado.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), fin)));
                    estado.store(hijo.ctx(), resp.id);
                }
                self.enfocar_ruta = false;
            }
            if resp.lost_focus() {
                if hijo.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let destino = nucleo::expandir(texto);
                    self.editando_ruta = None;
                    if destino.is_dir() {
                        acciones.push(Accion::Navegar(Ubicacion::Ruta(destino)));
                    } else if destino.exists() {
                        acciones.push(Accion::Abrir(destino));
                    } else {
                        self.avisar(format!("No existe «{}».", destino.display()), crate::app::TipoAviso::Error);
                    }
                } else {
                    self.editando_ruta = None;
                }
            }
            return;
        }

        if fondo.clicked() {
            acciones.push(Accion::EditarRuta);
        }
        let segmentos = self.segmentos();
        ScrollArea::horizontal()
            .id_salt("migas")
            .stick_to_right(true)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(&mut hijo, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let total = segmentos.len();
                    for (i, s) in segmentos.into_iter().enumerate() {
                        if i > 0 {
                            ui.label(RichText::new(ph::CARET_RIGHT).size(11.0).color(p.texto_tenue));
                        }
                        let ultimo = i + 1 == total;
                        let texto = match s.glifo {
                            Some(g) => format!("{g}  {}", s.texto),
                            None => s.texto.clone(),
                        };
                        let rich = if ultimo { RichText::new(texto).strong().color(p.texto) } else { RichText::new(texto).color(p.texto_tenue) };
                        let resp = ui.add(egui::Button::new(rich).frame(false).min_size(vec2(0.0, 24.0)));
                        let soltar = s.ruta.is_some() && resp.dnd_hover_payload::<Arrastre>().is_some();
                        if soltar {
                            ui.painter().rect_stroke(resp.rect, CornerRadius::same(6), Stroke::new(1.5, p.acento), StrokeKind::Inside);
                        }
                        if let Some(r) = &s.ruta {
                            if resp.clicked() && !ultimo {
                                acciones.push(Accion::Navegar(Ubicacion::Ruta(r.clone())));
                            }
                            if let Some(a) = crate::ui_util::soltado::<Arrastre>(&resp) {
                                let copiar = ui.input(|i| i.modifiers.ctrl);
                                acciones.push(Accion::Soltar { destino: r.clone(), rutas: a.0.clone(), copiar });
                            }
                            let r = r.clone();
                            resp.on_hover_cursor(CursorIcon::PointingHand).context_menu(|ui| {
                                if ui.button(format!("{}   Abrir en pestaña nueva", ph::BROWSERS)).clicked() {
                                    acciones.push(Accion::AbrirEnPestana(r.clone()));
                                    ui.close();
                                }
                                if ui.button(format!("{}   Abrir en ventana nueva", ph::APP_WINDOW)).clicked() {
                                    acciones.push(Accion::AbrirEnVentana(r.clone()));
                                    ui.close();
                                }
                                if ui.button(format!("{}   Copiar ruta", ph::LINK)).clicked() {
                                    acciones.push(Accion::CopiarRuta(vec![r.clone()]));
                                    ui.close();
                                }
                                if ui.button(format!("{}   Propiedades", ph::INFO)).clicked() {
                                    acciones.push(Accion::Propiedades(vec![r.clone()]));
                                    ui.close();
                                }
                            });
                        }
                    }
                });
            });
    }

    fn campo_busqueda(&mut self, ui: &mut Ui, p: &Paleta) {
        let Some(texto) = self.busqueda.as_mut() else { return };
        let resp = ui.add(
            egui::TextEdit::singleline(texto)
                .desired_width(190.0)
                .hint_text(format!("{}  Buscar aquí…", ph::MAGNIFYING_GLASS))
                .text_color(p.texto),
        );
        if self.enfocar_busqueda {
            resp.request_focus();
            // Cursor al final (si se abrió escribiendo una letra)
            if let Some(mut estado) = egui::TextEdit::load_state(ui.ctx(), resp.id) {
                let fin = egui::text::CCursor::new(texto.chars().count());
                estado.cursor.set_char_range(Some(egui::text::CCursorRange::one(fin)));
                estado.store(ui.ctx(), resp.id);
            }
            self.enfocar_busqueda = false;
        }
        if resp.changed() {
            self.visibles_sucios = true;
        }
        if resp.lost_focus() && texto.is_empty() {
            self.busqueda = None;
            self.visibles_sucios = true;
        }
    }

    // ------------------------------------------------------------------
    //  Barra superior
    // ------------------------------------------------------------------

    pub(crate) fn barra_superior(&mut self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        let p = self.paleta.clone();
        let integrada = self.config.barra_titulo_integrada;
        egui::Panel::top("barra_superior")
            .exact_size(ALTO_BARRA)
            .frame(Frame::new().fill(p.superficie).inner_margin(Margin::symmetric(8, 0)).stroke(Stroke::NONE))
            .show_separator_line(false)
            .show(ui, |ui| {
                let rect = ui.max_rect();
                // Arrastrar la ventana desde cualquier hueco de la barra
                let fondo = ui.interact(rect, Id::new("arrastrar_ventana"), Sense::click_and_drag());
                if integrada {
                    if fondo.drag_started_by(egui::PointerButton::Primary) {
                        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                    }
                    if fondo.double_clicked() {
                        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!self.maximizada));
                    }
                }
                ui.painter().line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    Stroke::new(1.0, con_alfa(p.borde, 0.6)),
                );

                let textos: std::collections::HashMap<&str, String> =
                    ["atras", "adelante", "subir", "ajustes", "ocultos", "buscar", "barra_lateral"]
                        .into_iter()
                        .map(|id| (id, self.atajos.texto(id)))
                        .collect();
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let a = |id: &str| textos.get(id).cloned().unwrap_or_default();
                    if boton_icono(ui, &p, ph::ARROW_LEFT, &format!("Atrás ({})", a("atras")), self.pos_historial > 0).clicked() {
                        acciones.push(Accion::Atras);
                    }
                    let hay_adelante = self.pos_historial + 1 < self.historial.len();
                    if boton_icono(ui, &p, ph::ARROW_RIGHT, &format!("Adelante ({})", a("adelante")), hay_adelante).clicked() {
                        acciones.push(Accion::Adelante);
                    }
                    let hay_padre = self.ubicacion.ruta().and_then(|r| r.parent()).is_some();
                    if boton_icono(ui, &p, ph::ARROW_UP, &format!("Carpeta superior ({})", a("subir")), hay_padre).clicked() {
                        acciones.push(Accion::Subir);
                    }
                    ui.add_space(8.0);

                    // Ancho de lo de la derecha para darle el resto a la ubicación
                    let ancho_busqueda = if self.busqueda.is_some() { 200.0 } else { 34.0 };
                    let derecha = ancho_busqueda + 4.0 * 34.0 + 30.0 + 16.0;
                    let ancho_medio = (ui.available_width() - derecha).max(120.0);

                    if self.config.barra_ubicacion == Posicion::Arriba {
                        self.barra_ruta(ui, &p, ancho_medio, acciones);
                    } else {
                        let (r, _) = ui.allocate_exact_size(vec2(ancho_medio, 30.0), Sense::hover());
                        let titulo = self.titulo.trim_end_matches(" — Solar").to_string();
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, titulo, FontId::proportional(15.0), p.texto);
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        let ajustes_abiertos = self.ajustes.is_some();
                        if boton_icono_estado(ui, &p, ph::GEAR_SIX, &format!("Ajustes ({})", a("ajustes")), true, ajustes_abiertos).clicked() {
                            acciones.push(Accion::Ajustes);
                        }
                        // El menú Vista, como el de Windows
                        let (glifo, nombre, _) = crate::vista::info_vista(self.vista());
                        let boton = ui
                            .add(
                                egui::Button::new(RichText::new(format!("{glifo} {}", ph::CARET_DOWN)).size(16.0).color(p.texto))
                                    .frame(false)
                                    .min_size(vec2(46.0, 30.0)),
                            )
                            .on_hover_text(format!("Vista: {nombre}"));
                        if boton.hovered() {
                            ui.painter().rect_filled(boton.rect, CornerRadius::same(8), p.hover);
                        }
                        egui::Popup::menu(&boton).show(|ui| self.menu_vistas(ui, acciones));
                        let ocultos = self.av().mostrar_ocultos;
                        if boton_icono_estado(ui, &p, if ocultos { ph::EYE } else { ph::EYE_SLASH }, &format!("Archivos ocultos ({})", a("ocultos")), true, ocultos).clicked() {
                            acciones.push(Accion::AlternarOcultos);
                        }
                        if self.busqueda.is_some() {
                            self.campo_busqueda(ui, &p);
                        } else if boton_icono(ui, &p, ph::MAGNIFYING_GLASS, &format!("Buscar ({})", a("buscar")), true).clicked() {
                            acciones.push(Accion::Buscar);
                        }
                        let lateral = self.config.mostrar_barra_lateral;
                        if boton_icono_estado(ui, &p, ph::SIDEBAR_SIMPLE, &format!("Barra lateral ({})", a("barra_lateral")), true, lateral).clicked() {
                            acciones.push(Accion::AlternarLateral);
                        }
                    });
                });
            });
    }

    /// Barra de ubicación abajo (si así lo elegiste en Ajustes).
    pub(crate) fn panel_ubicacion_abajo(&mut self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        let p = self.paleta.clone();
        egui::Panel::bottom("ubicacion_abajo")
            .exact_size(44.0)
            .frame(Frame::new().fill(p.superficie).inner_margin(Margin::symmetric(10, 0)))
            .show_separator_line(false)
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ui.painter().line_segment([rect.left_top(), rect.right_top()], Stroke::new(1.0, con_alfa(p.borde, 0.6)));
                ui.horizontal_centered(|ui| {
                    let ancho = ui.available_width();
                    self.barra_ruta(ui, &p, ancho, acciones);
                });
            });
    }

    // ------------------------------------------------------------------
    //  Barra de estado
    // ------------------------------------------------------------------

    fn armar_texto_estado(&self) -> String {
        let (mut n_sel, mut bytes, mut carpetas) = (0usize, 0u64, 0usize);
        if !self.seleccion.is_empty() {
            for &i in &self.visibles {
                let e = &self.entradas[i];
                if self.seleccion.contains(&e.ruta) {
                    n_sel += 1;
                    if e.es_carpeta() { carpetas += 1 } else { bytes += e.tamano }
                }
            }
        }
        if n_sel == 0 {
            let ocultos = if self.av().mostrar_ocultos || self.ubicacion == Ubicacion::Papelera {
                0
            } else {
                self.entradas.iter().filter(|e| e.es_oculto).count()
            };
            let mut t = formato::elementos(self.visibles.len());
            if ocultos > 0 {
                t.push_str(&if ocultos == 1 {
                    " (1 oculto)".to_string()
                } else {
                    format!(" ({} ocultos)", formato::miles(ocultos as u64))
                });
            }
            t
        } else {
            let mut t = format!(
                "{} de {} {}",
                formato::miles(n_sel as u64),
                formato::miles(self.visibles.len() as u64),
                if n_sel == 1 { "seleccionado" } else { "seleccionados" }
            );
            if bytes > 0 {
                t.push_str(&format!(" · {}", formato::tamano(bytes)));
            }
            if carpetas > 0 {
                t.push_str(&if carpetas == 1 {
                    " · 1 carpeta".to_string()
                } else {
                    format!(" · {} carpetas", formato::miles(carpetas as u64))
                });
            }
            t
        }
    }

    pub(crate) fn barra_estado(&mut self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        let p = self.paleta.clone();
        egui::Panel::bottom("barra_estado")
            .exact_size(30.0)
            .frame(Frame::new().fill(p.superficie).inner_margin(Margin::symmetric(12, 0)))
            .show_separator_line(false)
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ui.painter().line_segment([rect.left_top(), rect.right_top()], Stroke::new(1.0, con_alfa(p.borde, 0.5)));
                ui.horizontal_centered(|ui| {
                    // Recorrer miles de archivos en cada cuadro se nota: el texto
                    // se arma solo cuando cambia la selección o lo visible
                    let mostrar_todo = self.av().mostrar_ocultos || self.ubicacion == Ubicacion::Papelera;
                    let clave = (self.seleccion.version(), self.version_visibles, mostrar_todo);
                    let guardado = self.texto_estado.borrow().as_ref().filter(|(k, _)| *k == clave).map(|(_, t)| t.clone());
                    let texto = match guardado {
                        Some(t) => t,
                        None => {
                            let t = self.armar_texto_estado();
                            *self.texto_estado.borrow_mut() = Some((clave, t.clone()));
                            t
                        }
                    };
                    ui.label(RichText::new(texto).size(12.5).color(p.texto_tenue));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Tamaño de íconos
                        let mut tam = self.av().tamano_iconos;
                        ui.label(RichText::new(ph::MAGNIFYING_GLASS_PLUS).size(14.0).color(p.texto_tenue));
                        ui.spacing_mut().slider_width = 110.0;
                        let r = ui.add(egui::Slider::new(&mut tam, 40.0..=160.0).show_value(false).step_by(4.0))
                            .on_hover_text("Tamaño de los íconos (Ctrl + rueda del mouse)");
                        if r.changed() {
                            self.cambiar_av(|a| a.tamano_iconos = tam);
                        }
                        ui.label(RichText::new(ph::MAGNIFYING_GLASS_MINUS).size(14.0).color(p.texto_tenue));
                        ui.add_space(12.0);
                        if let Some(e) = self.espacio {
                            let usado = 1.0 - e.disponible as f32 / e.total.max(1) as f32;
                            ui_util::barra_capacidad(ui, &p, usado, 60.0);
                            ui.label(
                                RichText::new(format!("{} libres de {}", formato::tamano(e.disponible), formato::tamano(e.total)))
                                    .size(12.5)
                                    .color(p.texto_tenue),
                            );
                        }
                        if !self.tareas.is_empty() {
                            ui.add_space(12.0);
                            let t = &self.tareas[0];
                            let fraccion = t.fraccion();
                            let texto = match fraccion {
                                Some(f) => format!("{} · {:.0}%", t.titulo, f * 100.0),
                                None => t.titulo.clone(),
                            };
                            let resp = ui.add(egui::Button::new(RichText::new(texto).size(12.5)).frame(false));
                            ui.add(egui::ProgressBar::new(fraccion.unwrap_or(0.0)).desired_width(90.0).desired_height(6.0).animate(fraccion.is_none()));
                            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                self.mostrar_tareas = !self.mostrar_tareas;
                            }
                        }
                    });
                });
            });
        // Ctrl + rueda cambia el tamaño de los íconos
        let zoom = ui.input(|i| i.zoom_delta());
        if (zoom - 1.0).abs() > 0.001 && self.dialogo.is_none() && self.ajustes.is_none() {
            acciones.push(Accion::Zoom(if zoom > 1.0 { 4.0 } else { -4.0 }));
        }
    }

    // ------------------------------------------------------------------
    //  Barra lateral
    // ------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn fila_lateral(
        &self,
        ui: &mut Ui,
        p: &Paleta,
        glifo: &str,
        color_glifo: Color32,
        texto: &str,
        activa: bool,
        apagada: bool,
    ) -> egui::Response {
        let ancho = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(ancho, 32.0), Sense::click());
        let soltar = resp.dnd_hover_payload::<Arrastre>().is_some();
        let fondo = if activa {
            con_alfa(p.acento, 0.20)
        } else if soltar {
            con_alfa(p.acento, 0.16)
        } else if resp.hovered() {
            p.hover
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, CornerRadius::same(8), fondo);
        if soltar {
            ui.painter().rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.5, p.acento), StrokeKind::Inside);
        }
        ui.painter().text(
            rect.left_center() + vec2(18.0, 0.0),
            Align2::CENTER_CENTER,
            glifo,
            FontId::new(17.0, egui::FontFamily::Name(iconos::FUENTE_RELLENA.into())),
            if apagada { con_alfa(color_glifo, 0.5) } else { color_glifo },
        );
        let color = if apagada { p.texto_tenue } else { p.texto };
        let g = ui.painter().layout(
            texto.to_string(),
            FontId::proportional(14.0),
            color,
            (rect.width() - 46.0).max(20.0),
        );
        let g = if g.rows.len() > 1 {
            let mut job = egui::text::LayoutJob::simple(texto.to_string(), FontId::proportional(14.0), color, rect.width() - 46.0);
            job.wrap.max_rows = 1;
            job.wrap.overflow_character = Some('…');
            ui.painter().layout_job(job)
        } else {
            g
        };
        ui.painter().galley(pos2(rect.left() + 36.0, rect.center().y - g.size().y / 2.0), g, color);
        resp.on_hover_cursor(CursorIcon::PointingHand)
    }

    pub(crate) fn barra_lateral(&mut self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        let p = self.paleta.clone();
        let panel = match self.config.barra_lateral {
            nucleo::Lado::Izquierda => egui::Panel::left("barra_lateral"),
            nucleo::Lado::Derecha => egui::Panel::right("barra_lateral"),
        };
        // Se desliza al mostrarla o esconderla (F9)
        let mut visible = self.config.mostrar_barra_lateral;
        panel
            .resizable(true)
            .drag_to_open(false)
            .default_size(220.0)
            .size_range(170.0..=380.0)
            .frame(Frame::new().fill(p.superficie).inner_margin(Margin::symmetric(8, 10)))
            .show_separator_line(true)
            .show_collapsible(ui, &mut visible, |ui| {
                ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui_util::encabezado(ui, &p, "Lugares");
                    let lugares = self.config.lugares.clone();
                    for (i, l) in lugares.iter().enumerate() {
                        let ruta = nucleo::expandir(&l.ruta);
                        let existe = ruta.is_dir();
                        let estilo = self.carpetas.lock().ok().and_then(|db| db.obtener(&ruta)).unwrap_or_default();
                        let glifo = iconos::emblema(&l.icono)
                            .or_else(|| estilo.icono.as_deref().and_then(iconos::emblema))
                            .map(|e| e.glifo)
                            .unwrap_or(ph::FOLDER);
                        let color = estilo.color.unwrap_or(p.carpeta);
                        let activa = self.ubicacion.ruta() == Some(ruta.as_path());
                        let resp = self.fila_lateral(ui, &p, glifo, color, &l.nombre, activa, !existe);
                        let resp = resp.on_hover_text(l.ruta.clone());
                        if resp.clicked() {
                            acciones.push(Accion::Navegar(Ubicacion::Ruta(ruta.clone())));
                        }
                        if resp.middle_clicked() {
                            acciones.push(Accion::AbrirEnPestana(ruta.clone()));
                        }
                        if let Some(a) = crate::ui_util::soltado::<Arrastre>(&resp) {
                            let copiar = ui.input(|i| i.modifiers.ctrl);
                            acciones.push(Accion::Soltar { destino: ruta.clone(), rutas: a.0.clone(), copiar });
                        }
                        resp.context_menu(|ui| {
                            ui.set_min_width(220.0);
                            if ui.button(format!("{}   Abrir", ph::FOLDER_OPEN)).clicked() {
                                acciones.push(Accion::Navegar(Ubicacion::Ruta(ruta.clone())));
                                ui.close();
                            }
                            if ui.button(format!("{}   Abrir en pestaña nueva", ph::BROWSERS)).clicked() {
                                acciones.push(Accion::AbrirEnPestana(ruta.clone()));
                                ui.close();
                            }
                            if ui.button(format!("{}   Abrir en ventana nueva", ph::APP_WINDOW)).clicked() {
                                acciones.push(Accion::AbrirEnVentana(ruta.clone()));
                                ui.close();
                            }
                            ui.separator();
                            if existe {
                                self.menus_estilo(ui, std::slice::from_ref(&ruta), acciones);
                                ui.separator();
                            }
                            if ui.button(format!("{}   Editar lugares…", ph::PENCIL_SIMPLE)).clicked() {
                                acciones.push(Accion::Ajustes);
                                ui.close();
                            }
                            if ui.button(RichText::new(format!("{}   Quitar de Lugares", ph::X)).color(p.peligro)).clicked() {
                                acciones.push(Accion::QuitarLugar(i));
                                ui.close();
                            }
                        });
                    }
                    let glifo = if self.papelera_vacia { ph::TRASH_SIMPLE } else { ph::TRASH };
                    let resp = self.fila_lateral(ui, &p, glifo, p.texto_tenue, "Papelera", self.ubicacion == Ubicacion::Papelera, false);
                    if !self.papelera_vacia {
                        ui.painter().circle_filled(resp.rect.right_center() - vec2(14.0, 0.0), 3.5, p.acento);
                    }
                    let resp = resp.on_hover_text(if self.papelera_vacia { "La papelera está vacía" } else { "Hay cosas en la papelera" });
                    if resp.clicked() {
                        acciones.push(Accion::Navegar(Ubicacion::Papelera));
                    }
                    if resp.middle_clicked() {
                        acciones.push(Accion::AbrirPapeleraEnPestana);
                    }
                    if let Some(a) = crate::ui_util::soltado::<Arrastre>(&resp) {
                        acciones.push(Accion::Papelera(a.0.clone()));
                    }
                    resp.context_menu(|ui| {
                        if ui.button(format!("{}   Abrir", ph::FOLDER_OPEN)).clicked() {
                            acciones.push(Accion::Navegar(Ubicacion::Papelera));
                            ui.close();
                        }
                        if ui.add_enabled(!self.papelera_vacia, egui::Button::new(RichText::new(format!("{}   Vaciar papelera…", ph::BROOM)).color(p.peligro))).clicked() {
                            acciones.push(Accion::VaciarPapelera);
                            ui.close();
                        }
                    });

                    let mostrar: Vec<Dispositivo> = self
                        .dispositivos
                        .iter()
                        .filter(|d| if d.extraible { self.config.mostrar_usb } else { self.config.mostrar_discos_internos })
                        .cloned()
                        .collect();
                    if !mostrar.is_empty() {
                        ui.add_space(10.0);
                        ui_util::encabezado(ui, &p, "Dispositivos");
                        for d in mostrar {
                            self.fila_dispositivo(ui, &p, &d, acciones);
                        }
                    }
                });
            });
        if visible != self.config.mostrar_barra_lateral {
            // La escondieron arrastrando el borde
            self.config.mostrar_barra_lateral = visible;
            self.config_cambio();
        }
    }

    fn fila_dispositivo(&mut self, ui: &mut Ui, p: &Paleta, d: &Dispositivo, acciones: &mut Vec<Accion>) {
        let glifo = if d.extraible { ph::USB } else { ph::HARD_DRIVE };
        let activa = d.punto_montaje.as_ref().is_some_and(|m| self.ubicacion.ruta() == Some(m.as_path()));
        let ocupado = self.montando.contains(&d.dispositivo);
        let resp = self.fila_lateral(ui, p, glifo, if d.extraible { p.acento } else { p.texto_tenue }, &d.nombre, activa, !d.montado);
        let rect = resp.rect;
        // Botón de expulsar a la derecha
        let mut expulsar_clic = false;
        if d.montado && !ocupado {
            let caja = Rect::from_center_size(rect.right_center() - vec2(16.0, 0.0), vec2(26.0, 26.0));
            let r = ui.interact(caja, ui.id().with(("expulsar", &d.dispositivo)), Sense::click());
            if r.hovered() {
                ui.painter().rect_filled(caja, CornerRadius::same(6), con_alfa(p.texto, 0.1));
            }
            ui.painter().text(caja.center(), Align2::CENTER_CENTER, ph::EJECT, FontId::proportional(15.0), p.texto_tenue);
            if r.on_hover_text(if d.extraible { "Expulsar" } else { "Desmontar" }).clicked() {
                expulsar_clic = true;
                acciones.push(Accion::Desmontar { dispositivo: d.dispositivo.clone(), expulsar: d.extraible });
            }
        } else if ocupado {
            ui.put(Rect::from_center_size(rect.right_center() - vec2(16.0, 0.0), vec2(16.0, 16.0)), egui::Spinner::new().size(14.0));
        }
        if d.montado && d.tamano > 0 {
            let usado = 1.0 - d.libre as f32 / d.tamano as f32;
            let barra = Rect::from_min_size(pos2(rect.left() + 36.0, rect.bottom() - 4.0), vec2(rect.width() - 76.0, 3.0));
            ui.painter().rect_filled(barra, CornerRadius::same(2), con_alfa(p.texto, 0.12));
            let mut lleno = barra;
            lleno.set_width(barra.width() * usado.clamp(0.0, 1.0));
            ui.painter().rect_filled(lleno, CornerRadius::same(2), if usado > 0.9 { p.peligro } else { p.acento });
        }
        let detalle = format!(
            "{}\n{}{}",
            d.dispositivo,
            if d.montado {
                format!("{} libres de {}", formato::tamano(d.libre), formato::tamano(d.tamano))
            } else {
                format!("{} · sin montar (clic para montar)", formato::tamano(d.tamano))
            },
            d.sistema_archivos.as_deref().map(|f| format!(" · {f}")).unwrap_or_default()
        );
        let resp = resp.on_hover_text(detalle);
        if resp.clicked() && !expulsar_clic {
            match &d.punto_montaje {
                Some(m) => acciones.push(Accion::Navegar(Ubicacion::Ruta(m.clone()))),
                None => acciones.push(Accion::Montar { dispositivo: d.dispositivo.clone(), abrir: true }),
            }
        }
        if let (Some(m), Some(a)) = (&d.punto_montaje, crate::ui_util::soltado::<Arrastre>(&resp)) {
            acciones.push(Accion::Soltar { destino: m.clone(), rutas: a.0.clone(), copiar: true });
        }
        let d2 = d.clone();
        resp.context_menu(|ui| {
            ui.set_min_width(210.0);
            match &d2.punto_montaje {
                Some(m) => {
                    if ui.button(format!("{}   Abrir", ph::FOLDER_OPEN)).clicked() {
                        acciones.push(Accion::Navegar(Ubicacion::Ruta(m.clone())));
                        ui.close();
                    }
                    if ui.button(format!("{}   Abrir en ventana nueva", ph::APP_WINDOW)).clicked() {
                        acciones.push(Accion::AbrirEnVentana(m.clone()));
                        ui.close();
                    }
                    if ui.button(format!("{}   Propiedades", ph::INFO)).clicked() {
                        acciones.push(Accion::Propiedades(vec![m.clone()]));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(format!("{}   Desmontar", ph::ARROW_LINE_UP)).clicked() {
                        acciones.push(Accion::Desmontar { dispositivo: d2.dispositivo.clone(), expulsar: false });
                        ui.close();
                    }
                    if d2.extraible && ui.button(format!("{}   Expulsar con seguridad", ph::EJECT)).clicked() {
                        acciones.push(Accion::Desmontar { dispositivo: d2.dispositivo.clone(), expulsar: true });
                        ui.close();
                    }
                }
                None => {
                    if ui.button(format!("{}   Montar y abrir", ph::FOLDER_OPEN)).clicked() {
                        acciones.push(Accion::Montar { dispositivo: d2.dispositivo.clone(), abrir: true });
                        ui.close();
                    }
                    if ui.button(format!("{}   Solo montar", ph::PLUG)).clicked() {
                        acciones.push(Accion::Montar { dispositivo: d2.dispositivo.clone(), abrir: false });
                        ui.close();
                    }
                }
            }
        });
    }

    // ------------------------------------------------------------------
    //  Bordes para redimensionar (cuando Solar dibuja su propia barra)
    // ------------------------------------------------------------------

    pub(crate) fn bordes_ventana(&self, ctx: &egui::Context) {
        let pantalla = ctx.content_rect();
        let g = 5.0;
        let zonas = [
            (Rect::from_min_max(pantalla.left_top(), pos2(pantalla.left() + g * 2.0, pantalla.top() + g * 2.0)), ResizeDirection::NorthWest, CursorIcon::ResizeNorthWest),
            (Rect::from_min_max(pos2(pantalla.right() - g * 2.0, pantalla.top()), pos2(pantalla.right(), pantalla.top() + g * 2.0)), ResizeDirection::NorthEast, CursorIcon::ResizeNorthEast),
            (Rect::from_min_max(pos2(pantalla.left(), pantalla.bottom() - g * 2.0), pos2(pantalla.left() + g * 2.0, pantalla.bottom())), ResizeDirection::SouthWest, CursorIcon::ResizeSouthWest),
            (Rect::from_min_max(pantalla.right_bottom() - vec2(g * 2.0, g * 2.0), pantalla.right_bottom()), ResizeDirection::SouthEast, CursorIcon::ResizeSouthEast),
            (Rect::from_min_max(pantalla.left_top(), pos2(pantalla.right(), pantalla.top() + g)), ResizeDirection::North, CursorIcon::ResizeNorth),
            (Rect::from_min_max(pos2(pantalla.left(), pantalla.bottom() - g), pantalla.right_bottom()), ResizeDirection::South, CursorIcon::ResizeSouth),
            (Rect::from_min_max(pantalla.left_top(), pos2(pantalla.left() + g, pantalla.bottom())), ResizeDirection::West, CursorIcon::ResizeWest),
            (Rect::from_min_max(pos2(pantalla.right() - g, pantalla.top()), pantalla.right_bottom()), ResizeDirection::East, CursorIcon::ResizeEast),
        ];
        for (i, (rect, dir, cursor)) in zonas.into_iter().enumerate() {
            egui::Area::new(Id::new(("borde", i)))
                .order(egui::Order::Foreground)
                .fixed_pos(rect.min)
                .interactable(true)
                .show(ctx, |ui| {
                    let (_, r) = ui.allocate_exact_size(rect.size(), Sense::drag());
                    if r.hovered() || r.dragged() {
                        ctx.set_cursor_icon(cursor);
                    }
                    if r.drag_started() {
                        ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
                    }
                });
        }
    }
}

#[allow(dead_code)]
fn _nombre(p: &Path) -> String {
    tareas::nombre(p)
}
