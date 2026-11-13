//! Ventana de Ajustes: apariencia (temas, transparencia), distribución
//! (barra de ubicación y barra lateral), lugares, memorias USB, atajos de
//! teclado, archivos y "Acerca de". Todo se guarda en config.lua.

use crate::app::{Accion, Solar, TipoAviso, Ubicacion};
use crate::atajos::{self, Combinacion};
use crate::iconos::{self, Logotipo};
use crate::nucleo::{self, ColorTema, Lado, Lugar, Orden, OrdenOrbita, Posicion, Tema, Vista};
use crate::tema::{self, Paleta, con_alfa};
use crate::sonido::{self, Sonido};
use crate::ui_util::{self, fila, interruptor};
use crate::formato;
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, Event, FontId, Frame, Id, Key, Layout, Margin, Rect, RichText, ScrollArea,
    Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};
use egui_phosphor::regular as ph;

pub const ID_EDITOR: &str = "__editor__";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pestana {
    Apariencia,
    Distribucion,
    Lugares,
    Dispositivos,
    Atajos,
    Archivos,
    Actualizaciones,
    AcercaDe,
}

impl Pestana {
    const TODAS: [Pestana; 8] = [
        Pestana::Apariencia,
        Pestana::Distribucion,
        Pestana::Lugares,
        Pestana::Dispositivos,
        Pestana::Atajos,
        Pestana::Archivos,
        Pestana::Actualizaciones,
        Pestana::AcercaDe,
    ];

    fn nombre(self) -> (&'static str, &'static str) {
        match self {
            Pestana::Apariencia => (ph::PALETTE, "Apariencia"),
            Pestana::Distribucion => (ph::LAYOUT, "Distribución"),
            Pestana::Lugares => (ph::STAR, "Lugares"),
            Pestana::Dispositivos => (ph::USB, "Memorias USB"),
            Pestana::Atajos => (ph::KEYBOARD, "Atajos"),
            Pestana::Archivos => (ph::FILES, "Archivos"),
            Pestana::Actualizaciones => (ph::ARROWS_CLOCKWISE, "Actualizaciones"),
            Pestana::AcercaDe => (ph::SUN, "Acerca de"),
        }
    }
}

pub struct EditorTema {
    pub tema: Tema,
    pub tema_anterior: String,
    pub error: Option<String>,
}

pub struct EstadoAjustes {
    pub pestana: Pestana,
    pub capturando: Option<String>,
    pub aviso_atajo: Option<String>,
    pub editor: Option<EditorTema>,
    pub nuevo_lugar: String,
    pub terminal_detectada: Option<String>,
    /// Tamaño de la interfaz mientras se arrastra (se aplica al soltar)
    pub escala_arrastre: Option<f32>,
}

impl EstadoAjustes {
    pub fn nuevo(_app: &Solar) -> Self {
        Self {
            pestana: Pestana::Apariencia,
            capturando: None,
            aviso_atajo: None,
            editor: None,
            nuevo_lugar: String::new(),
            terminal_detectada: nucleo::detectar_terminal(),
            escala_arrastre: None,
        }
    }
}

/// "Mi tema bonito" -> "mi_tema_bonito"
fn id_de(nombre: &str) -> String {
    let mut s: String = formato::plegar(nombre)
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    while s.contains("__") {
        s = s.replace("__", "_");
    }
    let s = s.trim_matches('_').to_string();
    if s.is_empty() { "mi_tema".into() } else { s.chars().take(40).collect() }
}

