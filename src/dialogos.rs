//! Diálogos: renombrar, crear, confirmar, conflictos, propiedades,
//! personalizar carpeta (color + logotipo), abrir con…, avisos y "Acerca de".
//! También las notificaciones y la ventanita de tareas.

use crate::app::{Accion, CambioEstilo, Solar, TipoAviso, Ubicacion};
use crate::iconos::{self, Logotipo};
use crate::nucleo::{self, Afinidad, Conflicto, Entrada, Espacio, EstiloCarpeta, Info, Programa, TipoEntrada};
use crate::tareas::{self, Conteo, TipoTarea};
use crate::tema::{self, Paleta, con_alfa};
use crate::ui_util;
use crate::sonido::{self, Sonido};
use crate::formato;
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Ui,
    vec2,
};
use egui_phosphor::regular as ph;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub enum Dialogo {
    Renombrar { ruta: PathBuf, nombre: String, error: Option<String>, iniciado: bool },
    Nuevo { carpeta: bool, nombre: String, error: Option<String>, iniciado: bool },
    Confirmar { rutas: Vec<PathBuf>, permanente: bool },
    ConfirmarBorrarPapelera(Vec<PathBuf>),
    ConfirmarVaciar,
    Conflicto { origenes: Vec<PathBuf>, destino: PathBuf, copiar: bool, nombres: Vec<String> },
    PapeleraFallo { rutas: Vec<PathBuf>, mensaje: String },
    Propiedades(Box<EstadoPropiedades>),
    Personalizar(Box<EstadoPersonalizar>),
    AbrirCon(Box<EstadoAbrirCon>),
    Mensaje { titulo: String, texto: String, error: bool },
    AvisosConfig(Vec<String>),
    AcercaDe,
}

// ======================================================================
//  Estados
// ======================================================================

pub struct EstadoAbrirCon {
    pub rutas: Vec<PathBuf>,
    pub programas: Vec<Programa>,
    pub mime: String,
    pub tipo: String,
    pub filtro: String,
    pub elegido: Option<usize>,
    pub ver_todos: bool,
    pub recordar: bool,
    pub comando: String,
    pub con_comando: bool,
    pub error: Option<String>,
}

impl EstadoAbrirCon {
    pub fn nuevo(rutas: Vec<PathBuf>, descripcion: String) -> Self {
        let (programas, mime, error) = match nucleo::programas_para(&rutas, true) {
            Ok(l) => (l.programas, l.mime, None),
            Err(e) => (Vec::new(), String::new(), Some(e.mensaje)),
        };
        let elegido = programas.first().filter(|p| p.afinidad > Afinidad::Otro).map(|_| 0);
        let hay_recomendados = elegido.is_some();
        Self {
            rutas,
            programas,
            mime,
            tipo: descripcion,
            filtro: String::new(),
            elegido,
            ver_todos: !hay_recomendados,
            recordar: false,
            comando: String::new(),
            con_comando: false,
            error,
        }
    }
}

/// Glifo para un programa sin ícono, según sus categorías.
pub fn glifo_programa(p: &Programa) -> &'static str {
    let c = p.categorias.as_str();
    let tiene = |x: &str| c.split(';').any(|k| k == x);
    if p.terminal || tiene("TerminalEmulator") {
        ph::TERMINAL_WINDOW
    } else if tiene("TextEditor") || tiene("IDE") || tiene("Development") {
        ph::CODE
    } else if tiene("WebBrowser") || tiene("Network") {
        ph::GLOBE
    } else if tiene("Graphics") || tiene("Photography") || tiene("2DGraphics") {
        ph::PAINT_BRUSH
    } else if tiene("Video") {
        ph::FILM_STRIP
    } else if tiene("Audio") || tiene("Music") {
        ph::MUSIC_NOTES
    } else if tiene("Office") || tiene("WordProcessor") || tiene("Viewer") {
        ph::FILE_TEXT
    } else if tiene("Archiving") || tiene("Compression") {
        ph::FILE_ZIP
    } else if tiene("Game") {
        ph::GAME_CONTROLLER
    } else if tiene("System") || tiene("Settings") {
        ph::GEAR_SIX
    } else {
        ph::APP_WINDOW
    }
}

pub struct EstadoPropiedades {
    pub rutas: Vec<PathBuf>,
    pub info: Option<Result<Info, String>>,
    pub conteo: Arc<Mutex<Conteo>>,
    pub cancelar: Arc<AtomicBool>,
    pub calcula: bool,
    pub espacio: Option<Espacio>,
    pub error: Option<String>,
}

impl EstadoPropiedades {
    pub fn nuevo(ctx: &egui::Context, rutas: Vec<PathBuf>) -> Self {
        let info = (rutas.len() == 1).then(|| nucleo::info(&rutas[0]).map_err(|e| e.mensaje));
        let calcula = rutas.len() > 1 || rutas.iter().any(|r| r.is_dir());
        let conteo = Arc::new(Mutex::new(Conteo::default()));
        let cancelar = Arc::new(AtomicBool::new(false));
        if calcula {
            tareas::calcular_tamano(rutas.clone(), conteo.clone(), cancelar.clone(), ctx.clone());
        }
        let espacio = rutas.first().and_then(|r| nucleo::espacio(r));
        Self { rutas, info, conteo, cancelar, calcula, espacio, error: None }
    }
}

impl Drop for EstadoPropiedades {
    fn drop(&mut self) {
        self.cancelar.store(true, Ordering::Relaxed);
    }
}

pub struct EstadoPersonalizar {
    pub rutas: Vec<PathBuf>,
    pub estilo: EstiloCarpeta,
    pub buscar: String,
    pub texto: String,
    pub hex_color: String,
    pub hex_logo: String,
}

impl EstadoPersonalizar {
    pub fn nuevo(rutas: Vec<PathBuf>, estilo: EstiloCarpeta) -> Self {
        let texto = estilo.icono.as_deref().and_then(|i| i.strip_prefix("txt:")).unwrap_or("").to_string();
        Self {
            hex_color: estilo.color.map(tema::hex).unwrap_or_default(),
            hex_logo: estilo.color_icono.map(tema::hex).unwrap_or_default(),
            rutas,
            estilo,
            buscar: String::new(),
            texto,
        }
    }
}

// ======================================================================
//  Botones
// ======================================================================

fn boton_principal(ui: &mut Ui, p: &Paleta, texto: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(texto).color(p.texto_acento).strong())
            .fill(p.acento)
            .min_size(vec2(96.0, 32.0)),
    )
}

fn boton_peligro(ui: &mut Ui, p: &Paleta, texto: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(texto).color(Color32::WHITE).strong())
            .fill(p.peligro)
            .min_size(vec2(96.0, 32.0)),
    )
}

