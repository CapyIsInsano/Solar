//! Espacios de trabajo: grupos de pestañas con nombre (por ejemplo
//! «Trabajo», «Juegos», «Escuela») que recuerdan sus carpetas y la vista de
//! cada una. Cada espacio puede tener su propio tema y sus propios ajustes
//! de archivos, o usar los generales.
//!
//! Se guardan en ~/.config/solar/espacios.lua. Para que dos ventanas de
//! Solar no se pisen, cada espacio solo puede estar abierto en una ventana a
//! la vez (un candado en $XDG_RUNTIME_DIR). Una ventana nueva cuyo espacio ya
//! está abierto en otra usa un espacio «Temporal» que no se guarda (pero se
//! puede guardar como espacio nuevo). Al guardar, Solar vuelve a leer el
//! archivo y solo cambia lo suyo.

use crate::app::{Accion, Solar, TipoAviso, Ubicacion};
use crate::iconos::{self, FUENTE_RELLENA};
use crate::nucleo::{self, AjustesVista, EspacioTrabajo, Espacios, Orden, OrdenOrbita, PestanaGuardada, Vista};
use crate::pestanas::Pestana;
use crate::sonido::{self, Sonido};
use crate::tema::{self, Paleta, con_alfa};
use crate::ui_util::{self, fila, interruptor};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontFamily, FontId, RichText, Sense, Stroke, Ui, vec2};
use egui_phosphor::regular as ph;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// El espacio que muestra esta ventana.
pub struct EspacioActual {
    pub datos: EspacioTrabajo,
    /// No se guarda (su espacio ya estaba abierto en otra ventana)
    pub temporal: bool,
    /// Mientras exista, ninguna otra ventana puede abrir este espacio
    #[allow(dead_code)]
    candado: Option<Candado>,
}

/// Cambios a espacios que no son el de esta ventana (se aplican al guardar).
#[derive(Clone)]
pub enum OpEspacio {
    Renombrar { id: String, nombre: String },
    Eliminar(String),
    AgregarPestana { id: String, pestana: PestanaGuardada },
}

/// Qué pestañas lleva un espacio nuevo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Empezar {
    Abiertas,
    Actual,
    Personal,
}

/// El formulario de «Nuevo espacio» y «Configurar espacio».
#[derive(Clone, Debug)]
pub struct FormEspacio {
    /// None = espacio nuevo
    pub id: Option<String>,
    pub nombre: String,
    pub icono: String,
    pub color: Option<Color32>,
    pub tema: String,
    pub vista: Vista,
    pub propios: bool,
    pub ajustes: AjustesVista,
    pub empezar: Empezar,
    pub iniciado: bool,
    pub error: Option<String>,
}

// ----------------------------------------------------------------------
//  Candados (un espacio, una ventana)
// ----------------------------------------------------------------------

struct Candado(#[allow(dead_code)] std::fs::File);

fn dir_candados() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| PathBuf::from(format!("/tmp/solar-{}", unsafe { libc::getuid() })));
    base.join("solar")
}

fn archivo_candado(id: &str) -> PathBuf {
    let limpio: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    dir_candados().join(format!("espacio-{limpio}.lock"))
}

impl Candado {
    fn tomar(id: &str) -> Option<Self> {
        let ruta = archivo_candado(id);
        if let Some(d) = ruta.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let f = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(&ruta).ok()?;
        let r = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        (r == 0).then_some(Candado(f))
    }

    /// ¿Lo tiene otra ventana?
    fn ocupado(id: &str) -> bool {
        let Ok(f) = std::fs::OpenOptions::new().write(true).open(archivo_candado(id)) else { return false };
        let r = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if r == 0 {
            unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_UN) };
            false
        } else {
            true
        }
    }
}