impl Solar {
    pub(crate) fn ventana_ajustes(&mut self, ctx: &egui::Context, acciones: &mut Vec<Accion>) {
        let Some(mut estado) = self.ajustes.take() else { return };
        let p = self.paleta.clone();
        let mut abierto = true;
        let t = self.progreso_animacion(self.ajustes_desde, 0.22);
        if t < 1.0 {
            ctx.request_repaint();
        }
        ctx.set_transform_layer(
            egui::LayerId::new(egui::Order::Middle, Id::new("ventana_ajustes")),
            egui::emath::TSTransform::from_translation(vec2(0.0, (1.0 - t) * 18.0)),
        );
        egui::Window::new(RichText::new(format!("{}  Ajustes", ph::GEAR_SIX)).strong())
            .id(Id::new("ventana_ajustes"))
            .default_pos(ctx.content_rect().center())
            .pivot(Align2::CENTER_CENTER)
            .open(&mut abierto)
            .collapsible(false)
            .resizable(true)
            .default_size([800.0, 580.0])
            .min_width(660.0)
            .min_height(420.0)
            .frame(
                Frame::window(&ctx.global_style())
                    .fill(p.elevado)
                    .corner_radius(CornerRadius::same(16))
                    .inner_margin(Margin::same(14))
                    .multiply_with_opacity(t),
            )
            .show(ctx, |ui| {
                ui.multiply_opacity(t);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(170.0);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for pe in Pestana::TODAS {
                            let (glifo, nombre) = pe.nombre();
                            let activa = estado.pestana == pe;
                            let (rect, resp) = ui.allocate_exact_size(vec2(170.0, 34.0), Sense::click());
                            resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, activa, nombre));
                            let fondo = if activa {
                                con_alfa(p.acento, 0.2)
                            } else if resp.hovered() {
                                p.hover
                            } else {
                                Color32::TRANSPARENT
                            };
                            ui.painter().rect_filled(rect, CornerRadius::same(8), fondo);
                            ui.painter().text(
                                rect.left_center() + vec2(18.0, 0.0),
                                Align2::CENTER_CENTER,
                                glifo,
                                FontId::proportional(17.0),
                                if activa { p.acento } else { p.texto_tenue },
                            );
                            ui.painter().text(
                                rect.left_center() + vec2(36.0, 0.0),
                                Align2::LEFT_CENTER,
                                nombre,
                                FontId::proportional(14.0),
                                p.texto,
                            );
                            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                if estado.pestana != pe {
                                    sonido::pedir(Sonido::Clic);
                                }
                                estado.pestana = pe;
                                estado.capturando = None;
                                if pe == Pestana::Actualizaciones {
                                    self.tamano_cache = None;
                                }
                            }
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        // Cada pestaña recuerda su propio desplazamiento
                        ScrollArea::vertical().id_salt(("ajustes", estado.pestana as u8)).auto_shrink([false, false]).show(ui, |ui| {
                            ui.set_min_width(ui.available_width() - 8.0);
                            match estado.pestana {
                                Pestana::Apariencia => self.ajustes_apariencia(ui, &p, &mut estado),
                                Pestana::Distribucion => self.ajustes_distribucion(ui, &p),
                                Pestana::Lugares => self.ajustes_lugares(ui, &p, &mut estado),
                                Pestana::Dispositivos => self.ajustes_dispositivos(ui, &p, acciones),
                                Pestana::Atajos => self.ajustes_atajos(ui, &p, &mut estado),
                                Pestana::Archivos => self.ajustes_archivos(ui, &p, &mut estado),
                                Pestana::Actualizaciones => self.ajustes_actualizaciones(ui, &p, acciones),
                                Pestana::AcercaDe => self.ajustes_acerca(ui, &p, acciones),
                            }
                        });
                    });
                });
            });
        if abierto {
            self.ajustes = Some(estado);
        } else if let Some(ed) = estado.editor.take() {
            self.cancelar_editor(ed);
        }
    }

    // ------------------------------------------------------------------
    //  Apariencia
    // ------------------------------------------------------------------

    fn tarjeta_tema(&self, ui: &mut Ui, p: &Paleta, t: &Tema, activo: bool) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(162.0, 104.0), Sense::click());
        let pintor = ui.painter();
        let vista = Rect::from_min_size(rect.min, vec2(rect.width(), 74.0));
        pintor.rect_filled(vista, CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 }, t.c(ColorTema::Fondo));
        let lateral = Rect::from_min_size(vista.min, vec2(40.0, vista.height()));
        pintor.rect_filled(lateral, CornerRadius { nw: 10, ne: 0, sw: 0, se: 0 }, t.c(ColorTema::Superficie));
        for i in 0..3 {
            let y = vista.top() + 14.0 + i as f32 * 14.0;
            let color = if i == 0 { t.c(ColorTema::Acento) } else { con_alfa(t.c(ColorTema::TextoTenue), 0.6) };
            pintor.rect_filled(Rect::from_min_size(pos2(vista.left() + 8.0, y), vec2(24.0, 5.0)), CornerRadius::same(2), color);
        }
        for i in 0..3 {
            let r = Rect::from_min_size(pos2(vista.left() + 50.0 + i as f32 * 36.0, vista.top() + 16.0), vec2(30.0, 30.0));
            let logo = if i == 0 { Logotipo::Sol } else { Logotipo::Ninguno };
            iconos::carpeta(pintor, r, t.c(ColorTema::Carpeta), logo, None, 1.0);
            pintor.rect_filled(
                Rect::from_min_size(pos2(r.left() + 4.0, r.bottom() + 6.0), vec2(22.0, 4.0)),
                CornerRadius::same(2),
                con_alfa(t.c(ColorTema::Texto), 0.6),
            );
        }
        let pie = Rect::from_min_max(pos2(rect.left(), vista.bottom()), rect.max);
        pintor.rect_filled(pie, CornerRadius { nw: 0, ne: 0, sw: 10, se: 10 }, t.c(ColorTema::Elevado));
        pintor.text(pie.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, &t.nombre, FontId::proportional(13.0), t.c(ColorTema::Texto));
        if !t.integrado {
            pintor.text(pie.right_center() - vec2(10.0, 0.0), Align2::RIGHT_CENTER, "Lua", FontId::proportional(11.0), t.c(ColorTema::TextoTenue));
        }
        let borde = if activo {
            Stroke::new(2.5, p.acento)
        } else if resp.hovered() {
            Stroke::new(1.5, con_alfa(p.acento, 0.6))
        } else {
            Stroke::new(1.0, p.borde)
        };
        pintor.rect_stroke(rect, CornerRadius::same(10), borde, StrokeKind::Outside);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn ajustes_apariencia(&mut self, ui: &mut Ui, p: &Paleta, estado: &mut EstadoAjustes) {
        ui_util::titulo(ui, p, "Apariencia");
        if estado.editor.is_some() {
            self.editor_tema(ui, p, estado);
            return;
        }
        // Si el espacio de trabajo tiene su propio tema, aquí se cambia el general
        let propio = self.espacio_trabajo.datos.tema.clone();
        if !propio.is_empty() {
            let nombre_tema = self.temas.iter().find(|t| t.id == propio).map(|t| t.nombre.clone()).unwrap_or(propio.clone());
            let espacio = self.espacio_trabajo.datos.nombre.clone();
            aviso_espacio(ui, p, &format!("El espacio «{espacio}» usa su propio tema ({nombre_tema}). Lo que elijas aquí es el tema general."), |ui| {
                if ui.button("Usar el general aquí").clicked() {
                    self.espacio_trabajo.datos.tema.clear();
                    self.apariencia_aplicada = None;
                    self.marcar_sesion();
                }
            });
        }
        ui_util::encabezado(ui, p, "Tema de colores");
        let temas: Vec<Tema> = self.temas.iter().filter(|t| t.id != ID_EDITOR).cloned().collect();
        let por_fila = ((ui.available_width() + 10.0) / 172.0).floor().max(1.0) as usize;
        for grupo in temas.chunks(por_fila) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                for t in grupo {
                    if self.tarjeta_tema(ui, p, t, self.config.tema == t.id).clicked() && self.config.tema != t.id {
                        self.config.tema = t.id.clone();
                        self.config_cambio();
                    }
                }
            });
            ui.add_space(10.0);
        }
        ui.horizontal(|ui| {
            if ui.button(format!("{}  Crear un tema a partir de este…", ph::PAINT_BRUSH)).clicked() {
                let mut t = self.tema_actual().clone();
                let anterior = self.config.tema.clone();
                t.nombre = format!("{} (mío)", t.nombre);
                t.id = ID_EDITOR.into();
                t.integrado = false;
                self.temas.retain(|x| x.id != ID_EDITOR);
                self.temas.push(t.clone());
                self.config.tema = ID_EDITOR.into();
                estado.editor = Some(EditorTema { tema: t, tema_anterior: anterior, error: None });
            }
            if ui.button(format!("{}  Recargar temas", ph::ARROW_CLOCKWISE)).clicked() {
                self.recargar_temas();
                self.avisar("Temas recargados.", TipoAviso::Info);
            }
        });
        ui.label(
            RichText::new("Tus temas viven en ~/.config/solar/temas/ como archivos .lua (puedes editarlos a mano).")
                .small()
                .color(p.texto_tenue),
        );
        ui.add_space(14.0);

        ui_util::encabezado(ui, p, "Ventana");
        let mut cambio = false;
        fila(ui, p, "Fondo transparente", "Deja ver tu fondo de pantalla. Necesita un compositor (Niri, Hyprland, KWin, picom…).", |ui| {
            cambio |= interruptor(ui, p, &mut self.config.transparencia).changed();
        });
        let transparencia = self.config.transparencia;
        fila(ui, p, "Opacidad", "Qué tan sólido se ve el fondo cuando es transparente.", |ui| {
            ui.add_enabled_ui(transparencia, |ui| {
                let mut v = self.config.opacidad * 100.0;
                if ui.add(egui::Slider::new(&mut v, 30.0..=100.0).suffix(" %").step_by(1.0).fixed_decimals(0)).changed() {
                    self.config.opacidad = v / 100.0;
                    cambio = true;
                }
            });
        });
        fila(ui, p, "Tamaño de la interfaz", "Hace más grande o más chico todo (texto, botones, barras).", |ui| {
            // Cambiar la escala mientras arrastras movería el propio control,
            // así que mostramos el valor y lo aplicamos al soltar
            let mut v = estado.escala_arrastre.unwrap_or(self.config.escala * 100.0);
            let r = ui.add(egui::Slider::new(&mut v, 75.0..=200.0).suffix(" %").step_by(5.0).fixed_decimals(0));
            if r.dragged() {
                estado.escala_arrastre = Some(v);
            }
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                self.config.escala = v / 100.0;
                estado.escala_arrastre = None;
                cambio = true;
            }
        });
        fila(ui, p, "Fuente", "La integrada, la de tu sistema (fontconfig) o un archivo .ttf/.otf.", |ui| {
            let actual = match self.config.fuente.as_str() {
                "" => "Integrada".to_string(),
                "sistema" => "Del sistema".to_string(),
                otra => format!("Archivo: {}", std::path::Path::new(otra).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()),
            };
            egui::ComboBox::from_id_salt("fuente").selected_text(actual).width(190.0).show_ui(ui, |ui| {
                if ui.selectable_label(self.config.fuente.is_empty(), "Integrada").clicked() {
                    self.config.fuente.clear();
                    cambio = true;
                }
                if ui.selectable_label(self.config.fuente == "sistema", "Del sistema").clicked() {
                    self.config.fuente = "sistema".into();
                    cambio = true;
                }
            });
        });
        if !self.config.fuente.is_empty() && self.config.fuente != "sistema" || ui.input(|i| i.modifiers.alt) {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Ruta de la fuente:").color(p.texto_tenue));
                let mut ruta = if self.config.fuente == "sistema" { String::new() } else { self.config.fuente.clone() };
                if ui.add(egui::TextEdit::singleline(&mut ruta).desired_width(300.0).hint_text("/usr/share/fonts/…/Inter.ttf")).lost_focus() {
                    self.config.fuente = ruta;
                    cambio = true;
                }
            });
        }
        fila(ui, p, "Barra de título de Solar", "Solar dibuja sus propios botones de ventana. Se aplica al reiniciar.", |ui| {
            cambio |= interruptor(ui, p, &mut self.config.barra_titulo_integrada).changed();
        });

        ui.add_space(14.0);
        ui_util::encabezado(ui, p, "Sonido y animaciones");
        let salida = nucleo::sonido_salida();
        let detalle = if salida.is_empty() {
            "No encontré PulseAudio/PipeWire ni ALSA, así que no sonará nada.".to_string()
        } else {
            format!("Al hacer clic, al terminar de copiar, en errores… (sale por {salida}).")
        };
        fila(ui, p, "Efectos de sonido", &detalle, |ui| {
            cambio |= interruptor(ui, p, &mut self.config.sonidos).changed();
        });
        let sonidos = self.config.sonidos;
        fila(ui, p, "Volumen", "Los clics suenan un poco más bajito que los avisos.", |ui| {
            ui.add_enabled_ui(sonidos, |ui| {
                if ui.button(format!("{}  Probar", ph::SPEAKER_HIGH)).clicked() {
                    sonido::configurar(true, self.config.volumen);
                    sonido::probar();
                }
                let mut v = self.config.volumen * 100.0;
                let r = ui.add(egui::Slider::new(&mut v, 0.0..=100.0).suffix(" %").step_by(5.0).fixed_decimals(0));
                if r.changed() {
                    self.config.volumen = v / 100.0;
                    cambio = true;
                }
                if r.drag_stopped() {
                    sonido::configurar(true, self.config.volumen);
                    sonido::probar();
                }
            });
        });
        fila(ui, p, "Animaciones", "Transiciones suaves al abrir carpetas, menús, ventanas y al cambiar de tema. Apágalas si tu PC es lenta.", |ui| {
            cambio |= interruptor(ui, p, &mut self.config.animaciones).changed();
        });
        if cambio {
            self.config_cambio();
        }
    }

    fn editor_tema(&mut self, ui: &mut Ui, p: &Paleta, estado: &mut EstadoAjustes) {
        let Some(ed) = estado.editor.as_mut() else { return };
        ui_util::encabezado(ui, p, "Nuevo tema");
        let mut cambio = false;
        ui.horizontal(|ui| {
            ui.label("Nombre:");
            ui.add(egui::TextEdit::singleline(&mut ed.tema.nombre).desired_width(240.0));
            ui.add_space(12.0);
            ui.label("Oscuro");
            cambio |= interruptor(ui, p, &mut ed.tema.oscuro).changed();
        });
        ui.label(RichText::new(format!("Se guardará como ~/.config/solar/temas/{}.lua", id_de(&ed.tema.nombre))).small().color(p.texto_tenue));
        ui.add_space(8.0);
        egui::Grid::new("colores_tema").num_columns(4).spacing(vec2(12.0, 8.0)).show(ui, |ui| {
            for (i, c) in ColorTema::TODOS.iter().enumerate() {
                let mut color = ed.tema.colores[*c as usize];
                if egui::color_picker::color_edit_button_srgba(ui, &mut color, egui::color_picker::Alpha::Opaque).changed() {
                    ed.tema.colores[*c as usize] = color;
                    cambio = true;
                }
                ui.label(RichText::new(format!("{}  {}", c.nombre(), tema::hex(color))).color(p.texto));
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
        if cambio {
            let t = ed.tema.clone();
            if let Some(x) = self.temas.iter_mut().find(|x| x.id == ID_EDITOR) {
                *x = t;
            }
        }
        if let Some(e) = &ed.error {
            ui.label(RichText::new(e).color(p.peligro));
        }
        ui.add_space(12.0);
        let mut guardar = false;
        let mut cancelar = false;
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(RichText::new(format!("{}  Guardar tema", ph::FLOPPY_DISK)).color(p.texto_acento)).fill(p.acento)).clicked() {
                guardar = true;
            }
            if ui.button("Cancelar").clicked() {
                cancelar = true;
            }
        });
        if cancelar {
            if let Some(ed) = estado.editor.take() {
                self.cancelar_editor(ed);
            }
        } else if guardar && let Some(ed) = estado.editor.as_mut() {
            let id = id_de(&ed.tema.nombre);
            let mut t = ed.tema.clone();
            t.id = id.clone();
            let archivo = self.rutas.temas.join(format!("{id}.lua"));
            match nucleo::guardar_tema(&archivo, &t) {
                Ok(()) => {
                    estado.editor = None;
                    self.temas.retain(|x| x.id != ID_EDITOR);
                    self.recargar_temas();
                    self.config.tema = id;
                    self.config_cambio();
                    self.avisar(format!("Tema «{}» guardado.", t.nombre), TipoAviso::Exito);
                }
                Err(e) => ed.error = Some(e.mensaje),
            }
        }
    }

    pub(crate) fn cancelar_editor(&mut self, ed: EditorTema) {
        self.temas.retain(|x| x.id != ID_EDITOR);
        self.config.tema = ed.tema_anterior;
        self.apariencia_aplicada = None;
    }

    // ------------------------------------------------------------------
    //  Distribución
    // ------------------------------------------------------------------

    /// Dibujito de la ventana con la barra resaltada.
    fn esquema(&self, ui: &mut Ui, p: &Paleta, ubicacion: Posicion, lado: Lado, resaltar_lateral: bool, activo: bool) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(150.0, 100.0), Sense::click());
        let pintor = ui.painter();
        let ventana = rect.shrink(8.0);
        pintor.rect_filled(ventana, CornerRadius::same(8), self.paleta.fondo);
        let barra_sup = Rect::from_min_size(ventana.min, vec2(ventana.width(), 12.0));
        pintor.rect_filled(barra_sup, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, p.superficie);
        let ancho_lat = 32.0;
        let lat = match lado {
            Lado::Izquierda => Rect::from_min_size(pos2(ventana.left(), barra_sup.bottom()), vec2(ancho_lat, ventana.height() - 12.0)),
            Lado::Derecha => Rect::from_min_size(pos2(ventana.right() - ancho_lat, barra_sup.bottom()), vec2(ancho_lat, ventana.height() - 12.0)),
        };
        pintor.rect_filled(lat, CornerRadius::ZERO, if resaltar_lateral { con_alfa(p.acento, 0.7) } else { p.superficie });
        let ruta = match ubicacion {
            Posicion::Arriba => Rect::from_min_size(barra_sup.min + vec2(28.0, 3.0), vec2(ventana.width() - 56.0, 6.0)),
            Posicion::Abajo => Rect::from_min_size(pos2(ventana.left() + 6.0, ventana.bottom() - 10.0), vec2(ventana.width() - 12.0, 6.0)),
        };
        pintor.rect_filled(ruta, CornerRadius::same(3), if resaltar_lateral { con_alfa(p.texto_tenue, 0.5) } else { p.acento });
        let contenido = Rect::from_min_max(
            pos2(if lado == Lado::Izquierda { lat.right() + 8.0 } else { ventana.left() + 8.0 }, barra_sup.bottom() + 8.0),
            pos2(if lado == Lado::Derecha { lat.left() - 8.0 } else { ventana.right() - 8.0 }, ventana.bottom() - 16.0),
        );
        for i in 0..3 {
            let r = Rect::from_min_size(contenido.min + vec2(i as f32 * 22.0, 0.0), vec2(18.0, 18.0));
            if r.right() < contenido.right() {
                iconos::carpeta(pintor, r, p.carpeta, Logotipo::Ninguno, None, 0.9);
            }
        }
        let borde = if activo { Stroke::new(2.5, p.acento) } else if resp.hovered() { Stroke::new(1.5, con_alfa(p.acento, 0.5)) } else { Stroke::new(1.0, p.borde) };
        pintor.rect_stroke(rect, CornerRadius::same(10), borde, StrokeKind::Inside);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn ajustes_distribucion(&mut self, ui: &mut Ui, p: &Paleta) {
        ui_util::titulo(ui, p, "Distribución");
        ui_util::encabezado(ui, p, "Barra de ubicación");
        ui.label(RichText::new("La barra que te dice dónde estás (Carpeta personal › Juegos › Retro). Haz clic en un pedazo para ir ahí, o en el espacio vacío para escribir una ruta.").color(p.texto_tenue));
        ui.add_space(6.0);
        let mut cambio = false;
        ui.horizontal(|ui| {
            for (pos, nombre) in [(Posicion::Arriba, "Arriba"), (Posicion::Abajo, "Abajo")] {
                ui.vertical(|ui| {
                    let activo = self.config.barra_ubicacion == pos;
                    if self.esquema(ui, p, pos, self.config.barra_lateral, false, activo).clicked() {
                        self.config.barra_ubicacion = pos;
                        cambio = true;
                    }
                    ui.label(RichText::new(nombre).color(if activo { p.acento } else { p.texto }));
                });
            }
        });
        ui.add_space(12.0);
        ui_util::encabezado(ui, p, "Barra lateral");
        ui.horizontal(|ui| {
            for (lado, nombre) in [(Lado::Izquierda, "Izquierda"), (Lado::Derecha, "Derecha")] {
                ui.vertical(|ui| {
                    let activo = self.config.barra_lateral == lado;
                    if self.esquema(ui, p, self.config.barra_ubicacion, lado, true, activo).clicked() {
                        self.config.barra_lateral = lado;
                        cambio = true;
                    }
                    ui.label(RichText::new(nombre).color(if activo { p.acento } else { p.texto }));
                });
            }
        });
        fila(ui, p, "Mostrar la barra lateral", &format!("También con {}.", self.atajos.texto("barra_lateral")), |ui| {
            cambio |= interruptor(ui, p, &mut self.config.mostrar_barra_lateral).changed();
        });
        ui.add_space(8.0);
        ui_util::encabezado(ui, p, "Vista");
        fila(ui, p, "Ver esta pestaña como", "Cada pestaña recuerda su vista. También con Ctrl+1 a Ctrl+4.", |ui| {
            let actual = self.vista();
            let (glifo, nombre, _) = crate::vista::info_vista(actual);
            egui::ComboBox::from_id_salt("vista_pestana").selected_text(format!("{glifo}  {nombre}")).width(190.0).show_ui(ui, |ui| {
                for v in Vista::TODAS {
                    let (g, n, _) = crate::vista::info_vista(v);
                    if ui.selectable_label(actual == v, format!("{g}  {n}")).clicked() && actual != v {
                        if let Some(pe) = self.pestanas.get_mut(self.activa) {
                            pe.vista = v;
                        }
                        self.espacio_trabajo.datos.vista = v;
                        self.disposicion = None;
                        self.desplazar_a_foco = true;
                        self.marcar_sesion();
                    }
                }
            });
        });
        fila(ui, p, "Vista de los espacios nuevos", "", |ui| {
            let (glifo, nombre, _) = crate::vista::info_vista(self.config.vista);
            egui::ComboBox::from_id_salt("vista_general").selected_text(format!("{glifo}  {nombre}")).width(190.0).show_ui(ui, |ui| {
                for v in Vista::TODAS {
                    let (g, n, _) = crate::vista::info_vista(v);
                    if ui.selectable_label(self.config.vista == v, format!("{g}  {n}")).clicked() {
                        self.config.vista = v;
                        cambio = true;
                    }
                }
            });
        });
        fila(ui, p, "Tamaño de los íconos", "También con Ctrl + rueda del mouse.", |ui| {
            let mut tam = self.av().tamano_iconos;
            if ui.add(egui::Slider::new(&mut tam, 40.0..=160.0).step_by(4.0).suffix(" px").fixed_decimals(0)).changed() {
                self.cambiar_av(|a| a.tamano_iconos = tam);
            }
        });
        if cambio {
            self.config_cambio();
        }
    }

    // ------------------------------------------------------------------
    //  Lugares
    // ------------------------------------------------------------------

    fn ajustes_lugares(&mut self, ui: &mut Ui, p: &Paleta, estado: &mut EstadoAjustes) {
        ui_util::titulo(ui, p, "Lugares");
        ui.label(RichText::new("Los atajos de la barra lateral. También puedes dar clic derecho en una carpeta y elegir «Añadir a Lugares».").color(p.texto_tenue));
        ui.add_space(8.0);
        let mut cambio = false;
        let mut quitar = None;
        let mut mover: Option<(usize, usize)> = None;
        let n = self.config.lugares.len();
        egui::Grid::new("lugares").num_columns(4).spacing(vec2(8.0, 6.0)).show(ui, |ui| {
            for i in 0..n {
                let l = &mut self.config.lugares[i];
                let glifo = iconos::emblema(&l.icono).map(|e| e.glifo).unwrap_or(ph::FOLDER);
                ui.menu_button(ui_util::glifo_relleno(glifo, 18.0, p.carpeta), |ui| {
                    ui.set_min_width(300.0);
                    ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                        if let Some(id) = ui_util::rejilla_logotipos(ui, p, Some(&l.icono), "", 8, 32.0) {
                            l.icono = id.to_string();
                            cambio = true;
                            ui.close();
                        }
                    });
                    if ui.button("Sin ícono").clicked() {
                        l.icono.clear();
                        cambio = true;
                        ui.close();
                    }
                });
                cambio |= ui.add_sized([150.0, 26.0], egui::TextEdit::singleline(&mut l.nombre)).changed();
                let existe = nucleo::expandir(&l.ruta).is_dir();
                let r = ui.add_sized(
                    [240.0, 26.0],
                    egui::TextEdit::singleline(&mut l.ruta).text_color(if existe { p.texto } else { p.peligro }),
                );
                cambio |= r.changed();
                ui.horizontal(|ui| {
                    if ui.add_enabled(i > 0, egui::Button::new(ph::ARROW_UP).small()).clicked() {
                        mover = Some((i, i - 1));
                    }
                    if ui.add_enabled(i + 1 < n, egui::Button::new(ph::ARROW_DOWN).small()).clicked() {
                        mover = Some((i, i + 1));
                    }
                    if ui.add(egui::Button::new(RichText::new(ph::X).color(p.peligro)).small()).on_hover_text("Quitar").clicked() {
                        quitar = Some(i);
                    }
                });
                ui.end_row();
            }
        });
        if let Some((a, b)) = mover {
            self.config.lugares.swap(a, b);
            cambio = true;
        }
        if let Some(i) = quitar {
            self.config.lugares.remove(i);
            cambio = true;
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut estado.nuevo_lugar).hint_text("~/Proyectos").desired_width(240.0));
            let ruta = nucleo::expandir(&estado.nuevo_lugar);
            if ui.add_enabled(!estado.nuevo_lugar.trim().is_empty() && ruta.is_dir(), egui::Button::new(format!("{}  Añadir", ph::PLUS))).clicked() {
                let nombre = crate::tareas::nombre(&ruta);
                self.config.lugares.push(Lugar { nombre, ruta: nucleo::contraer(&ruta), icono: String::new() });
                estado.nuevo_lugar.clear();
                cambio = true;
            }
            if let Some(dir) = self.ubicacion.ruta().map(std::path::Path::to_path_buf)
                && ui.button(format!("{}  Añadir la carpeta actual", ph::FOLDER_PLUS)).clicked()
            {
                self.config.lugares.push(Lugar { nombre: crate::tareas::nombre(&dir), ruta: nucleo::contraer(&dir), icono: String::new() });
                cambio = true;
            }
        });
        ui.add_space(6.0);
        if ui.button(format!("{}  Volver a los lugares de siempre", ph::ARROW_COUNTER_CLOCKWISE)).clicked() {
            self.config.lugares = nucleo::Config::defecto().lugares;
            cambio = true;
        }
        if cambio {
            self.config_cambio();
        }
    }

    // ------------------------------------------------------------------
    //  Memorias USB y discos
    // ------------------------------------------------------------------

    fn ajustes_dispositivos(&mut self, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        ui_util::titulo(ui, p, "Memorias USB y discos");
        let mut cambio = false;
        let antes = self.config.mostrar_discos_internos;
        fila(ui, p, "Mostrar memorias USB y tarjetas SD", "Aparecen en la barra lateral al conectarlas, con su espacio libre y botón para expulsar.", |ui| {
            cambio |= interruptor(ui, p, &mut self.config.mostrar_usb).changed();
        });
        fila(ui, p, "Mostrar discos internos", "Otras particiones (por ejemplo la de Windows) y discos de datos. Las del sistema no se muestran.", |ui| {
            cambio |= interruptor(ui, p, &mut self.config.mostrar_discos_internos).changed();
        });
        if antes != self.config.mostrar_discos_internos {
            self.vigia.enviar(crate::tareas::ComandoVigia::Dispositivos { incluir_internos: self.config.mostrar_discos_internos });
        }
        if cambio {
            self.config_cambio();
        }
        ui.add_space(8.0);
        ui_util::encabezado(ui, p, "Conectados ahora");
        if self.dispositivos.is_empty() {
            ui.label(RichText::new("No hay memorias ni discos extra conectados.").color(p.texto_tenue));
        }
        for d in self.dispositivos.clone() {
            Frame::new().fill(con_alfa(p.texto, 0.04)).corner_radius(CornerRadius::same(10)).inner_margin(Margin::same(10)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if d.extraible { ph::USB } else { ph::HARD_DRIVE }).size(22.0).color(p.acento));
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&d.nombre).strong());
                        let estado = if d.montado {
                            format!("{} · {} libres de {}", d.punto_montaje.as_ref().map(|m| m.display().to_string()).unwrap_or_default(), formato::tamano(d.libre), formato::tamano(d.tamano))
                        } else {
                            format!("{} · sin montar · {}", d.dispositivo, formato::tamano(d.tamano))
                        };
                        ui.label(RichText::new(estado).small().color(p.texto_tenue));
                        if d.montado && d.tamano > 0 {
                            ui_util::barra_capacidad(ui, p, 1.0 - d.libre as f32 / d.tamano as f32, 220.0);
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if self.montando.contains(&d.dispositivo) {
                            ui.spinner();
                        } else if d.montado {
                            if ui.button(format!("{}  {}", ph::EJECT, if d.extraible { "Expulsar" } else { "Desmontar" })).clicked() {
                                acciones.push(Accion::Desmontar { dispositivo: d.dispositivo.clone(), expulsar: d.extraible });
                            }
                            if let Some(m) = &d.punto_montaje
                                && ui.button(format!("{}  Abrir", ph::FOLDER_OPEN)).clicked()
                            {
                                acciones.push(Accion::Navegar(Ubicacion::Ruta(m.clone())));
                            }
                        } else if ui.button(format!("{}  Montar", ph::PLUG)).clicked() {
                            acciones.push(Accion::Montar { dispositivo: d.dispositivo.clone(), abrir: true });
                        }
                    });
                });
            });
            ui.add_space(6.0);
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new("Para montar y expulsar sin ser root, Solar usa udisks2 (udisksctl). En Gentoo: sys-fs/udisks; en Arch: udisks2.")
                .small()
                .color(p.texto_tenue),
        );
    }

    // ------------------------------------------------------------------
    //  Atajos de teclado
    // ------------------------------------------------------------------

    fn ajustes_atajos(&mut self, ui: &mut Ui, p: &Paleta, estado: &mut EstadoAjustes) {
        ui_util::titulo(ui, p, "Atajos de teclado");
        ui.label(RichText::new("Haz clic en un atajo y presiona la combinación nueva. Esc cancela, Retroceso lo deja sin atajo.").color(p.texto_tenue));
        if let Some(a) = &estado.aviso_atajo {
            ui.label(RichText::new(a).color(p.aviso));
        }
        ui.add_space(6.0);

        // Capturar la combinación
        if let Some(id) = estado.capturando.clone() {
            let mut nueva: Option<Option<Combinacion>> = None;
            ui.input(|i| {
                for ev in &i.events {
                    let (tecla, m) = match ev {
                        Event::Key { key, pressed: true, modifiers, .. } => (*key, *modifiers),
                        Event::Copy => (Key::C, egui::Modifiers { ctrl: true, ..i.modifiers }),
                        Event::Cut => (Key::X, egui::Modifiers { ctrl: true, ..i.modifiers }),
                        Event::Paste(_) => (Key::V, egui::Modifiers { ctrl: true, ..i.modifiers }),
                        _ => continue,
                    };
                    if tecla == Key::Escape && !m.any() {
                        nueva = Some(self.atajos.de(&id));
                    } else if tecla == Key::Backspace && !m.any() {
                        nueva = Some(None);
                    } else {
                        nueva = Some(Some(Combinacion::de_evento(tecla, m)));
                    }
                    break;
                }
            });
            if let Some(c) = nueva {
                estado.capturando = None;
                estado.aviso_atajo = None;
                if let Some(c) = &c
                    && let Some(otro) = self.atajos.conflicto(&id, c)
                {
                    let nombre = atajos::accion(&otro).map(|a| a.nombre).unwrap_or("otra acción");
                    estado.aviso_atajo = Some(format!("{} era de «{}»; se lo quité.", c.mostrar(), nombre));
                    if let Some(x) = self.atajos.lista.iter_mut().find(|(a, _)| *a == otro) {
                        x.1 = None;
                    }
                }
                if let Some(x) = self.atajos.lista.iter_mut().find(|(a, _)| *a == id) {
                    x.1 = c;
                }
                self.config_cambio();
            }
        }

        let defecto = nucleo::atajos_defecto();
        for grupo in atajos::GRUPOS {
            ui_util::encabezado(ui, p, grupo);
            for a in atajos::ACCIONES.iter().filter(|a| a.grupo == grupo) {
                let actual = self.atajos.de(a.id);
                let original = defecto.iter().find(|(id, _)| id == a.id).and_then(|(_, t)| Combinacion::parsear(t));
                fila(ui, p, a.nombre, "", |ui| {
                    if actual != original
                        && ui.add(egui::Button::new(ph::ARROW_COUNTER_CLOCKWISE).small()).on_hover_text("Volver al de siempre").clicked()
                    {
                        if let Some(x) = self.atajos.lista.iter_mut().find(|(id, _)| id == a.id) {
                            x.1 = original;
                        }
                        self.config_cambio();
                    }
                    let capturando = estado.capturando.as_deref() == Some(a.id);
                    let texto = if capturando {
                        RichText::new("Presiona las teclas…").color(p.acento)
                    } else {
                        match actual {
                            Some(c) => RichText::new(c.mostrar()).strong(),
                            None => RichText::new("Sin atajo").color(p.texto_tenue),
                        }
                    };
                    let b = egui::Button::new(texto).min_size(vec2(170.0, 28.0)).selected(capturando);
                    if ui.add(b).clicked() {
                        estado.capturando = if capturando { None } else { Some(a.id.to_string()) };
                    }
                });
            }
        }
        ui.add_space(8.0);
        if ui.button(format!("{}  Volver a todos los de siempre", ph::ARROW_COUNTER_CLOCKWISE)).clicked() {
            let (a, _) = atajos::Atajos::de_config(&defecto);
            self.atajos = a;
            estado.aviso_atajo = None;
            self.config_cambio();
        }
    }

    // ------------------------------------------------------------------
    //  Archivos y comportamiento
    // ------------------------------------------------------------------

    fn ajustes_archivos(&mut self, ui: &mut Ui, p: &Paleta, estado: &mut EstadoAjustes) {
        ui_util::titulo(ui, p, "Archivos");
        let mut cambio = false;
        let antes = *self.av();
        if self.espacio_trabajo.datos.propios {
            let espacio = self.espacio_trabajo.datos.nombre.clone();
            aviso_espacio(
                ui,
                p,
                &format!("El espacio «{espacio}» tiene sus propios ajustes de archivos y de la órbita. Aquí cambias los generales (los de los demás espacios)."),
                |ui| {
                    if ui.button("Configurar el espacio…").clicked() {
                        let f = self.form_espacio(false);
                        self.dialogo = Some(crate::dialogos::Dialogo::Espacio(Box::new(f)));
                    }
                },
            );
        }
        let a = &mut self.config.archivos;
        fila(ui, p, "Mostrar archivos ocultos", &format!("Los que empiezan con punto. También con {}.", self.atajos.texto("ocultos")), |ui| {
            cambio |= interruptor(ui, p, &mut a.mostrar_ocultos).changed();
        });
        fila(ui, p, "Carpetas primero", "", |ui| {
            cambio |= interruptor(ui, p, &mut a.carpetas_primero).changed();
        });
        fila(ui, p, "Ordenar por", "", |ui| {
            cambio |= ui.checkbox(&mut a.orden_descendente, "Al revés").changed();
            egui::ComboBox::from_id_salt("orden").selected_text(a.orden.nombre()).show_ui(ui, |ui| {
                for o in Orden::TODOS {
                    if ui.selectable_label(a.orden == o, o.nombre()).clicked() {
                        a.orden = o;
                        cambio = true;
                    }
                }
            });
        });
        ui.add_space(6.0);
        ui_util::encabezado(ui, p, "Vista Órbita");
        fila(ui, p, "Que giren alrededor de la carpeta", "Se detienen solos al pasar el mouse por encima.", |ui| {
            cambio |= interruptor(ui, p, &mut a.orbita_girar).changed();
        });
        fila(ui, p, "Velocidad del giro", "", |ui| {
            cambio |= ui.add(egui::Slider::new(&mut a.orbita_velocidad, 0.1..=4.0).logarithmic(true).max_decimals(1).suffix("×")).changed();
        });
        fila(ui, p, "Acomodo", "Qué va más cerca del centro.", |ui| {
            egui::ComboBox::from_id_salt("orbita_orden").selected_text(a.orbita_orden.nombre()).width(210.0).show_ui(ui, |ui| {
                for o in OrdenOrbita::TODOS {
                    if ui.selectable_label(a.orbita_orden == o, o.nombre()).clicked() {
                        a.orbita_orden = o;
                        cambio = true;
                    }
                }
            });
        });
        fila(ui, p, "Órbitas con electricidad", "Rayos que recorren las órbitas.", |ui| {
            cambio |= interruptor(ui, p, &mut a.orbita_electricidad).changed();
        });
        let ahora = *self.av();
        self.despues_de_cambiar_av(&antes, &ahora);
        ui.add_space(6.0);
        ui_util::encabezado(ui, p, "Al abrir Solar");
        fila(ui, p, "Volver a las pestañas de la última vez", "Cada espacio de trabajo recuerda las suyas.", |ui| {
            cambio |= interruptor(ui, p, &mut self.config.restaurar_sesion).changed();
        });
        ui.add_space(6.0);
        ui_util::encabezado(ui, p, "Abrir");
        fila(ui, p, "Abrir con un solo clic", "Como en KDE. Si está apagado, se abre con doble clic.", |ui| {
            cambio |= interruptor(ui, p, &mut self.config.un_clic).changed();
        });
        ui.add_space(6.0);
        ui_util::encabezado(ui, p, "Confirmaciones");
        fila(ui, p, "Preguntar antes de mandar a la papelera", "", |ui| {
            cambio |= interruptor(ui, p, &mut self.config.confirmar_papelera).changed();
        });
        fila(ui, p, "Preguntar antes de eliminar para siempre", "Muy recomendado.", |ui| {
            cambio |= interruptor(ui, p, &mut self.config.confirmar_eliminar).changed();
        });
        ui.add_space(6.0);
        ui_util::encabezado(ui, p, "Terminal");
        let detectada = estado.terminal_detectada.clone().unwrap_or_else(|| "ninguna".into());
        fila(ui, p, "Programa de terminal", &format!("Vacío = detectar automáticamente (ahora: {detectada})."), |ui| {
            if ui.button("Probar").clicked() {
                let dir = self.ubicacion.ruta().map(std::path::Path::to_path_buf).unwrap_or_else(nucleo::dir_hogar);
                if let Err(e) = nucleo::abrir_terminal(&self.config.terminal, &dir) {
                    self.avisar(e.mensaje, TipoAviso::Error);
                }
            }
            cambio |= ui.add(egui::TextEdit::singleline(&mut self.config.terminal).hint_text("foot, kitty, alacritty…").desired_width(170.0)).changed();
        });
        if cambio {
            self.config_cambio();
        }
    }

    // ------------------------------------------------------------------
    //  Actualizaciones y caché
    // ------------------------------------------------------------------

    fn ajustes_actualizaciones(&mut self, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        use crate::actualizar::{self, Estado};
        ui_util::titulo(ui, p, "Actualizaciones");
        ui.label(
            RichText::new(format!(
                "Solar revisa en GitHub ({}) si hay versiones publicadas o commits nuevos en la rama principal, y se actualiza solo, sin desinstalar ni reinstalar.",
                self.config.repo_actualizaciones
            ))
            .color(p.texto_tenue),
        );
        ui.add_space(10.0);

        let estado = self.actualizador.estado();
        let ancho = ui.available_width();
        Frame::new()
            .fill(con_alfa(p.texto, 0.04))
            .stroke(Stroke::new(1.0, p.borde))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(ancho - 30.0);
                let (glifo, color, titulo, detalle): (&str, Color32, String, String) = match &estado {
                    Estado::Nada => (ph::SUN, p.acento, format!("Tienes Solar {}", actualizar::version_completa()), "Aún no he revisado si hay algo nuevo.".into()),
                    Estado::Buscando => (ph::CLOUD_ARROW_DOWN, p.acento, "Buscando en GitHub…".into(), String::new()),
                    Estado::AlDia => (ph::CHECK_CIRCLE, p.exito, format!("Estás al día ({})", actualizar::version_completa()), "No hay versiones ni commits más nuevos.".into()),
                    Estado::Disponible(v) => (
                        ph::GIFT,
                        p.acento,
                        if v.rama.is_some() {
                            format!("Hay {}", v.nombre)
                        } else {
                            format!("Hay una versión nueva: {}{}", v.nombre, if v.previa { " (de prueba)" } else { "" })
                        },
                        format!(
                            "Tienes la {}.{}{}",
                            actualizar::version_completa(),
                            if v.fecha.is_empty() {
                                String::new()
                            } else if v.rama.is_some() {
                                format!(" Último cambio: {}.", v.fecha)
                            } else {
                                format!(" Publicada el {}.", v.fecha)
                            },
                            if v.binario.is_some() { " Trae el programa listo: solo se descarga." } else { " Se compilará desde el código (necesitas cargo)." }
                        ),
                    ),
                    Estado::Trabajando { paso, detalle, .. } => (ph::GEAR_SIX, p.acento, paso.clone(), detalle.clone()),
                    Estado::Listo { version } => (ph::CHECK_CIRCLE, p.exito, format!("Solar {version} quedó instalado"), "Reinícialo para empezar a usarlo.".into()),
                    Estado::Permiso { version, .. } => (
                        ph::LOCK_KEY,
                        p.aviso,
                        format!("Solar {version} está listo, pero necesito permiso"),
                        "Solar está instalado en una carpeta del sistema. Copia este comando en una terminal:".into(),
                    ),
                    Estado::Error(e) => (ph::WARNING_CIRCLE, p.peligro, "No se pudo completar".into(), e.clone()),
                };
                ui.horizontal(|ui| {
                    let girando = matches!(estado, Estado::Buscando | Estado::Trabajando { .. });
                    if girando {
                        ui.add(egui::Spinner::new().size(26.0).color(color));
                    } else {
                        ui.label(RichText::new(glifo).size(26.0).color(color));
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(titulo).size(15.5).strong().color(p.texto));
                        if !detalle.is_empty() {
                            ui.add(egui::Label::new(RichText::new(detalle).color(p.texto_tenue)).wrap());
                        }
                    });
                });
                if let Estado::Trabajando { fraccion: Some(f), .. } = &estado {
                    ui.add_space(6.0);
                    ui.add(egui::ProgressBar::new(*f).desired_height(8.0).animate(true));
                }
                if let Estado::Permiso { comando, .. } = &estado {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        Frame::new().fill(con_alfa(p.texto, 0.06)).corner_radius(CornerRadius::same(6)).inner_margin(Margin::same(8)).show(ui, |ui| {
                            ui.add(egui::Label::new(RichText::new(comando.as_str()).monospace().size(12.0)).wrap());
                        });
                        if ui.button(format!("{}  Copiar", ph::COPY)).clicked() {
                            ui.ctx().copy_text(comando.clone());
                            self.avisar("Comando copiado.", TipoAviso::Info);
                        }
                    });
                }
                if let Estado::Disponible(v) = &estado
                    && !v.notas.trim().is_empty()
                {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Qué trae:").strong());
                    ScrollArea::vertical().id_salt("notas").max_height(140.0).show(ui, |ui| {
                        for linea in v.notas.lines().take(60) {
                            let l = linea.trim_start_matches('#').trim();
                            if !l.is_empty() {
                                ui.add(egui::Label::new(RichText::new(l).color(p.texto_tenue)).wrap());
                            }
                        }
                    });
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| match &estado {
                    Estado::Disponible(v) => {
                        if ui_util::boton_acento(ui, p, &format!("{}  Actualizar ahora", ph::DOWNLOAD_SIMPLE)).clicked() {
                            acciones.push(Accion::Actualizar(v.clone()));
                        }
                        if ui.button("Revisar otra vez").clicked() {
                            acciones.push(Accion::BuscarActualizaciones);
                        }
                    }
                    Estado::Buscando => {}
                    Estado::Trabajando { .. } => {
                        if ui.button(format!("{}  Cancelar", ph::X)).clicked() {
                            acciones.push(Accion::CancelarActualizacion);
                        }
                    }
                    Estado::Listo { .. } | Estado::Permiso { .. } => {
                        if ui_util::boton_acento(ui, p, &format!("{}  Reiniciar Solar", ph::ARROW_CLOCKWISE)).clicked() {
                            acciones.push(Accion::Reiniciar);
                        }
                    }
                    _ => {
                        if ui_util::boton_acento(ui, p, &format!("{}  Buscar actualizaciones", ph::ARROWS_CLOCKWISE)).clicked() {
                            acciones.push(Accion::BuscarActualizaciones);
                        }
                    }
                });
            });

        ui.add_space(12.0);
        let mut cambio = false;
        fila(ui, p, "Buscar al abrir Solar", "Una vez al día, sin molestar: solo avisa si hay algo nuevo.", |ui| {
            cambio |= interruptor(ui, p, &mut self.config.buscar_actualizaciones).changed();
        });
        fila(ui, p, "Repositorio", "Usuario/repo de GitHub de donde salen las versiones.", |ui| {
            cambio |= ui.add(egui::TextEdit::singleline(&mut self.config.repo_actualizaciones).desired_width(190.0)).changed();
        });
        if cambio {
            self.config_cambio();
        }

        ui.add_space(14.0);
        ui_util::encabezado(ui, p, "Caché");
        let tamano = *self.tamano_cache.get_or_insert_with(actualizar::tamano_cache);
        let detalle = format!(
            "Descargas y compilaciones de actualizaciones en {}. Ahora ocupa {}. También olvida los íconos de programas que Solar ya cargó.",
            nucleo::contraer(&actualizar::dir_cache()),
            crate::formato::tamano(tamano)
        );
        fila(ui, p, "Borrar caché", &detalle, |ui| {
            if ui.button(format!("{}  Borrar caché", ph::BROOM)).clicked() {
                acciones.push(Accion::BorrarCache);
            }
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Para publicar una versión: sube la versión en Cargo.toml y crea un release en GitHub con su etiqueta (por ejemplo v0.2.0-alpha). \
                 Si le adjuntas el binario (por ejemplo solar-linux-x86_64.tar.gz) se descarga directo; si no, se compila.",
            )
            .small()
            .color(p.texto_tenue),
        );
    }

    // ------------------------------------------------------------------
    //  Acerca de
    // ------------------------------------------------------------------

    fn ajustes_acerca(&mut self, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(96.0, 96.0), Sense::hover());
            iconos::logo(ui.painter(), rect);
            ui.vertical(|ui| {
                ui.add_space(14.0);
                ui.label(RichText::new("Solar").size(26.0).strong());
                ui.label(RichText::new(format!("Versión {}", crate::actualizar::version_completa())).color(p.texto_tenue));
                ui.label(RichText::new("Núcleo en C · Interfaz en Rust · Configuración en Lua").small().color(p.texto_tenue));
                ui.label(RichText::new(format!("{}  Por ahora, exclusivo de Linux", ph::LINUX_LOGO)).small().color(p.texto_tenue));
            });
        });
        ui.add_space(12.0);
        ui_util::encabezado(ui, p, "Tus archivos de configuración");
        let rutas = [
            ("Configuración", self.rutas.config.clone(), false),
            ("Temas", self.rutas.temas.clone(), true),
            ("Carpetas personalizadas", nucleo::dir_datos().join("carpetas.tsv"), false),
        ];
        for (nombre, ruta, es_dir) in rutas {
            fila(ui, p, nombre, &ruta.display().to_string(), |ui| {
                if ui.button(if es_dir { "Abrir carpeta" } else { "Abrir" }).clicked() {
                    if es_dir {
                        acciones.push(Accion::Navegar(Ubicacion::Ruta(ruta.clone())));
                    } else if let Err(e) = nucleo::abrir(&ruta) {
                        self.avisar(e.mensaje, TipoAviso::Error);
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            if ui.button(format!("{}  Recargar configuración", ph::ARROW_CLOCKWISE)).clicked() {
                let ctx = ui.ctx().clone();
                self.recargar_config(&ctx);
            }
            if ui.button(format!("{}  Acerca de Solar", ph::INFO)).clicked() {
                acciones.push(Accion::AcercaDe);
            }
        });
        ui.add_space(12.0);
        ui_util::encabezado(ui, p, "Hacer un tema en Lua");
        let ejemplo = "-- ~/.config/solar/temas/bosque.lua\nreturn {\n  nombre = \"Bosque\",\n  base = \"venus\",       -- hereda lo que no pongas\n  colores = {\n    acento  = \"#22C55E\",\n    carpeta = \"#15803D\",\n  },\n}";
        Frame::new().fill(con_alfa(p.texto, 0.05)).corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(10)).show(ui, |ui| {
            ui.label(RichText::new(ejemplo).monospace().color(p.texto));
        });
    }
}

/// Una nota arriba de una página de Ajustes cuando el espacio de trabajo
/// tiene lo suyo.
fn aviso_espacio(ui: &mut Ui, p: &Paleta, texto: &str, botones: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(con_alfa(p.acento, 0.10))
        .stroke(egui::Stroke::new(1.0, con_alfa(p.acento, 0.45)))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(ph::PLANET).color(p.acento).size(16.0));
                ui.label(RichText::new(texto).color(p.texto));
            });
            ui.add_space(4.0);
            ui.horizontal(botones);
        });
    ui.add_space(10.0);
}