/// Fila de botones alineados a la derecha (sin estirarse a lo alto).
fn fila_botones<R>(ui: &mut Ui, contenido: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| ui.with_layout(Layout::right_to_left(Align::Center), contenido).inner).inner
}

fn boton(ui: &mut Ui, texto: &str) -> egui::Response {
    ui.add(egui::Button::new(texto).min_size(vec2(88.0, 32.0)))
}

/// Explica por qué un nombre no sirve (o None si está bien).
fn problema_nombre(nombre: &str) -> Option<&'static str> {
    let n = nombre.trim();
    if n.is_empty() {
        return Some("Escribe un nombre.");
    }
    if nucleo::validar_nombre(n).is_err() {
        return Some(if n.contains('/') { "El nombre no puede llevar «/»." } else { "Ese nombre no se puede usar." });
    }
    if n.starts_with('.') {
        return Some("Empieza con punto: quedará oculto.");
    }
    None
}

fn nombre_o_cantidad(rutas: &[PathBuf]) -> String {
    if rutas.len() == 1 { format!("«{}»", tareas::nombre(&rutas[0])) } else { formato::elementos(rutas.len()) }
}

impl Solar {
    pub(crate) fn mostrar_dialogo(&mut self, ctx: &egui::Context, acciones: &mut Vec<Accion>) {
        let Some(mut d) = self.dialogo.take() else {
            self.dialogo_desde.0 = None;
            return;
        };
        let p = self.paleta.clone();
        let mut cerrar = false;
        // Entrada suave: aparece y sube un poquito
        let clase = std::mem::discriminant(&d);
        if self.dialogo_desde.0 != Some(clase) {
            self.dialogo_desde = (Some(clase), std::time::Instant::now());
        }
        let t = self.progreso_animacion(self.dialogo_desde.1, 0.2);
        if t < 1.0 {
            ctx.request_repaint();
        }
        ctx.set_transform_layer(
            egui::LayerId::new(egui::Order::Foreground, Id::new("dialogo")),
            egui::emath::TSTransform::from_translation(vec2(0.0, (1.0 - t) * 14.0)),
        );
        let ancho = match &d {
            Dialogo::Personalizar(_) => 680.0,
            Dialogo::AbrirCon(_) => 520.0,
            Dialogo::Propiedades(_) => 460.0,
            Dialogo::AcercaDe => 420.0,
            Dialogo::AvisosConfig(_) => 520.0,
            _ => 440.0,
        };
        let marco = Frame::new()
            .fill(p.elevado)
            .stroke(Stroke::new(1.0, p.borde))
            .corner_radius(CornerRadius::same(16))
            .inner_margin(Margin::same(22))
            .shadow(ctx.global_style().visuals.window_shadow)
            .multiply_with_opacity(t);
        let respuesta = egui::Modal::new(Id::new("dialogo"))
            .frame(marco)
            .backdrop_color(Color32::from_black_alpha((90.0 * t) as u8))
            .show(ctx, |ui| {
                ui.multiply_opacity(t);
                ui.set_width(ancho);
                match &mut d {
                    Dialogo::Renombrar { ruta, nombre, error, iniciado } => {
                        cerrar = self.dialogo_renombrar(ui, &p, ruta, nombre, error, iniciado);
                    }
                    Dialogo::Nuevo { carpeta, nombre, error, iniciado } => {
                        cerrar = self.dialogo_nuevo(ui, &p, *carpeta, nombre, error, iniciado);
                    }
                    Dialogo::Confirmar { rutas, permanente } => {
                        ui_util::titulo(ui, &p, if *permanente { "¿Eliminar para siempre?" } else { "¿Mover a la papelera?" });
                        let texto = if *permanente {
                            format!("Se eliminará {} y no se podrá recuperar.", nombre_o_cantidad(rutas))
                        } else {
                            format!("{} se moverá a la papelera. Podrás restaurarlo después.", nombre_o_cantidad(rutas))
                        };
                        ui.label(texto);
                        ui.add_space(14.0);
                        fila_botones(ui, |ui| {
                            let ok = if *permanente {
                                boton_peligro(ui, &p, "Eliminar para siempre")
                            } else {
                                boton_principal(ui, &p, "Mover a la papelera")
                            };
                            if ok.clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                let t = if *permanente {
                                    TipoTarea::Eliminar { rutas: rutas.clone() }
                                } else {
                                    TipoTarea::Papelera { rutas: rutas.clone() }
                                };
                                acciones.push(Accion::Tarea(t));
                                cerrar = true;
                            }
                            if boton(ui, "Cancelar").clicked() {
                                cerrar = true;
                            }
                        });
                    }
                    Dialogo::ConfirmarBorrarPapelera(rutas) => {
                        ui_util::titulo(ui, &p, "¿Eliminar para siempre?");
                        ui.label(format!("Se eliminará {} de la papelera y no se podrá recuperar.", nombre_o_cantidad(rutas)));
                        ui.add_space(14.0);
                        fila_botones(ui, |ui| {
                            if boton_peligro(ui, &p, "Eliminar para siempre").clicked() {
                                acciones.push(Accion::Tarea(TipoTarea::BorrarDePapelera { rutas: rutas.clone() }));
                                cerrar = true;
                            }
                            if boton(ui, "Cancelar").clicked() {
                                cerrar = true;
                            }
                        });
                    }
                    Dialogo::ConfirmarVaciar => {
                        ui_util::titulo(ui, &p, "¿Vaciar la papelera?");
                        ui.label("Todo lo que está en la papelera (también la de tus memorias USB) se eliminará para siempre.");
                        ui.add_space(14.0);
                        fila_botones(ui, |ui| {
                            if boton_peligro(ui, &p, "Vaciar papelera").clicked() {
                                acciones.push(Accion::Tarea(TipoTarea::VaciarPapelera));
                                cerrar = true;
                            }
                            if boton(ui, "Cancelar").clicked() {
                                cerrar = true;
                            }
                        });
                    }
                    Dialogo::Conflicto { origenes, destino, copiar, nombres } => {
                        let titulo = if nombres.len() == 1 {
                            format!("«{}» ya existe", nombres[0])
                        } else {
                            format!("{} ya existen", formato::elementos(nombres.len()))
                        };
                        ui_util::titulo(ui, &p, &titulo);
                        ui.label(if nombres.len() == 1 {
                            format!("Ya hay algo con ese nombre en «{}». ¿Qué hacemos?", tareas::nombre(destino))
                        } else {
                            format!("En «{}» ya hay elementos con esos nombres. ¿Qué hacemos?", tareas::nombre(destino))
                        });
                        if nombres.len() > 1 {
                            ui.add_space(4.0);
                            for n in nombres.iter().take(5) {
                                ui.label(RichText::new(format!("• {n}")).color(p.texto_tenue));
                            }
                            if nombres.len() > 5 {
                                ui.label(RichText::new(format!("…y {} más", nombres.len() - 5)).color(p.texto_tenue));
                            }
                        }
                        ui.add_space(4.0);
                        ui.label(RichText::new("Reemplazar combina las carpetas y reemplaza los archivos repetidos.").small().color(p.texto_tenue));
                        ui.add_space(14.0);
                        let mut elegido = None;
                        fila_botones(ui, |ui| {
                            if boton_principal(ui, &p, "Conservar ambos").clicked() {
                                elegido = Some(Conflicto::Renombrar);
                            }
                            if boton(ui, "Reemplazar").clicked() {
                                elegido = Some(Conflicto::Reemplazar);
                            }
                            if boton(ui, "Omitir").clicked() {
                                elegido = Some(Conflicto::Omitir);
                            }
                            if boton(ui, "Cancelar").clicked() {
                                cerrar = true;
                            }
                        });
                        if let Some(conflicto) = elegido {
                            let (origenes, destino) = (origenes.clone(), destino.clone());
                            acciones.push(Accion::Tarea(if *copiar {
                                TipoTarea::Copiar { origenes, destino, conflicto }
                            } else {
                                TipoTarea::Mover { origenes, destino, conflicto }
                            }));
                            cerrar = true;
                        }
                    }
                    Dialogo::PapeleraFallo { rutas, mensaje } => {
                        ui_util::titulo(ui, &p, "No se pudo usar la papelera");
                        ui.label(mensaje.as_str());
                        ui.add_space(14.0);
                        fila_botones(ui, |ui| {
                            if boton_peligro(ui, &p, "Eliminar para siempre").clicked() {
                                acciones.push(Accion::Tarea(TipoTarea::Eliminar { rutas: rutas.clone() }));
                                cerrar = true;
                            }
                            if boton(ui, "Cancelar").clicked() {
                                cerrar = true;
                            }
                        });
                    }
                    Dialogo::Propiedades(e) => cerrar = self.dialogo_propiedades(ui, &p, e),
                    Dialogo::Personalizar(e) => cerrar = self.dialogo_personalizar(ui, &p, e),
                    Dialogo::AbrirCon(e) => cerrar = self.dialogo_abrir_con(ui, &p, e),
                    Dialogo::Mensaje { titulo, texto, error } => {
                        ui.horizontal(|ui| {
                            let (g, c) = if *error { (ph::WARNING_CIRCLE, p.peligro) } else { (ph::INFO, p.acento) };
                            ui.label(RichText::new(g).size(26.0).color(c));
                            ui_util::titulo(ui, &p, titulo);
                        });
                        ui.label(texto.as_str());
                        ui.add_space(14.0);
                        fila_botones(ui, |ui| {
                            if boton_principal(ui, &p, "Entendido").clicked() {
                                cerrar = true;
                            }
                        });
                    }
                    Dialogo::AvisosConfig(avisos) => {
                        ui_util::titulo(ui, &p, "Revisé tu configuración");
                        ui.label("Encontré algunas cosas que no entendí; las ignoré y usé los valores por defecto:");
                        ui.add_space(6.0);
                        ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                            for a in avisos.iter() {
                                for linea in a.lines().filter(|l| !l.trim().is_empty()) {
                                    ui.label(RichText::new(format!("• {linea}")).color(p.texto_tenue));
                                }
                            }
                        });
                        ui.add_space(14.0);
                        fila_botones(ui, |ui| {
                            if boton_principal(ui, &p, "Entendido").clicked() {
                                cerrar = true;
                            }
                            if boton(ui, "Abrir config.lua").clicked()
                                && let Err(e) = nucleo::abrir(&self.rutas.config)
                            {
                                self.avisar(e.mensaje, TipoAviso::Error);
                            }
                        });
                    }
                    Dialogo::AcercaDe => cerrar = self.dialogo_acerca(ui, &p),
                }
            });
        if respuesta.should_close() {
            cerrar = true;
        }
        if cerrar {
            sonido::pedir(Sonido::Retraccion);
        }
        if !cerrar && self.dialogo.is_none() {
            self.dialogo = Some(d);
        }
    }

    fn campo_nombre(ui: &mut Ui, nombre: &mut String, iniciado: &mut bool, es_carpeta: bool) -> egui::Response {
        let resp = ui.add(egui::TextEdit::singleline(nombre).desired_width(f32::INFINITY).font(FontId::proportional(15.0)));
        if let Some(problema) = problema_nombre(nombre) {
            ui.label(RichText::new(problema).small().color(ui.visuals().warn_fg_color));
        }
        if !*iniciado {
            resp.request_focus();
            // Selecciona el nombre sin la extensión
            let total = nombre.chars().count();
            let fin = if es_carpeta {
                total
            } else {
                nombre.rfind('.').filter(|&i| i > 0).map(|i| nombre[..i].chars().count()).unwrap_or(total)
            };
            if let Some(mut estado) = egui::TextEdit::load_state(ui.ctx(), resp.id) {
                estado.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(fin),
                )));
                estado.store(ui.ctx(), resp.id);
            }
            *iniciado = true;
        }
        resp
    }

    fn dialogo_renombrar(
        &mut self,
        ui: &mut Ui,
        p: &Paleta,
        ruta: &PathBuf,
        nombre: &mut String,
        error: &mut Option<String>,
        iniciado: &mut bool,
    ) -> bool {
        let es_carpeta = ruta.is_dir();
        ui_util::titulo(ui, p, if es_carpeta { "Renombrar carpeta" } else { "Renombrar archivo" });
        let resp = Self::campo_nombre(ui, nombre, iniciado, es_carpeta);
        if let Some(e) = error {
            ui.label(RichText::new(e.as_str()).color(p.peligro));
        }
        ui.add_space(14.0);
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let mut cerrar = false;
        let mut confirmar = enter;
        fila_botones(ui, |ui| {
            if boton_principal(ui, p, "Renombrar").clicked() {
                confirmar = true;
            }
            if boton(ui, "Cancelar").clicked() {
                cerrar = true;
            }
        });
        if confirmar && nucleo::validar_nombre(nombre.trim()).is_ok() {
            let nuevo = nombre.trim().to_string();
            if nuevo == tareas::nombre(ruta) {
                return true;
            }
            match nucleo::renombrar(ruta, &nuevo) {
                Ok(nueva) => {
                    if es_carpeta && let Ok(mut db) = self.carpetas.lock() {
                        db.renombrar(ruta, &nueva);
                    }
                    if let Some(l) = self.config.lugares.iter_mut().find(|l| nucleo::expandir(&l.ruta) == *ruta) {
                        l.ruta = nucleo::contraer(&nueva);
                        self.config_cambio();
                    }
                    self.seleccionar_al_cargar = Some(vec![nueva]);
                    self.cargar(ui.ctx(), false);
                    cerrar = true;
                }
                Err(e) => {
                    *error = Some(if e.codigo == 17 { format!("Ya existe algo llamado «{nuevo}».") } else { e.mensaje });
                    *iniciado = true;
                }
            }
        }
        cerrar
    }

    fn dialogo_nuevo(
        &mut self,
        ui: &mut Ui,
        p: &Paleta,
        carpeta: bool,
        nombre: &mut String,
        error: &mut Option<String>,
        iniciado: &mut bool,
    ) -> bool {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
            if carpeta {
                iconos::carpeta(ui.painter(), rect, p.carpeta, Logotipo::Ninguno, None, 1.0);
            } else {
                iconos::archivo(ui.painter(), rect, nucleo::Categoria::Texto, nombre, false, 1.0);
            }
            ui_util::titulo(ui, p, if carpeta { "Nueva carpeta" } else { "Nuevo archivo" });
        });
        let resp = Self::campo_nombre(ui, nombre, iniciado, carpeta);
        if let Some(e) = error {
            ui.label(RichText::new(e.as_str()).color(p.peligro));
        }
        ui.add_space(14.0);
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let mut cerrar = false;
        let mut confirmar = enter;
        fila_botones(ui, |ui| {
            if boton_principal(ui, p, "Crear").clicked() {
                confirmar = true;
            }
            if boton(ui, "Cancelar").clicked() {
                cerrar = true;
            }
        });
        if confirmar
            && nucleo::validar_nombre(nombre.trim()).is_ok()
            && let Some(dir) = self.ubicacion.ruta().map(PathBuf::from)
        {
            let n = nombre.trim().to_string();
            let r = if carpeta { nucleo::crear_carpeta(&dir, &n) } else { nucleo::crear_archivo(&dir, &n) };
            match r {
                Ok(ruta) => {
                    self.seleccionar_al_cargar = Some(vec![ruta]);
                    self.cargar(ui.ctx(), false);
                    cerrar = true;
                }
                Err(e) => {
                    *error = Some(if e.codigo == 17 { format!("Ya existe algo llamado «{n}».") } else { e.mensaje });
                }
            }
        }
        cerrar
    }

    fn entrada_para(&self, ruta: &std::path::Path, info: Option<&Info>) -> Entrada {
        if let Some(e) = self.entrada(ruta) {
            return e.clone();
        }
        let nombre = tareas::nombre(ruta);
        let (tipo, categoria) = info.map(|i| (i.tipo, i.categoria)).unwrap_or((TipoEntrada::Carpeta, nucleo::Categoria::Carpeta));
        Entrada {
            plegado: String::new(),
            nombre: nombre.clone(),
            ruta: ruta.to_path_buf(),
            tamano: info.map(|i| i.tamano).unwrap_or(0),
            modificado: info.map(|i| i.modificado).unwrap_or(0),
            modo: info.map(|i| i.modo).unwrap_or(0),
            tipo,
            categoria,
            es_enlace: info.is_some_and(|i| i.es_enlace),
            es_oculto: nombre.starts_with('.'),
            descripcion: info.map(|i| i.descripcion.clone()).unwrap_or_default(),
            estilo: self.estilo_de(ruta),
        }
    }

    fn dialogo_propiedades(&mut self, ui: &mut Ui, p: &Paleta, e: &mut EstadoPropiedades) -> bool {
        let conteo = e.conteo.lock().map(|c| *c).unwrap_or_default();
        let fila = |ui: &mut Ui, etiqueta: &str, valor: String| {
            ui.label(RichText::new(etiqueta).color(p.texto_tenue));
            ui.add(egui::Label::new(RichText::new(valor).color(p.texto)).wrap());
            ui.end_row();
        };
        let info = e.info.clone();
        ui.horizontal(|ui| {
            let primera = &e.rutas[0];
            let entrada = self.entrada_para(primera, info.as_ref().and_then(|r| r.as_ref().ok()));
            crate::vista::icono_entrada(self, ui, &entrada, 64.0);
            ui.vertical(|ui| {
                let titulo = if e.rutas.len() == 1 { entrada.nombre.clone() } else { formato::elementos(e.rutas.len()) };
                ui.add(egui::Label::new(RichText::new(titulo).size(18.0).strong().color(p.texto)).wrap());
                let sub = if e.rutas.len() == 1 { entrada.descripcion.clone() } else { "Selección múltiple".into() };
                ui.label(RichText::new(sub).color(p.texto_tenue));
            });
        });
        ui.add_space(12.0);
        let mut nuevo_modo = None;
        egui::Grid::new("propiedades").num_columns(2).spacing(vec2(14.0, 8.0)).show(ui, |ui| {
            if let Some(dir) = e.rutas[0].parent() {
                fila(ui, "Ubicación", nucleo::contraer(dir));
            }
            let tamano = match (&info, e.calcula) {
                (_, true) => {
                    let mut t = formato::tamano(conteo.bytes);
                    if !conteo.terminado {
                        t.push_str("  (calculando…)");
                    }
                    t
                }
                (Some(Ok(i)), false) if i.tamano >= 1000 => {
                    format!("{} ({} bytes)", formato::tamano(i.tamano), formato::miles(i.tamano))
                }
                (Some(Ok(i)), false) => formato::tamano(i.tamano),
                _ => "—".into(),
            };
            fila(ui, "Tamaño", tamano);
            if e.calcula {
                fila(ui, "Contenido", format!("{} archivos, {} carpetas", formato::miles(conteo.archivos), formato::miles(conteo.carpetas.saturating_sub(e.rutas.iter().filter(|r| r.is_dir()).count() as u64))));
            }
            match &info {
                Some(Ok(i)) => {
                    if let Some(d) = &i.destino_enlace {
                        fila(ui, "Enlace a", d.clone());
                    }
                    fila(ui, "Modificado", formato::fecha_completa(i.modificado));
                    fila(ui, "Accedido", formato::fecha_completa(i.accedido));
                    if let Some(c) = i.creado {
                        fila(ui, "Creado", formato::fecha_completa(c));
                    }
                    fila(ui, "Propietario", format!("{} (grupo {})", i.propietario, i.grupo));
                    ui.label(RichText::new("Permisos").color(p.texto_tenue));
                    let mut modo = i.modo & 0o777;
                    let antes = modo;
                    egui::Grid::new("permisos").num_columns(4).spacing(vec2(10.0, 4.0)).show(ui, |ui| {
                        ui.label("");
                        ui.label(RichText::new("Leer").small().color(p.texto_tenue));
                        ui.label(RichText::new("Escribir").small().color(p.texto_tenue));
                        ui.label(RichText::new(if i.tipo == TipoEntrada::Carpeta { "Entrar" } else { "Ejecutar" }).small().color(p.texto_tenue));
                        ui.end_row();
                        for (nombre, desp) in [("Tú", 6), ("Grupo", 3), ("Otros", 0)] {
                            ui.label(RichText::new(nombre).small());
                            for bit in [4u32, 2, 1] {
                                let mascara = bit << desp;
                                let mut activo = modo & mascara != 0;
                                if ui.checkbox(&mut activo, "").changed() {
                                    if activo { modo |= mascara } else { modo &= !mascara }
                                }
                            }
                            ui.end_row();
                        }
                    });
                    ui.end_row();
                    ui.label("");
                    ui.label(RichText::new(format!("{}  ({:o})", formato::permisos(modo), modo)).monospace().color(p.texto_tenue));
                    ui.end_row();
                    if modo != antes {
                        nuevo_modo = Some((i.modo & !0o777) | modo);
                    }
                }
                Some(Err(err)) => fila(ui, "Error", err.clone()),
                None => {}
            }
            if let Some(esp) = e.espacio {
                fila(ui, "Espacio libre", format!("{} de {}", formato::tamano(esp.disponible), formato::tamano(esp.total)));
            }
        });
        if let Some(m) = nuevo_modo {
            match nucleo::cambiar_permisos(&e.rutas[0], m) {
                Ok(()) => {
                    e.info = Some(nucleo::info(&e.rutas[0]).map_err(|x| x.mensaje));
                    e.error = None;
                }
                Err(x) => e.error = Some(x.mensaje),
            }
        }
        if let Some(err) = &e.error {
            ui.label(RichText::new(err.as_str()).color(p.peligro));
        }
        if e.calcula && !conteo.terminado {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        ui.add_space(14.0);
        let mut cerrar = false;
        fila_botones(ui, |ui| {
            if boton_principal(ui, p, "Cerrar").clicked() {
                cerrar = true;
            }
            let carpetas: Vec<PathBuf> = e.rutas.iter().filter(|r| r.is_dir()).cloned().collect();
            if carpetas.len() == e.rutas.len() && self.ubicacion != Ubicacion::Papelera && boton(ui, &format!("{}  Personalizar", ph::SPARKLE)).clicked() {
                let estilo = self.estilo_de(&carpetas[0]).unwrap_or_default();
                self.dialogo = Some(Dialogo::Personalizar(Box::new(EstadoPersonalizar::nuevo(carpetas, estilo))));
                cerrar = true;
            }
        });
        cerrar
    }

    fn dialogo_personalizar(&mut self, ui: &mut Ui, p: &Paleta, e: &mut EstadoPersonalizar) -> bool {
        let titulo = if e.rutas.len() == 1 {
            format!("Personalizar «{}»", tareas::nombre(&e.rutas[0]))
        } else {
            format!("Personalizar {} carpetas", e.rutas.len())
        };
        ui_util::titulo(ui, p, &titulo);
        let mut cerrar = false;
        ui.horizontal_top(|ui| {
            // Vista previa
            ui.vertical(|ui| {
                ui.set_width(190.0);
                Frame::new().fill(p.fondo).corner_radius(CornerRadius::same(14)).inner_margin(Margin::same(14)).show(ui, |ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(162.0, 162.0), Sense::hover());
                    let color = e.estilo.color.unwrap_or(p.carpeta);
                    iconos::carpeta(ui.painter(), rect, color, Logotipo::de(e.estilo.icono.as_deref()), e.estilo.color_icono, 1.0);
                    let (r2, _) = ui.allocate_exact_size(vec2(162.0, 40.0), Sense::hover());
                    let pequenos = [24.0, 32.0, 48.0];
                    let mut x = r2.left() + 10.0;
                    for t in pequenos {
                        let rr = egui::Rect::from_min_size(egui::pos2(x, r2.bottom() - t), vec2(t, t));
                        iconos::carpeta(ui.painter(), rr, color, Logotipo::de(e.estilo.icono.as_deref()), e.estilo.color_icono, 1.0);
                        x += t + 14.0;
                    }
                });
                ui.add_space(8.0);
                let desc_color = e.estilo.color.map(tema::nombre_color).unwrap_or_else(|| "Color del tema".into());
                let mut desc_logo = match e.estilo.icono.as_deref() {
                    None => "Sin logotipo".to_string(),
                    Some(i) if i.starts_with("txt:") => format!("Texto «{}»", &i[4..]),
                    Some(i) => iconos::emblema(i).map(|x| x.nombre.to_string()).unwrap_or_else(|| i.to_string()),
                };
                if let (Some(_), Some(c)) = (&e.estilo.icono, e.estilo.color_icono) {
                    desc_logo.push_str(&format!(" · {}", tema::nombre_color(c)));
                }
                ui.label(RichText::new(desc_color).small().color(p.texto_tenue));
                ui.label(RichText::new(desc_logo).small().color(p.texto_tenue));
            });
            ui.add_space(10.0);
            ui.vertical(|ui| {
                // Que quepa completo en pantallas de 768 px (como la X220)
                let alto = (ui.ctx().content_rect().height() - 250.0).clamp(240.0, 560.0);
                ScrollArea::vertical().max_height(alto).show(ui, |ui| {
                    ui_util::encabezado(ui, p, "Color de la carpeta");
                    if let Some(c) = ui_util::rejilla_colores(ui, p, e.estilo.color, 11, 11.0) {
                        e.estilo.color = Some(c);
                        e.hex_color = tema::hex(c);
                    }
                    ui.horizontal(|ui| {
                        if ui.selectable_label(e.estilo.color.is_none(), "Del tema").clicked() {
                            e.estilo.color = None;
                            e.hex_color.clear();
                        }
                        ui.label(RichText::new("Personalizado:").color(p.texto_tenue));
                        let mut c = e.estilo.color.unwrap_or(p.carpeta);
                        if egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::Opaque).changed() {
                            e.estilo.color = Some(c);
                            e.hex_color = tema::hex(c);
                        }
                        let r = ui.add(egui::TextEdit::singleline(&mut e.hex_color).desired_width(76.0).hint_text("#F97316"));
                        if r.changed() && let Some(c) = tema::de_hex(&e.hex_color) {
                            e.estilo.color = Some(c);
                        }
                    });
                    ui.add_space(4.0);
                    ui_util::encabezado(ui, p, "Logotipo");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut e.buscar).hint_text(format!("{}  Buscar (mando, música…)", ph::MAGNIFYING_GLASS)).desired_width(220.0));
                        if ui.selectable_label(e.estilo.icono.is_none(), "Sin logotipo").clicked() {
                            e.estilo.icono = None;
                        }
                    });
                    // Tres filas a la vista; el resto con la rueda o buscando
                    let filas = 3.0 * 36.0 + 2.0 * ui.spacing().item_spacing.y;
                    ScrollArea::vertical().id_salt("logos").max_height(filas).show(ui, |ui| {
                        if let Some(id) = ui_util::rejilla_logotipos(ui, p, e.estilo.icono.as_deref(), &e.buscar, 11, 36.0) {
                            e.estilo.icono = Some(id.to_string());
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("O escribe un texto o emoji:").color(p.texto_tenue));
                        let r = ui.add(egui::TextEdit::singleline(&mut e.texto).desired_width(70.0).char_limit(4).hint_text("★"));
                        if r.changed() {
                            e.estilo.icono = if e.texto.trim().is_empty() { None } else { Some(format!("txt:{}", e.texto.trim())) };
                        }
                    });
                    ui.add_space(4.0);
                    ui_util::encabezado(ui, p, "Color del logotipo");
                    if let Some(c) = ui_util::rejilla_colores(ui, p, e.estilo.color_icono, 11, 11.0) {
                        e.estilo.color_icono = Some(c);
                        e.hex_logo = tema::hex(c);
                    }
                    ui.horizontal(|ui| {
                        if ui.selectable_label(e.estilo.color_icono.is_none(), "Automático").clicked() {
                            e.estilo.color_icono = None;
                            e.hex_logo.clear();
                        }
                        ui.label(RichText::new("Personalizado:").color(p.texto_tenue));
                        let base = e.estilo.color.unwrap_or(p.carpeta);
                        let mut c = e.estilo.color_icono.unwrap_or(tema::paleta_carpeta(base).logotipo);
                        if egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::Opaque).changed() {
                            e.estilo.color_icono = Some(c);
                            e.hex_logo = tema::hex(c);
                        }
                        let r = ui.add(egui::TextEdit::singleline(&mut e.hex_logo).desired_width(76.0).hint_text("#14532D"));
                        if r.changed() && let Some(c) = tema::de_hex(&e.hex_logo) {
                            e.estilo.color_icono = Some(c);
                        }
                    });
                });
            });
        });
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if ui.button(format!("{}  Quitar personalización", ph::ARROW_COUNTER_CLOCKWISE)).clicked() {
                e.estilo = EstiloCarpeta::default();
                e.texto.clear();
                e.hex_color.clear();
                e.hex_logo.clear();
            }
            fila_botones(ui, |ui| {
                if boton_principal(ui, p, "Aplicar").clicked() {
                    let rutas = e.rutas.clone();
                    let estilo = e.estilo.clone();
                    self.poner_estilo(&rutas, &estilo);
                    cerrar = true;
                }
                if boton(ui, "Cancelar").clicked() {
                    cerrar = true;
                }
            });
        });
        cerrar
    }

    fn dialogo_abrir_con(&mut self, ui: &mut Ui, p: &Paleta, e: &mut EstadoAbrirCon) -> bool {
        let mut cerrar = false;
        ui_util::titulo(ui, p, &format!("Abrir {} con…", nombre_o_cantidad(&e.rutas)));
        if !e.tipo.is_empty() || !e.mime.is_empty() {
            let detalle = match (e.tipo.is_empty(), e.mime.is_empty()) {
                (false, false) => format!("{} · {}", e.tipo, e.mime),
                (false, true) => e.tipo.clone(),
                _ => e.mime.clone(),
            };
            ui.label(RichText::new(detalle).small().color(p.texto_tenue));
            ui.add_space(4.0);
        }
        let r = ui.add(
            egui::TextEdit::singleline(&mut e.filtro)
                .hint_text(format!("{}  Buscar programa…", ph::MAGNIFYING_GLASS))
                .desired_width(f32::INFINITY),
        );
        if r.changed() {
            e.elegido = None;
        }
        ui.add_space(6.0);

        let filtro = formato::plegar(e.filtro.trim());
        let visibles: Vec<usize> = e
            .programas
            .iter()
            .enumerate()
            .filter(|(_, x)| {
                if filtro.is_empty() {
                    e.ver_todos || x.afinidad > Afinidad::Otro
                } else {
                    formato::plegar(&x.nombre).contains(&filtro) || formato::plegar(&x.descripcion).contains(&filtro)
                }
            })
            .map(|(i, _)| i)
            .collect();
        if e.elegido.is_none_or(|i| !visibles.contains(&i)) {
            e.elegido = visibles.first().copied();
        }
        let mut abrir: Option<usize> = None;
        ScrollArea::vertical().max_height(300.0).auto_shrink([false, true]).show(ui, |ui| {
            if visibles.is_empty() {
                ui.add_space(10.0);
                let t = if e.programas.is_empty() {
                    "No encontré programas instalados (lanzadores .desktop). Puedes escribir un comando abajo."
                } else if !filtro.is_empty() {
                    "Ningún programa se llama así."
                } else {
                    "Ningún programa dice abrir este tipo de archivo. Mira todos los programas o escribe un comando."
                };
                ui.label(RichText::new(t).color(p.texto_tenue));
                ui.add_space(10.0);
            }
            let mut seccion_previa: Option<u8> = None;
            for &i in &visibles {
                let prog = &e.programas[i];
                let seccion = match prog.afinidad {
                    Afinidad::Predeterminado | Afinidad::Exacta => 0,
                    Afinidad::Padre => 1,
                    Afinidad::Otro => 2,
                };
                if filtro.is_empty() && seccion_previa != Some(seccion) {
                    let titulo = match seccion {
                        0 => "Recomendados",
                        1 => "También pueden abrirlo",
                        _ => "Otros programas",
                    };
                    if seccion_previa.is_some() {
                        ui.add_space(4.0);
                    }
                    ui_util::encabezado(ui, p, titulo);
                    seccion_previa = Some(seccion);
                }
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
                let sel = e.elegido == Some(i);
                let h = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered() || sel, 0.12);
                if sel {
                    ui.painter().rect(rect, CornerRadius::same(10), p.seleccion, Stroke::new(1.0, p.seleccion_borde), egui::StrokeKind::Inside);
                } else if h > 0.0 {
                    ui.painter().rect_filled(rect, CornerRadius::same(10), con_alfa(p.hover, h * (p.hover.a() as f32 / 255.0)));
                }
                let caja_icono = egui::Rect::from_center_size(rect.left_center() + vec2(26.0, 0.0), vec2(30.0, 30.0));
                let textura = self.iconos_programas.borrow_mut().textura(ui.ctx(), &prog.icono);
                match textura {
                    Some(t) => {
                        egui::Image::new(&t).paint_at(ui, caja_icono);
                    }
                    None => {
                        ui.painter().rect_filled(caja_icono, CornerRadius::same(8), con_alfa(p.acento, 0.16));
                        ui.painter().text(caja_icono.center(), Align2::CENTER_CENTER, glifo_programa(prog), FontId::proportional(18.0), p.acento);
                    }
                }
                let x = caja_icono.right() + 12.0;
                let mut y = rect.center().y;
                let tiene_sub = !prog.descripcion.is_empty() || prog.terminal;
                if tiene_sub {
                    y -= 8.0;
                }
                let g = ui.painter().layout_no_wrap(prog.nombre.clone(), FontId::proportional(14.5), p.texto);
                let ancho_nombre = g.size().x;
                ui.painter().galley(egui::pos2(x, y - g.size().y / 2.0), g, p.texto);
                if prog.afinidad == Afinidad::Predeterminado {
                    let g = ui.painter().layout_no_wrap("Predeterminado".into(), FontId::proportional(11.0), p.acento);
                    let caja = egui::Rect::from_min_size(egui::pos2(x + ancho_nombre + 8.0, y - 9.0), g.size() + vec2(12.0, 6.0));
                    ui.painter().rect_filled(caja, CornerRadius::same(9), con_alfa(p.acento, 0.15));
                    ui.painter().galley(caja.min + vec2(6.0, 3.0), g, p.acento);
                }
                if tiene_sub {
                    let mut sub = prog.descripcion.clone();
                    if prog.terminal {
                        if !sub.is_empty() {
                            sub.push_str(" · ");
                        }
                        sub.push_str("se abre en la terminal");
                    }
                    let g = ui.painter().layout(sub, FontId::proportional(12.0), p.texto_tenue, rect.right() - x - 10.0);
                    ui.painter().galley(egui::pos2(x, y + 9.0), g, p.texto_tenue);
                }
                let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
                if resp.clicked() {
                    e.elegido = Some(i);
                    sonido::pedir(Sonido::Clic);
                }
                if resp.double_clicked() {
                    abrir = Some(i);
                }
            }
        });

        // Teclado: flechas para elegir, Enter para abrir
        if !e.con_comando && !visibles.is_empty() {
            let pos = e.elegido.and_then(|i| visibles.iter().position(|&v| v == i)).unwrap_or(0);
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                e.elegido = Some(visibles[(pos + 1).min(visibles.len() - 1)]);
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                e.elegido = Some(visibles[pos.saturating_sub(1)]);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                abrir = e.elegido;
            }
        }

        ui.add_space(6.0);
        let otros = e.programas.iter().filter(|x| x.afinidad == Afinidad::Otro).count();
        ui.horizontal(|ui| {
            if otros > 0 && e.filtro.trim().is_empty() {
                let texto = if e.ver_todos { "Solo los recomendados".to_string() } else { format!("Ver todos los programas ({otros} más)") };
                if ui.link(texto).clicked() {
                    e.ver_todos = !e.ver_todos;
                }
                ui.label(RichText::new("·").color(p.texto_tenue));
            }
            if ui.link(if e.con_comando { "Elegir de la lista" } else { "Usar un comando…" }).clicked() {
                e.con_comando = !e.con_comando;
            }
        });
        if e.con_comando {
            ui.add_space(4.0);
            let r = ui.add(
                egui::TextEdit::singleline(&mut e.comando)
                    .hint_text("Programa o comando, por ejemplo: mpv --fs")
                    .desired_width(f32::INFINITY),
            );
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !e.comando.trim().is_empty() {
                abrir = Some(usize::MAX);
            }
        }
        let un_tipo = !e.mime.is_empty() && e.mime != "application/octet-stream";
        let extension = e
            .rutas
            .first()
            .and_then(|r| r.extension())
            .map(|x| format!("los archivos .{}", x.to_string_lossy()))
            .unwrap_or_else(|| "los archivos de este tipo".into());
        if un_tipo && !e.con_comando {
            ui.add_space(4.0);
            ui.checkbox(&mut e.recordar, format!("Usar siempre este programa para {extension}"));
        }
        if let Some(err) = &e.error {
            ui.label(RichText::new(err.as_str()).color(p.peligro));
        }
        ui.add_space(12.0);
        fila_botones(ui, |ui| {
            let listo = if e.con_comando { !e.comando.trim().is_empty() } else { e.elegido.is_some() };
            if ui
                .add_enabled(listo, egui::Button::new(RichText::new("Abrir").color(p.texto_acento)).fill(p.acento).min_size(vec2(96.0, 32.0)))
                .clicked()
            {
                abrir = if e.con_comando { Some(usize::MAX) } else { e.elegido };
            }
            if boton(ui, "Cancelar").clicked() {
                cerrar = true;
            }
        });

        if let Some(i) = abrir {
            let resultado = if i == usize::MAX {
                nucleo::abrir_con(e.comando.trim(), &e.rutas)
            } else {
                let prog = e.programas[i].clone();
                let r = nucleo::programa_lanzar(&prog, &e.rutas, &self.config.terminal);
                if r.is_ok() && e.recordar && un_tipo {
                    match nucleo::programa_predeterminar(&prog.id, &e.mime) {
                        Ok(()) => self.avisar(format!("Desde ahora, {extension} se abrirán con {}.", prog.nombre), TipoAviso::Exito),
                        Err(x) => self.avisar(format!("No se pudo guardar el predeterminado: {}", x.mensaje), TipoAviso::Error),
                    }
                }
                r
            };
            match resultado {
                Ok(()) => cerrar = true,
                Err(x) => e.error = Some(x.mensaje),
            }
        }
        cerrar
    }

    fn dialogo_acerca(&mut self, ui: &mut Ui, p: &Paleta) -> bool {
        let mut cerrar = false;
        ui.vertical_centered(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(132.0, 132.0), Sense::hover());
            iconos::logo(ui.painter(), rect);
            ui.label(RichText::new("Solar").size(28.0).strong().color(p.texto));
            ui.label(RichText::new(format!("Versión {}", nucleo::version())).color(p.texto_tenue));
            ui.add_space(6.0);
            ui.label(RichText::new("Gestor de archivos").color(p.texto));
            ui.label(
                RichText::new(format!("{} Núcleo en C  ·  {} Interfaz en Rust  ·  {} Configuración en Lua", ph::CPU, ph::GEAR_SIX, ph::MOON))
                    .small()
                    .color(p.texto_tenue),
            );
            ui.add_space(10.0);
            ui.label(RichText::new("Hecho por Arturo (CapyIsInsano)").color(p.texto));
            ui.label(RichText::new("Licencia GPL-3.0 o posterior").small().color(p.texto_tenue));
            ui.add_space(6.0);
            ui.label(
                RichText::new("Usa egui/eframe, Lua 5.4 e íconos Phosphor (MIT).")
                    .small()
                    .color(p.texto_tenue),
            );
            ui.add_space(14.0);
            if boton_principal(ui, p, "Cerrar").clicked() {
                cerrar = true;
            }
        });
        cerrar
    }

    // ------------------------------------------------------------------
    //  Notificaciones y tareas
    // ------------------------------------------------------------------

    pub(crate) fn mostrar_notificaciones(&mut self, ctx: &egui::Context) {
        let vida = Duration::from_millis(4200);
        self.notificaciones.retain(|n| n.creada.elapsed() < vida);
        if self.notificaciones.is_empty() {
            return;
        }
        let p = self.paleta.clone();
        let mut quitar = None;
        egui::Area::new(Id::new("notificaciones"))
            .order(egui::Order::Foreground)
            .anchor(Align2::RIGHT_BOTTOM, vec2(-16.0, -42.0))
            .interactable(true)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Max), |ui| {
                    for (i, n) in self.notificaciones.iter().enumerate() {
                        let t = n.creada.elapsed().as_secs_f32();
                        let alfa = if t > 3.6 { ((4.2 - t) / 0.6).clamp(0.0, 1.0) } else { (t / 0.15).clamp(0.0, 1.0) };
                        // Entra deslizándose desde la derecha
                        let entrada = if self.config.animaciones { crate::app::ease(t / 0.28) } else { 1.0 };
                        let desliz = ((1.0 - entrada) * 60.0).round() as i8;
                        let (glifo, color) = match n.tipo {
                            TipoAviso::Info => (ph::INFO, p.acento),
                            TipoAviso::Exito => (ph::CHECK_CIRCLE, p.exito),
                            TipoAviso::Error => (ph::WARNING_CIRCLE, p.peligro),
                        };
                        let r = Frame::new()
                            .fill(con_alfa(p.elevado, 0.97 * alfa))
                            .stroke(Stroke::new(1.0, con_alfa(p.borde, alfa)))
                            .corner_radius(CornerRadius::same(12))
                            .inner_margin(Margin::symmetric(14, 10))
                            .outer_margin(Margin { left: 0, right: -desliz, top: 0, bottom: 0 })
                            .shadow(egui::Shadow { offset: [0, 4], blur: 14, spread: 0, color: Color32::from_black_alpha((70.0 * alfa) as u8) })
                            .show(ui, |ui| {
                                ui.set_max_width(380.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(glifo).size(18.0).color(con_alfa(color, alfa)));
                                    ui.add(egui::Label::new(RichText::new(&n.texto).color(con_alfa(p.texto, alfa))).wrap());
                                });
                            })
                            .response
                            .interact(Sense::click());
                        if r.clicked() {
                            quitar = Some(i);
                        }
                        ui.add_space(6.0);
                    }
                });
            });
        if let Some(i) = quitar {
            self.notificaciones.remove(i);
        }
        ctx.request_repaint_after(Duration::from_millis(50));
    }

    pub(crate) fn ventana_tareas(&mut self, ctx: &egui::Context) {
        if !self.mostrar_tareas || self.tareas.is_empty() {
            if self.tareas.is_empty() {
                self.mostrar_tareas = false;
            }
            return;
        }
        let p = self.paleta.clone();
        let mut cancelar = None;
        egui::Area::new(Id::new("tareas"))
            .order(egui::Order::Foreground)
            .anchor(Align2::RIGHT_BOTTOM, vec2(-16.0, -40.0))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(p.elevado)
                    .stroke(Stroke::new(1.0, p.borde))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(14))
                    .shadow(ctx.global_style().visuals.popup_shadow)
                    .show(ui, |ui| {
                        ui.set_width(360.0);
                        ui.label(RichText::new("Tareas en curso").strong());
                        for t in &self.tareas {
                            ui.add_space(8.0);
                            let glifo = match t.tipo {
                                TipoTarea::Copiar { .. } => ph::COPY,
                                TipoTarea::Mover { .. } => ph::ARROW_RIGHT,
                                TipoTarea::Papelera { .. } | TipoTarea::VaciarPapelera => ph::TRASH,
                                TipoTarea::Restaurar { .. } => ph::ARROW_COUNTER_CLOCKWISE,
                                TipoTarea::Eliminar { .. } | TipoTarea::BorrarDePapelera { .. } => ph::X_CIRCLE,
                            };
                            ui.label(format!("{glifo}  {}", t.titulo));
                            let pr = t.estado.lock().map(|e| e.progreso.clone()).unwrap_or_default();
                            let segundos = t.inicio.elapsed().as_secs_f64();
                            if pr.bytes_total > 0 && pr.bytes_hechos > 0 && segundos > 1.0 {
                                let velocidad = pr.bytes_hechos as f64 / segundos;
                                let resta = (pr.bytes_total.saturating_sub(pr.bytes_hechos)) as f64 / velocidad.max(1.0);
                                ui.label(
                                    RichText::new(format!(
                                        "{}/s · quedan {}",
                                        formato::tamano(velocidad as u64),
                                        if resta < 60.0 { format!("{resta:.0} s") } else { format!("{:.0} min", resta / 60.0) }
                                    ))
                                    .small()
                                    .color(p.texto_tenue),
                                );
                            }
                            let f = t.fraccion();
                            ui.add(egui::ProgressBar::new(f.unwrap_or(0.0)).animate(f.is_none()).desired_height(8.0));
                            ui.horizontal(|ui| {
                                let detalle = if pr.bytes_total > 0 {
                                    format!("{} de {} · {}", formato::tamano(pr.bytes_hechos), formato::tamano(pr.bytes_total), pr.actual)
                                } else {
                                    pr.actual.clone()
                                };
                                ui.add(egui::Label::new(RichText::new(detalle).small().color(p.texto_tenue)).truncate());
                                fila_botones(ui, |ui| {
                                    if ui.small_button("Cancelar").clicked() {
                                        cancelar = Some(t.id);
                                    }
                                });
                            });
                        }
                    });
            });
        if let Some(id) = cancelar {
            self.cancelar_tarea(id);
        }
    }
}

#[allow(dead_code)]
fn _cambio(_: CambioEstilo) {}