fn nuevo_id() -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) as u64;
    let mut x = t ^ ((std::process::id() as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15;
    let mut s = String::new();
    for _ in 0..8 {
        let d = (x % 36) as u32;
        s.push(char::from_digit(d, 36).unwrap_or('x'));
        x /= 36;
    }
    s
}

pub fn espacio_vacio(id: String, nombre: &str, config: &nucleo::Config) -> EspacioTrabajo {
    EspacioTrabajo {
        id,
        nombre: nombre.to_string(),
        icono: String::new(),
        color: String::new(),
        tema: String::new(),
        vista: config.vista,
        propios: false,
        ajustes: config.archivos,
        pestanas: Vec::new(),
        activa: 0,
    }
}

/// Glifo y color con que se dibuja un espacio.
pub fn apariencia(e: &EspacioTrabajo, p: &Paleta) -> (&'static str, Color32) {
    let glifo = iconos::emblema(&e.icono).map(|x| x.glifo).unwrap_or(ph::PLANET);
    let color = tema::de_hex(&e.color).unwrap_or(p.acento);
    (glifo, color)
}

/// Lo que se decide al abrir Solar.
pub struct Sesion {
    pub espacio: EspacioActual,
    pub lista: Vec<EspacioTrabajo>,
    pub pestanas: Vec<(Ubicacion, Vista)>,
    pub activa: usize,
    pub avisos: Option<String>,
}

/// Lee los espacios y decide qué abrir: el último espacio usado (con sus
/// pestañas si `restaurar_sesion`) y, si se pidió una carpeta, en una pestaña
/// más. Si ese espacio ya está abierto en otra ventana, un espacio temporal.
pub fn iniciar_sesion(archivo: &Path, config: &nucleo::Config, abrir: Option<PathBuf>, desde: Option<&str>) -> Sesion {
    let carga = Espacios::cargar(archivo, &config.archivos);
    let mut lista = carga.espacios.lista;
    let avisos = carga.avisos;
    if lista.is_empty() && carga.existia && avisos.is_some() {
        // El archivo tiene errores: no lo pisamos hasta que se arregle
        let datos = espacio_vacio(nuevo_id(), "Temporal", config);
        let u = Ubicacion::Ruta(abrir.unwrap_or_else(nucleo::dir_hogar));
        return Sesion { espacio: EspacioActual { datos, temporal: true, candado: None }, lista, pestanas: vec![(u, config.vista)], activa: 0, avisos };
    }
    if lista.is_empty() {
        lista.push(espacio_vacio(nuevo_id(), "Principal", config));
    }
    // Al abrir una ventana nueva desde otra, el espacio de esa otra
    let preferido = desde
        .and_then(|d| lista.iter().position(|e| e.id == d))
        .or_else(|| lista.iter().position(|e| e.id == carga.espacios.activo))
        .unwrap_or(0);
    let pestanas_de = |e: &EspacioTrabajo| -> Vec<(Ubicacion, Vista)> {
        e.pestanas
            .iter()
            .filter_map(|p| match &p.ruta {
                None => Some((Ubicacion::Papelera, p.vista)),
                Some(r) if r.is_dir() => Some((Ubicacion::Ruta(r.clone()), p.vista)),
                Some(_) => None, // ya no existe
            })
            .collect()
    };
    if let Some(candado) = Candado::tomar(&lista[preferido].id) {
        let datos = lista[preferido].clone();
        let mut pestanas = if config.restaurar_sesion { pestanas_de(&datos) } else { Vec::new() };
        let mut activa = datos.activa.min(pestanas.len().saturating_sub(1));
        if let Some(r) = abrir {
            // Si ya estaba abierta, se va a esa pestaña
            match pestanas.iter().position(|(u, _)| u.ruta() == Some(r.as_path())) {
                Some(i) => activa = i,
                None => {
                    pestanas.push((Ubicacion::Ruta(r), datos.vista));
                    activa = pestanas.len() - 1;
                }
            }
        }
        if pestanas.is_empty() {
            pestanas.push((Ubicacion::Ruta(nucleo::dir_hogar()), datos.vista));
            activa = 0;
        }
        return Sesion { espacio: EspacioActual { datos, temporal: false, candado: Some(candado) }, lista, pestanas, activa, avisos };
    }
    // Ese espacio ya está abierto en otra ventana: uno temporal con sus ajustes
    let base = &lista[preferido];
    let mut datos = base.clone();
    datos.id = nuevo_id();
    datos.nombre = "Temporal".into();
    datos.icono = String::new();
    datos.color = String::new();
    datos.pestanas.clear();
    let u = Ubicacion::Ruta(abrir.unwrap_or_else(nucleo::dir_hogar));
    let vista = base.pestanas.get(base.activa).map(|p| p.vista).unwrap_or(base.vista);
    Sesion { espacio: EspacioActual { datos, temporal: true, candado: None }, lista, pestanas: vec![(u, vista)], activa: 0, avisos }
}

impl Solar {
    /// Los ajustes de archivos que valen ahora (los del espacio, si tiene propios).
    pub fn av(&self) -> &AjustesVista {
        let d = &self.espacio_trabajo.datos;
        if d.propios { &d.ajustes } else { &self.config.archivos }
    }

    /// Cambia los ajustes de archivos (los del espacio o los generales) y
    /// rehace lo que haga falta.
    pub fn cambiar_av(&mut self, f: impl FnOnce(&mut AjustesVista)) {
        let antes = *self.av();
        if self.espacio_trabajo.datos.propios {
            f(&mut self.espacio_trabajo.datos.ajustes);
            self.marcar_sesion();
        } else {
            f(&mut self.config.archivos);
            self.config_cambio();
        }
        let ahora = *self.av();
        self.despues_de_cambiar_av(&antes, &ahora);
    }

    pub(crate) fn despues_de_cambiar_av(&mut self, antes: &AjustesVista, ahora: &AjustesVista) {
        if antes.mostrar_ocultos != ahora.mostrar_ocultos {
            self.visibles_sucios = true;
        }
        if antes.orden != ahora.orden
            || antes.orden_descendente != ahora.orden_descendente
            || antes.carpetas_primero != ahora.carpetas_primero
        {
            self.cargar_contexto_pendiente = true;
        }
    }

    /// El tema que se ve: el del espacio, si tiene uno, o el general.
    pub fn tema_id(&self) -> &str {
        let propio = &self.espacio_trabajo.datos.tema;
        if self.config.tema != crate::ajustes::ID_EDITOR && !propio.is_empty() && self.temas.iter().any(|t| t.id == *propio) {
            propio
        } else {
            &self.config.tema
        }
    }

    pub fn marcar_sesion(&mut self) {
        if self.espacios_sucios.is_none() {
            self.espacios_sucios = Some(Instant::now());
        }
    }

    /// El espacio de esta ventana tal como está ahora (con sus pestañas).
    pub fn foto_espacio(&self) -> EspacioTrabajo {
        let mut e = self.espacio_trabajo.datos.clone();
        e.pestanas = (0..self.pestanas.len())
            .map(|i| PestanaGuardada { ruta: self.ubicacion_de(i).ruta().map(Path::to_path_buf), vista: self.pestanas[i].vista })
            .collect();
        e.activa = self.activa;
        e
    }

    /// Guarda si pasó un rato desde el último cambio.
    pub(crate) fn guardar_sesion_si_hace_falta(&mut self, ctx: &egui::Context) {
        let Some(desde) = self.espacios_sucios else { return };
        if desde.elapsed() < Duration::from_millis(1200) {
            ctx.request_repaint_after(Duration::from_millis(1300));
            return;
        }
        self.guardar_espacios();
    }

    /// Lee el archivo, aplica lo de esta ventana y lo escribe.
    pub fn guardar_espacios(&mut self) {
        self.espacios_sucios = None;
        let carga = Espacios::cargar(&self.rutas.espacios, &self.config.archivos);
        if carga.existia && carga.avisos.is_some() && carga.espacios.lista.is_empty() {
            // Está roto (¿editado a mano?): mejor no tocarlo
            return;
        }
        let mut disco = carga.espacios;
        for op in self.ops_espacios.drain(..) {
            match op {
                OpEspacio::Renombrar { id, nombre } => {
                    if let Some(e) = disco.lista.iter_mut().find(|e| e.id == id) {
                        e.nombre = nombre;
                    }
                }
                OpEspacio::Eliminar(id) => disco.lista.retain(|e| e.id != id),
                OpEspacio::AgregarPestana { id, pestana } => {
                    if let Some(e) = disco.lista.iter_mut().find(|e| e.id == id) {
                        e.pestanas.push(pestana);
                    }
                }
            }
        }
        if !self.espacio_trabajo.temporal {
            let foto = self.foto_espacio();
            disco.activo = foto.id.clone();
            match disco.lista.iter_mut().find(|e| e.id == foto.id) {
                Some(e) => *e = foto,
                None => disco.lista.push(foto),
            }
        }
        if let Err(e) = disco.guardar(&self.rutas.espacios) {
            self.avisar(format!("No se pudieron guardar los espacios: {e}"), TipoAviso::Error);
        }
        self.espacios = disco.lista;
    }

    /// Vuelve a leer la lista (para el menú) y cuáles están en otras ventanas.
    pub fn refrescar_espacios(&mut self) {
        let carga = Espacios::cargar(&self.rutas.espacios, &self.config.archivos);
        if !carga.espacios.lista.is_empty() || !carga.existia {
            self.espacios = carga.espacios.lista;
        }
        let mio = self.espacio_trabajo.datos.id.clone();
        self.ocupados = self.espacios.iter().filter(|e| e.id != mio && Candado::ocupado(&e.id)).map(|e| e.id.clone()).collect();
    }

    /// Cambia esta ventana a otro espacio (guardando el actual).
    pub fn cambiar_espacio(&mut self, ctx: &egui::Context, id: &str) {
        if id == self.espacio_trabajo.datos.id {
            return;
        }
        self.refrescar_espacios();
        let Some(destino) = self.espacios.iter().find(|e| e.id == id).cloned() else {
            self.avisar("Ese espacio ya no existe.", TipoAviso::Error);
            return;
        };
        let Some(candado) = Candado::tomar(id) else {
            self.avisar(format!("«{}» está abierto en otra ventana de Solar.", destino.nombre), TipoAviso::Info);
            return;
        };
        self.guardar_espacios();
        self.entrar_a_espacio(ctx, destino, candado, true);
        sonido::pedir(Sonido::Efecto);
    }

    /// Pone en la ventana el espacio `datos` (con sus pestañas).
    fn entrar_a_espacio(&mut self, ctx: &egui::Context, datos: EspacioTrabajo, candado: Candado, avisar: bool) {
        let antes = *self.av();
        // Las pestañas del espacio anterior se sueltan
        self.soltar_todas_las_pestanas();
        let mut pestanas: Vec<Pestana> = Vec::new();
        for p in &datos.pestanas {
            let u = match &p.ruta {
                None => Ubicacion::Papelera,
                Some(r) if r.is_dir() => Ubicacion::Ruta(r.clone()),
                Some(_) => continue,
            };
            let id = self.nuevo_id_pestana();
            pestanas.push(Pestana::nueva(id, u, p.vista));
        }
        if pestanas.is_empty() {
            let id = self.nuevo_id_pestana();
            pestanas.push(Pestana::nueva(id, Ubicacion::Ruta(nucleo::dir_hogar()), datos.vista));
        }
        let activa = datos.activa.min(pestanas.len() - 1);
        self.pestanas = pestanas;
        self.cerradas.clear();
        self.espacio_trabajo = EspacioActual { datos, temporal: false, candado: Some(candado) };
        let ahora = *self.av();
        self.despues_de_cambiar_av(&antes, &ahora);
        self.cargar_pestana(ctx, activa);
        self.guardar_espacios();
        if avisar {
            let nombre = self.espacio_trabajo.datos.nombre.clone();
            self.avisar(format!("Espacio «{nombre}»."), TipoAviso::Info);
        }
    }

    fn soltar_todas_las_pestanas(&mut self) {
        let entradas = std::mem::take(&mut self.entradas);
        crate::memoria::soltar_aparte((entradas, std::mem::take(&mut self.indice)), true);
        self.visibles.clear();
        self.pestanas.clear();
        self.activa = 0;
    }

    /// Crea un espacio con lo del formulario y se cambia a él.
    pub fn crear_espacio(&mut self, ctx: &egui::Context, f: FormEspacio) {
        let id = nuevo_id();
        let Some(candado) = Candado::tomar(&id) else {
            self.avisar("No se pudo crear el espacio (no hay dónde poner su candado).", TipoAviso::Error);
            return;
        };
        let pestanas: Vec<PestanaGuardada> = match f.empezar {
            Empezar::Abiertas => self.foto_espacio().pestanas,
            Empezar::Actual => vec![PestanaGuardada { ruta: self.ubicacion.ruta().map(Path::to_path_buf), vista: self.vista() }],
            Empezar::Personal => vec![PestanaGuardada { ruta: Some(nucleo::dir_hogar()), vista: f.vista }],
        };
        let activa = if f.empezar == Empezar::Abiertas { self.activa } else { 0 };
        let datos = EspacioTrabajo {
            id,
            nombre: f.nombre.trim().to_string(),
            icono: f.icono,
            color: f.color.map(tema::hex).unwrap_or_default(),
            tema: f.tema,
            vista: f.vista,
            propios: f.propios,
            ajustes: f.ajustes,
            pestanas,
            activa,
        };
        // El temporal se vuelve este espacio; uno de verdad se guarda antes de irse
        if !self.espacio_trabajo.temporal {
            self.guardar_espacios();
        }
        self.entrar_a_espacio(ctx, datos, candado, true);
        sonido::pedir(Sonido::Listo);
    }

    /// Aplica el formulario de «Configurar espacio» (solo el de esta ventana).
    pub fn configurar_espacio(&mut self, f: FormEspacio) {
        let antes = *self.av();
        let d = &mut self.espacio_trabajo.datos;
        d.nombre = f.nombre.trim().to_string();
        d.icono = f.icono;
        d.color = f.color.map(tema::hex).unwrap_or_default();
        d.tema = f.tema;
        d.vista = f.vista;
        d.propios = f.propios;
        d.ajustes = f.ajustes;
        let ahora = *self.av();
        self.despues_de_cambiar_av(&antes, &ahora);
        self.apariencia_aplicada = None;
        if self.espacio_trabajo.temporal {
            self.avisar("Este espacio es temporal: guárdalo como espacio para que se recuerde.", TipoAviso::Info);
        }
        self.marcar_sesion();
    }

    pub fn renombrar_espacio(&mut self, id: &str, nombre: &str) {
        let nombre = nombre.trim().to_string();
        if id == self.espacio_trabajo.datos.id {
            self.espacio_trabajo.datos.nombre = nombre;
            self.marcar_sesion();
        } else {
            self.ops_espacios.push(OpEspacio::Renombrar { id: id.to_string(), nombre });
            self.guardar_espacios();
        }
    }

    /// Borra un espacio (sus carpetas no se tocan). Si es el de esta ventana,
    /// antes se cambia a otro.
    pub fn eliminar_espacio(&mut self, ctx: &egui::Context, id: &str) {
        self.refrescar_espacios();
        if self.ocupados.contains(id) {
            self.avisar("Ese espacio está abierto en otra ventana; ciérrala primero.", TipoAviso::Info);
            return;
        }
        let nombre = self.espacios.iter().find(|e| e.id == id).map(|e| e.nombre.clone()).unwrap_or_default();
        if id == self.espacio_trabajo.datos.id {
            let libre = self.espacios.iter().find(|e| e.id != id && !self.ocupados.contains(&e.id)).cloned();
            let (destino, candado) = match libre.and_then(|e| Candado::tomar(&e.id).map(|c| (e, c))) {
                Some(x) => x,
                None => {
                    let e = espacio_vacio(nuevo_id(), "Principal", &self.config);
                    let Some(c) = Candado::tomar(&e.id) else { return };
                    (e, c)
                }
            };
            self.ops_espacios.push(OpEspacio::Eliminar(id.to_string()));
            self.entrar_a_espacio(ctx, destino, candado, false);
        } else {
            self.ops_espacios.push(OpEspacio::Eliminar(id.to_string()));
            self.guardar_espacios();
        }
        sonido::pedir(Sonido::Retraccion);
        if !nombre.is_empty() {
            self.avisar(format!("Se eliminó el espacio «{nombre}» (tus carpetas siguen ahí)."), TipoAviso::Info);
        }
    }

    /// Manda una pestaña a otro espacio (se cierra aquí).
    pub fn pestana_a_espacio(&mut self, ctx: &egui::Context, pestana: u64, espacio: &str) {
        self.refrescar_espacios();
        if self.ocupados.contains(espacio) {
            self.avisar("Ese espacio está abierto en otra ventana.", TipoAviso::Info);
            return;
        }
        let Some(i) = self.indice_pestana(pestana) else { return };
        let p = PestanaGuardada { ruta: self.ubicacion_de(i).ruta().map(Path::to_path_buf), vista: self.pestanas[i].vista };
        self.ops_espacios.push(OpEspacio::AgregarPestana { id: espacio.to_string(), pestana: p });
        let nombre = self.espacios.iter().find(|e| e.id == espacio).map(|e| e.nombre.clone()).unwrap_or_default();
        if self.pestanas.len() > 1 {
            self.cerrar_pestana(ctx, i);
        }
        self.guardar_espacios();
        self.avisar(format!("La pestaña se mandó a «{nombre}»."), TipoAviso::Info);
    }

    pub fn form_espacio(&self, nuevo: bool) -> FormEspacio {
        let d = &self.espacio_trabajo.datos;
        if nuevo {
            let n = self.espacios.len() + 1;
            FormEspacio {
                id: None,
                nombre: format!("Espacio {n}"),
                icono: String::new(),
                color: None,
                tema: String::new(),
                vista: self.vista(),
                propios: false,
                ajustes: *self.av(),
                empezar: if self.pestanas.len() > 1 { Empezar::Abiertas } else { Empezar::Actual },
                iniciado: false,
                error: None,
            }
        } else {
            FormEspacio {
                id: Some(d.id.clone()),
                nombre: d.nombre.clone(),
                icono: d.icono.clone(),
                color: tema::de_hex(&d.color),
                tema: d.tema.clone(),
                vista: d.vista,
                propios: d.propios,
                // Al activar ajustes propios se empieza con los que se ven ahora
                ajustes: *self.av(),
                empezar: Empezar::Abiertas,
                iniciado: false,
                error: None,
            }
        }
    }

    // ------------------------------------------------------------------
    //  Interfaz: el botón del espacio y su menú
    // ------------------------------------------------------------------

    pub(crate) fn chip_espacio(&mut self, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        let d = &self.espacio_trabajo.datos;
        let temporal = self.espacio_trabajo.temporal;
        let (glifo, color) = apariencia(d, p);
        let nombre = d.nombre.clone();
        let g = ui.painter().layout_no_wrap(nombre.clone(), FontId::proportional(13.0), p.texto);
        let ancho = (g.size().x + 54.0).min(190.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(ancho, 28.0), Sense::click());
        let h = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(14), con_alfa(color, 0.14 + 0.10 * h));
        if temporal {
            // Borde punteado: este espacio no se guarda
            borde_punteado(painter, rect.shrink(0.75), Stroke::new(1.2, con_alfa(color, 0.85)));
        }
        painter.text(
            rect.left_center() + vec2(15.0, 0.0),
            Align2::CENTER_CENTER,
            glifo,
            FontId::new(15.0, FontFamily::Name(FUENTE_RELLENA.into())),
            color,
        );
        let g = crate::vista::texto_ajustado(painter, &nombre, ancho - 50.0, 1, 13.0, p.texto, false);
        painter.galley(rect.left_center() + vec2(28.0, -g.size().y / 2.0), g, p.texto);
        painter.text(rect.right_center() - vec2(12.0, 0.0), Align2::CENTER_CENTER, ph::CARET_DOWN, FontId::proportional(11.0), p.texto_tenue);
        let resp = resp
            .on_hover_text(if temporal {
                "Espacio temporal (su espacio ya estaba abierto en otra ventana; no se guarda)".to_string()
            } else {
                format!("Espacio de trabajo: {nombre} ({})", self.atajos.texto("espacios"))
            })
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if resp.clicked() || self.abrir_menu_espacios {
            self.refrescar_espacios();
        }
        let abrir = std::mem::take(&mut self.abrir_menu_espacios);
        let popup = egui::Popup::menu(&resp).open_memory(if abrir { Some(egui::SetOpenCommand::Bool(true)) } else if resp.clicked() { Some(egui::SetOpenCommand::Toggle) } else { None });
        popup.show(|ui| self.menu_espacios(ui, p, acciones));
    }

    fn menu_espacios(&self, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        ui.set_min_width(290.0);
        ui.add_space(2.0);
        ui.label(RichText::new("ESPACIOS DE TRABAJO").small().color(p.texto_tenue));
        ui.add_space(2.0);
        let mio = &self.espacio_trabajo.datos.id;
        let mut lista: Vec<&EspacioTrabajo> = self.espacios.iter().collect();
        if self.espacio_trabajo.temporal {
            lista.insert(0, &self.espacio_trabajo.datos);
        }
        for e in lista {
            let actual = e.id == *mio;
            let ocupado = self.ocupados.contains(&e.id);
            let (glifo, color) = apariencia(e, p);
            let n = if actual { self.pestanas.len() } else { e.pestanas.len().max(1) };
            let detalle = if ocupado {
                "en otra ventana".to_string()
            } else if n == 1 {
                "1 pestaña".into()
            } else {
                format!("{n} pestañas")
            };
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
            let encima = ui.rect_contains_pointer(rect);
            // Renombrar y eliminar, con botoncitos que salen al pasar el mouse
            // (no para el temporal ni para los que están en otra ventana)
            let editable = !ocupado && !(actual && self.espacio_trabajo.temporal);
            let botones = editable && encima;
            if actual {
                ui.painter().rect_filled(rect, CornerRadius::same(8), con_alfa(color, 0.18));
            } else if encima && !ocupado {
                ui.painter().rect_filled(rect, CornerRadius::same(8), p.hover);
            }
            let alfa = if ocupado { 0.45 } else { 1.0 };
            ui.painter().text(
                rect.left_center() + vec2(16.0, 0.0),
                Align2::CENTER_CENTER,
                glifo,
                FontId::new(16.0, FontFamily::Name(FUENTE_RELLENA.into())),
                con_alfa(color, alfa),
            );
            let titulo = crate::vista::texto_ajustado(ui.painter(), &e.nombre, rect.width() - 150.0, 1, 13.5, con_alfa(p.texto, alfa), false);
            ui.painter().galley(rect.left_center() + vec2(34.0, -titulo.size().y / 2.0), titulo, p.texto);
            if botones {
                let boton = |ui: &mut Ui, centro: egui::Pos2, glifo: &str, sal: &str, ayuda: &str, tinta: Color32| {
                    let r = egui::Rect::from_center_size(centro, vec2(26.0, 26.0));
                    let b = ui.interact(r, ui.id().with((sal, &e.id)), Sense::click()).on_hover_text(ayuda);
                    if b.hovered() {
                        ui.painter().rect_filled(r, CornerRadius::same(6), con_alfa(p.texto, 0.12));
                    }
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, glifo, FontId::proportional(14.0), tinta);
                    b.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                };
                if boton(ui, rect.right_center() - vec2(44.0, 0.0), ph::PENCIL_SIMPLE, "renombrar", "Renombrar", p.texto) {
                    acciones.push(Accion::RenombrarEspacio(e.id.clone()));
                    ui.close();
                }
                if boton(ui, rect.right_center() - vec2(16.0, 0.0), ph::TRASH, "eliminar", "Eliminar", p.peligro) {
                    acciones.push(Accion::PreguntarEliminarEspacio(e.id.clone(), e.nombre.clone()));
                    ui.close();
                }
            } else {
                ui.painter().text(rect.right_center() - vec2(if actual { 30.0 } else { 10.0 }, 0.0), Align2::RIGHT_CENTER, detalle, FontId::proportional(11.5), p.texto_tenue);
                if actual {
                    ui.painter().text(rect.right_center() - vec2(12.0, 0.0), Align2::CENTER_CENTER, ph::CHECK, FontId::proportional(14.0), color);
                }
            }
            if resp.clicked() && !actual && !ocupado {
                acciones.push(Accion::CambiarEspacio(e.id.clone()));
                ui.close();
            }
            if ocupado {
                resp.on_hover_text("Está abierto en otra ventana de Solar");
            }
        }
        ui.separator();
        let temporal = self.espacio_trabajo.temporal;
        if temporal {
            if ui.button(format!("{}   Guardar como espacio…", ph::FLOPPY_DISK)).clicked() {
                acciones.push(Accion::NuevoEspacio);
                ui.close();
            }
        } else if ui.button(format!("{}   Nuevo espacio…", ph::PLUS)).clicked() {
            acciones.push(Accion::NuevoEspacio);
            ui.close();
        }
        if ui.button(format!("{}   Configurar este espacio…", ph::SLIDERS_HORIZONTAL)).clicked() {
            acciones.push(Accion::ConfigurarEspacio);
            ui.close();
        }
        if !temporal {
            let id = self.espacio_trabajo.datos.id.clone();
            let nombre = self.espacio_trabajo.datos.nombre.clone();
            if ui.button(format!("{}   Renombrar «{}»…", ph::PENCIL_SIMPLE, recortar(&nombre, 18))).clicked() {
                acciones.push(Accion::RenombrarEspacio(id.clone()));
                ui.close();
            }
            if ui.button(RichText::new(format!("{}   Eliminar «{}»…", ph::TRASH, recortar(&nombre, 18))).color(p.peligro)).clicked() {
                acciones.push(Accion::PreguntarEliminarEspacio(id, nombre));
                ui.close();
            }
        }
    }

    // ------------------------------------------------------------------
    //  Formulario (nuevo y configurar)
    // ------------------------------------------------------------------

    /// Devuelve true si hay que cerrar el diálogo.
    pub(crate) fn dialogo_espacio(&mut self, ui: &mut Ui, p: &Paleta, f: &mut FormEspacio, acciones: &mut Vec<Accion>) -> bool {
        let nuevo = f.id.is_none();
        let temporal_a_guardar = nuevo && self.espacio_trabajo.temporal;
        ui_util::titulo(
            ui,
            p,
            if temporal_a_guardar {
                "Guardar como espacio"
            } else if nuevo {
                "Nuevo espacio de trabajo"
            } else {
                "Configurar espacio"
            },
        );
        ui.label(
            RichText::new("Un espacio guarda sus pestañas y la vista de cada una. Puede tener su propio tema y sus propios ajustes.")
                .color(p.texto_tenue),
        );
        ui.add_space(10.0);
        let mut cerrar = false;
        egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                let (glifo, color) = {
                    let icono = iconos::emblema(&f.icono).map(|x| x.glifo).unwrap_or(ph::PLANET);
                    (icono, f.color.unwrap_or(p.acento))
                };
                let (rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                ui.painter().circle_filled(rect.center(), 19.0, con_alfa(color, 0.2));
                ui.painter().text(rect.center(), Align2::CENTER_CENTER, glifo, FontId::new(21.0, FontFamily::Name(FUENTE_RELLENA.into())), color);
                let r = ui.add(egui::TextEdit::singleline(&mut f.nombre).desired_width(300.0).hint_text("Nombre (Trabajo, Juegos, Escuela…)"));
                if !f.iniciado {
                    // Con el nombre seleccionado, para escribir encima
                    r.request_focus();
                    if let Some(mut estado) = egui::TextEdit::load_state(ui.ctx(), r.id) {
                        let fin = egui::text::CCursor::new(f.nombre.chars().count());
                        estado.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), fin)));
                        estado.store(ui.ctx(), r.id);
                    }
                    f.iniciado = true;
                }
            });
            ui.add_space(8.0);
            ui_util::encabezado(ui, p, "Color");
            if let Some(c) = ui_util::rejilla_colores(ui, p, f.color, 11, 10.0) {
                f.color = Some(c);
            }
            ui_util::encabezado(ui, p, "Logotipo");
            egui::ScrollArea::vertical().id_salt("logos_espacio").max_height(118.0).show(ui, |ui| {
                if let Some(id) = ui_util::rejilla_logotipos(ui, p, Some(&f.icono), "", 12, 30.0) {
                    f.icono = id.to_string();
                }
            });
            ui.add_space(6.0);
            ui_util::encabezado(ui, p, "Apariencia");
            let general = self.temas.iter().find(|t| t.id == self.config.tema).map(|t| t.nombre.clone()).unwrap_or_default();
            fila(ui, p, "Tema", "Cada espacio puede verse distinto.", |ui| {
                let actual = if f.tema.is_empty() {
                    format!("El general ({general})")
                } else {
                    self.temas.iter().find(|t| t.id == f.tema).map(|t| t.nombre.clone()).unwrap_or(f.tema.clone())
                };
                egui::ComboBox::from_id_salt("tema_espacio").selected_text(actual).width(190.0).show_ui(ui, |ui| {
                    if ui.selectable_label(f.tema.is_empty(), format!("El general ({general})")).clicked() {
                        f.tema.clear();
                    }
                    for t in self.temas.iter().filter(|t| t.id != crate::ajustes::ID_EDITOR) {
                        if ui.selectable_label(f.tema == t.id, &t.nombre).clicked() {
                            f.tema = t.id.clone();
                        }
                    }
                });
            });
            fila(ui, p, "Vista para pestañas nuevas", "", |ui| {
                for v in Vista::TODAS.iter().rev() {
                    let (glifo, nombre, _) = crate::vista::info_vista(*v);
                    if ui.selectable_label(f.vista == *v, RichText::new(glifo).size(16.0)).on_hover_text(nombre).clicked() {
                        f.vista = *v;
                    }
                }
            });
            ui.add_space(6.0);
            ui_util::encabezado(ui, p, "Archivos");
            fila(ui, p, "Ajustes propios", "Apagado: usa los generales (Ajustes › Archivos).", |ui| {
                interruptor(ui, p, &mut f.propios);
            });
            if f.propios {
                let a = &mut f.ajustes;
                fila(ui, p, "Tamaño de los íconos", "", |ui| {
                    ui.add(egui::Slider::new(&mut a.tamano_iconos, 40.0..=160.0).step_by(4.0).show_value(false));
                });
                fila(ui, p, "Ordenar por", "", |ui| {
                    ui.checkbox(&mut a.orden_descendente, "Al revés");
                    egui::ComboBox::from_id_salt("orden_espacio").selected_text(a.orden.nombre()).show_ui(ui, |ui| {
                        for o in Orden::TODOS {
                            ui.selectable_value(&mut a.orden, o, o.nombre());
                        }
                    });
                });
                fila(ui, p, "Carpetas primero", "", |ui| {
                    interruptor(ui, p, &mut a.carpetas_primero);
                });
                fila(ui, p, "Mostrar archivos ocultos", "", |ui| {
                    interruptor(ui, p, &mut a.mostrar_ocultos);
                });
                fila(ui, p, "Órbita: que giren", "", |ui| {
                    interruptor(ui, p, &mut a.orbita_girar);
                });
                fila(ui, p, "Órbita: velocidad", "", |ui| {
                    ui.add(egui::Slider::new(&mut a.orbita_velocidad, 0.1..=4.0).logarithmic(true).max_decimals(1).suffix("×"));
                });
                fila(ui, p, "Órbita: acomodo", "", |ui| {
                    egui::ComboBox::from_id_salt("orbita_espacio").selected_text(a.orbita_orden.nombre()).width(200.0).show_ui(ui, |ui| {
                        for o in OrdenOrbita::TODOS {
                            ui.selectable_value(&mut a.orbita_orden, o, o.nombre());
                        }
                    });
                });
                fila(ui, p, "Órbita: electricidad", "", |ui| {
                    interruptor(ui, p, &mut a.orbita_electricidad);
                });
            }
            if nuevo {
                ui.add_space(6.0);
                ui_util::encabezado(ui, p, "Empezar con");
                let n = self.pestanas.len();
                ui.radio_value(&mut f.empezar, Empezar::Abiertas, if n == 1 { "La pestaña abierta".to_string() } else { format!("Las {n} pestañas abiertas") });
                ui.radio_value(&mut f.empezar, Empezar::Actual, "Solo la carpeta de esta pestaña");
                ui.radio_value(&mut f.empezar, Empezar::Personal, "La carpeta personal");
            }
        });
        if let Some(e) = &f.error {
            ui.add_space(6.0);
            ui.label(RichText::new(e).color(p.peligro));
        }
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let texto = if nuevo { if temporal_a_guardar { "Guardar espacio" } else { "Crear espacio" } } else { "Guardar" };
                let ok = ui_util::boton_acento(ui, p, texto).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Cancelar").clicked() {
                    cerrar = true;
                }
                if ok {
                    let nombre = f.nombre.trim();
                    let repetido = self
                        .espacios
                        .iter()
                        .any(|e| e.nombre.trim().eq_ignore_ascii_case(nombre) && Some(&e.id) != f.id.as_ref());
                    if nombre.is_empty() {
                        f.error = Some("Ponle un nombre.".into());
                    } else if repetido {
                        f.error = Some("Ya hay un espacio con ese nombre.".into());
                    } else {
                        let listo = std::mem::replace(f, FormEspacio { id: None, ..self.form_espacio(true) });
                        acciones.push(Accion::GuardarFormEspacio(Box::new(listo)));
                        cerrar = true;
                    }
                }
            });
        });
        cerrar
    }
}

/// Un borde punteado con forma de píldora (como el botón del espacio).
fn borde_punteado(painter: &egui::Painter, r: egui::Rect, trazo: Stroke) {
    let radio = r.height() / 2.0;
    let recto = (r.width() - 2.0 * radio).max(0.0);
    let medio = std::f32::consts::PI * radio;
    let perimetro = 2.0 * recto + 2.0 * medio;
    // Un punto del borde, a `d` de la esquina de arriba a la izquierda (en el sentido del reloj)
    let punto = |d: f32| -> egui::Pos2 {
        let d = d.rem_euclid(perimetro);
        let (izq, der) = (r.left() + radio, r.right() - radio);
        if d < recto {
            egui::pos2(izq + d, r.top())
        } else if d < recto + medio {
            let a = -std::f32::consts::FRAC_PI_2 + (d - recto) / radio;
            egui::pos2(der + radio * a.cos(), r.center().y + radio * a.sin())
        } else if d < 2.0 * recto + medio {
            egui::pos2(der - (d - recto - medio), r.bottom())
        } else {
            let a = std::f32::consts::FRAC_PI_2 + (d - 2.0 * recto - medio) / radio;
            egui::pos2(izq + radio * a.cos(), r.center().y + radio * a.sin())
        }
    };
    let n = (perimetro / 9.0).round().max(4.0) as usize;
    let paso = perimetro / n as f32;
    for i in 0..n {
        let inicio = i as f32 * paso;
        let largo = paso * 0.55;
        let puntos: Vec<egui::Pos2> = (0..=3).map(|k| punto(inicio + largo * k as f32 / 3.0)).collect();
        painter.add(egui::Shape::line(puntos, trazo));
    }
}

fn recortar(t: &str, n: usize) -> String {
    if t.chars().count() <= n { t.to_string() } else { format!("{}…", t.chars().take(n - 1).collect::<String>()) }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn un_espacio_una_ventana() {
        let dir = std::env::temp_dir().join(format!("solar-candados-{}", std::process::id()));
        // SAFETY: las pruebas de este módulo no leen el entorno en paralelo
        unsafe { std::env::set_var("XDG_RUNTIME_DIR", &dir) };
        let _ = std::fs::create_dir_all(&dir);
        let id = nuevo_id();
        assert!(!Candado::ocupado(&id));
        let c = Candado::tomar(&id).expect("primer candado");
        assert!(Candado::ocupado(&id));
        assert!(Candado::tomar(&id).is_none());
        drop(c);
        assert!(!Candado::ocupado(&id));
        assert_eq!(archivo_candado("a/b c").file_name().unwrap(), "espacio-a_b_c.lock");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ids_distintos() {
        let a = nuevo_id();
        std::thread::sleep(Duration::from_millis(2));
        let b = nuevo_id();
        assert_eq!(a.len(), 8);
        assert_ne!(a, b);
    }
}
