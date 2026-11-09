//! La aplicación: estado, navegación, selección y acciones.
//! Las partes visibles están en `vista.rs`, `barras.rs`, `dialogos.rs` y
//! `ajustes.rs`.

use crate::actualizar::{self, Actualizador};
use crate::ajustes::EstadoAjustes;
use crate::atajos::Atajos;
use crate::dialogos::{Dialogo, EstadoAbrirCon, EstadoPersonalizar, EstadoPropiedades};
use crate::sonido::{self, Sonido};
use crate::nucleo::{
    self, Carpetas, Config, Conflicto, Dispositivo, Entrada, Espacio, EstiloCarpeta, Lugar, Orden, Programa, Tema, TipoEntrada,
    Vista,
};
use crate::tareas::{self, ComandoVigia, Mensaje, Tarea, TipoTarea};
use crate::tema::{self, Paleta};
use eframe::egui::{self, Color32, Pos2, ViewportCommand};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, PartialEq, Debug)]
pub enum Ubicacion {
    Ruta(PathBuf),
    Papelera,
}

impl Ubicacion {
    pub fn ruta(&self) -> Option<&Path> {
        match self {
            Ubicacion::Ruta(r) => Some(r),
            Ubicacion::Papelera => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModoPortapapeles {
    Copiar,
    Cortar,
}

#[derive(Clone, Debug)]
pub struct Portapapeles {
    pub rutas: Vec<PathBuf>,
    pub modo: ModoPortapapeles,
    pub texto: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TipoAviso {
    Info,
    Exito,
    Error,
}

pub struct Notificacion {
    pub texto: String,
    pub tipo: TipoAviso,
    pub creada: Instant,
}

/// Arrastre de archivos dentro de Solar (carga útil de egui).
#[derive(Clone, Debug)]
pub struct Arrastre(pub Vec<PathBuf>);

pub struct Banda {
    pub inicio: Pos2,
    pub base: HashSet<PathBuf>,
}

#[derive(Clone, Debug)]
pub enum CambioEstilo {
    Color(Option<Color32>),
    Logotipo(Option<String>),
    ColorLogotipo(Option<Color32>),
    Quitar,
}

#[derive(Clone, Debug)]
pub enum Accion {
    Navegar(Ubicacion),
    Atras,
    Adelante,
    Subir,
    Inicio,
    Recargar,
    Abrir(PathBuf),
    AbrirSeleccion,
    AbrirCon(Vec<PathBuf>),
    AbrirConPrograma { programa: Box<Programa>, rutas: Vec<PathBuf> },
    AbrirEnVentana(PathBuf),
    AbrirTerminal(PathBuf),
    Copiar(Vec<PathBuf>),
    Cortar(Vec<PathBuf>),
    Pegar(PathBuf),
    PegarTexto(String),
    Duplicar(Vec<PathBuf>),
    Renombrar(PathBuf),
    Papelera(Vec<PathBuf>),
    Eliminar(Vec<PathBuf>),
    Restaurar(Vec<PathBuf>),
    BorrarDePapelera(Vec<PathBuf>),
    VaciarPapelera,
    Propiedades(Vec<PathBuf>),
    CopiarRuta(Vec<PathBuf>),
    AgregarLugar(PathBuf),
    QuitarLugar(usize),
    Estilo(Vec<PathBuf>, CambioEstilo),
    Personalizar(Vec<PathBuf>),
    NuevaCarpeta,
    NuevoArchivo,
    SeleccionarTodo,
    AlternarOcultos,
    CambiarVista(Vista),
    Ordenar(Orden),
    OrdenDescendente(bool),
    CarpetasPrimero(bool),
    Zoom(f32),
    AlternarLateral,
    EditarRuta,
    Buscar,
    Ajustes,
    NuevaVentana,
    Cerrar,
    Montar { dispositivo: String, abrir: bool },
    Desmontar { dispositivo: String, expulsar: bool },
    Soltar { destino: PathBuf, rutas: Vec<PathBuf>, copiar: bool },
    TraerExternos(Vec<PathBuf>),
    AcercaDe,
    Tarea(TipoTarea),
    BuscarActualizaciones,
    Actualizar(Box<actualizar::Version>),
    CancelarActualizacion,
    Reiniciar,
    BorrarCache,
}

/// Programas del submenú «Abrir con» para cierta selección.
pub type MenuProgramas = (Vec<PathBuf>, Rc<Vec<Programa>>);

pub struct Rutas {
    pub config: PathBuf,
    pub temas: PathBuf,
}

pub struct Solar {
    pub(crate) rutas: Rutas,
    pub(crate) config: Config,
    pub(crate) config_guardada: Config,
    pub(crate) ultimo_cambio: Option<Instant>,
    pub(crate) temas: Vec<Tema>,
    pub(crate) paleta: Paleta,
    pub(crate) apariencia_aplicada: Option<(String, bool, u32, f32, String, bool)>,
    pub(crate) atajos: Atajos,
    pub(crate) carpetas: Arc<Mutex<Carpetas>>,

    pub(crate) ubicacion: Ubicacion,
    pub(crate) historial: Vec<Ubicacion>,
    pub(crate) pos_historial: usize,
    pub(crate) entradas: Vec<Entrada>,
    /// ruta -> posición en `entradas` (para no buscar uno por uno)
    pub(crate) indice: HashMap<PathBuf, usize>,
    pub(crate) originales_papelera: Vec<PathBuf>,
    pub(crate) visibles: Vec<usize>,
    pub(crate) visibles_sucios: bool,
    pub(crate) cargando_desde: Option<Instant>,
    pub(crate) generacion: u64,
    pub(crate) error_listado: Option<String>,
    pub(crate) espacio: Option<Espacio>,

    pub(crate) seleccion: HashSet<PathBuf>,
    pub(crate) ancla: Option<PathBuf>,
    pub(crate) foco: Option<PathBuf>,
    pub(crate) desplazar_a_foco: bool,
    pub(crate) seleccionar_al_cargar: Option<Vec<PathBuf>>,
    pub(crate) banda: Option<Banda>,
    pub(crate) columnas: usize,
    pub(crate) filas_visibles: usize,

    pub(crate) busqueda: Option<String>,
    pub(crate) enfocar_busqueda: bool,
    pub(crate) editando_ruta: Option<String>,
    pub(crate) enfocar_ruta: bool,
    pub(crate) portapapeles: Option<Portapapeles>,

    pub(crate) dispositivos: Vec<Dispositivo>,
    pub(crate) papelera_vacia: bool,
    pub(crate) montando: HashSet<String>,

    pub(crate) dialogo: Option<Dialogo>,
    pub(crate) ajustes: Option<EstadoAjustes>,
    pub(crate) notificaciones: Vec<Notificacion>,
    pub(crate) tareas: Vec<Tarea>,
    pub(crate) mostrar_tareas: bool,
    pub(crate) siguiente_tarea: u64,

    pub(crate) tx: Sender<Mensaje>,
    rx: Receiver<Mensaje>,
    pub(crate) vigia: Sender<ComandoVigia>,
    pub(crate) titulo: String,
    pub(crate) maximizada: bool,
    pub(crate) avisos_inicio: Vec<String>,

    // Abrir con…
    pub(crate) iconos_programas: RefCell<crate::iconos_programas::Cache>,
    pub(crate) menu_programas: RefCell<Option<MenuProgramas>>,
    // Actualizaciones
    pub(crate) actualizador: Actualizador,
    pub(crate) aviso_actualizacion: Option<String>,
    pub(crate) busqueda_manual: bool,
    pub(crate) tamano_cache: Option<u64>,
    // Animaciones
    pub(crate) animar_carga: bool,
    pub(crate) carga_animada: Option<Instant>,
    pub(crate) pop: HashMap<PathBuf, Instant>,
    pub(crate) transicion_tema: Option<(Tema, Instant)>,
    pub(crate) fuente_aplicada: Option<String>,
    pub(crate) dialogo_desde: (Option<std::mem::Discriminant<Dialogo>>, Instant),
    pub(crate) ajustes_desde: Instant,
}

pub struct Inicio {
    pub rutas: Rutas,
    pub config: Config,
    pub temas: Vec<Tema>,
    pub carpetas: Carpetas,
    pub avisos: Vec<String>,
    pub abrir: Option<PathBuf>,
}

impl Solar {
    pub fn nuevo(cc: &eframe::CreationContext<'_>, inicio: Inicio) -> Self {
        let ctx = cc.egui_ctx.clone();
        ctx.options_mut(|o| {
            o.zoom_with_keyboard = false;
        });
        let (tx, rx) = std::sync::mpsc::channel();
        let vigia = tareas::iniciar_vigia(tx.clone(), ctx.clone(), inicio.config.mostrar_discos_internos);
        let (atajos, mut avisos_atajos) = Atajos::de_config(&inicio.config.atajos);
        let mut avisos = inicio.avisos;
        avisos.append(&mut avisos_atajos);

        let tema = buscar_tema(&inicio.temas, &inicio.config.tema).clone();
        let paleta = tema::paleta(&tema, inicio.config.transparencia, inicio.config.opacidad);
        let hogar = nucleo::dir_hogar();
        let ubicacion = Ubicacion::Ruta(inicio.abrir.unwrap_or(hogar));

        let mut app = Self {
            rutas: inicio.rutas,
            config_guardada: inicio.config.clone(),
            config: inicio.config,
            ultimo_cambio: None,
            temas: inicio.temas,
            paleta,
            apariencia_aplicada: None,
            atajos,
            carpetas: Arc::new(Mutex::new(inicio.carpetas)),
            ubicacion: ubicacion.clone(),
            historial: vec![ubicacion.clone()],
            pos_historial: 0,
            entradas: Vec::new(),
            indice: HashMap::new(),
            originales_papelera: Vec::new(),
            visibles: Vec::new(),
            visibles_sucios: true,
            cargando_desde: None,
            generacion: 0,
            error_listado: None,
            espacio: None,
            seleccion: HashSet::new(),
            ancla: None,
            foco: None,
            desplazar_a_foco: false,
            seleccionar_al_cargar: None,
            banda: None,
            columnas: 1,
            filas_visibles: 10,
            busqueda: None,
            enfocar_busqueda: false,
            editando_ruta: None,
            enfocar_ruta: false,
            portapapeles: None,
            dispositivos: Vec::new(),
            papelera_vacia: nucleo::papelera_vacia(),
            montando: HashSet::new(),
            dialogo: None,
            ajustes: None,
            notificaciones: Vec::new(),
            tareas: Vec::new(),
            mostrar_tareas: false,
            siguiente_tarea: 1,
            tx,
            rx,
            vigia,
            titulo: String::new(),
            maximizada: false,
            avisos_inicio: avisos,
            iconos_programas: RefCell::default(),
            menu_programas: RefCell::new(None),
            actualizador: Actualizador::default(),
            aviso_actualizacion: None,
            busqueda_manual: false,
            tamano_cache: None,
            animar_carga: true,
            carga_animada: None,
            pop: HashMap::new(),
            transicion_tema: None,
            fuente_aplicada: None,
            dialogo_desde: (None, Instant::now()),
            ajustes_desde: Instant::now(),
        };
        sonido::configurar(app.config.sonidos, app.config.volumen);
        sonido::iniciar(true);
        crate::escritorio::integrar();
        if app.config.buscar_actualizaciones && actualizar::toca_revisar() {
            app.actualizador.buscar(app.config.repo_actualizaciones.clone(), ctx.clone());
        }
        app.aplicar_apariencia(&ctx);
        app.cargar(&ctx, true);
        if !app.avisos_inicio.is_empty() {
            app.dialogo = Some(Dialogo::AvisosConfig(app.avisos_inicio.clone()));
        }
        app
    }

    // ------------------------------------------------------------------
    //  Apariencia
    // ------------------------------------------------------------------

    pub fn tema_actual(&self) -> &Tema {
        buscar_tema(&self.temas, &self.config.tema)
    }

    pub(crate) fn aplicar_apariencia(&mut self, ctx: &egui::Context) {
        let tema = self.tema_actual().clone();
        let clave = (
            format!("{}{:?}", tema.id, tema.colores),
            self.config.transparencia,
            self.config.opacidad.to_bits(),
            self.config.escala,
            self.config.fuente.clone(),
            self.config.animaciones,
        );
        // Cambio de tema: los colores se mezclan poco a poco (≈ 0,35 s)
        if let Some((origen, desde)) = &self.transicion_tema {
            let t = desde.elapsed().as_secs_f32() / 0.35;
            if t < 1.0 && self.config.animaciones {
                let t = ease(t);
                let mut mezcla = tema.clone();
                for (c, o) in mezcla.colores.iter_mut().zip(origen.colores.iter()) {
                    *c = tema::mezclar(*o, *c, t);
                }
                self.paleta = tema::paleta(&mezcla, self.config.transparencia, self.config.opacidad);
                tema::aplicar(ctx, &mezcla, &self.paleta);
                self.aplicar_tiempo_animacion(ctx);
                self.apariencia_aplicada = None;
                ctx.request_repaint();
                return;
            }
            self.transicion_tema = None;
        }
        if self.apariencia_aplicada.as_ref() == Some(&clave) {
            return;
        }
        if let Some(antes) = &self.apariencia_aplicada
            && !antes.0.starts_with(&format!("{}[", tema.id))
            && self.config.animaciones
            && let Some(origen) = self.temas.iter().find(|t| antes.0.starts_with(&format!("{}[", t.id)))
        {
            // Empezar la transición desde el tema anterior
            self.transicion_tema = Some((origen.clone(), Instant::now()));
            sonido::pedir(Sonido::Efecto);
            self.apariencia_aplicada = None;
            ctx.request_repaint();
            return;
        }
        let fuente_cambio = self.fuente_aplicada.as_ref() != Some(&clave.4);
        self.fuente_aplicada = Some(clave.4.clone());
        self.paleta = tema::paleta(&tema, self.config.transparencia, self.config.opacidad);
        tema::aplicar(ctx, &tema, &self.paleta);
        self.aplicar_tiempo_animacion(ctx);
        ctx.set_zoom_factor(self.config.escala);
        if fuente_cambio && let Err(e) = crate::fuentes::configurar(ctx, &self.config.fuente) {
            self.avisar(e, TipoAviso::Error);
        }
        self.apariencia_aplicada = Some(clave);
    }

    fn aplicar_tiempo_animacion(&self, ctx: &egui::Context) {
        let tiempo = if self.config.animaciones { 0.16 } else { 0.0 };
        ctx.all_styles_mut(|s| s.animation_time = tiempo);
    }

    /// 0 → 1 de una animación que empezó en `desde` y dura `segundos`.
    pub(crate) fn progreso_animacion(&self, desde: Instant, segundos: f32) -> f32 {
        if !self.config.animaciones {
            return 1.0;
        }
        ease((desde.elapsed().as_secs_f32() / segundos).clamp(0.0, 1.0))
    }

    // ------------------------------------------------------------------
    //  Notificaciones
    // ------------------------------------------------------------------

    pub fn avisar(&mut self, texto: impl Into<String>, tipo: TipoAviso) {
        if tipo == TipoAviso::Error {
            sonido::pedir(Sonido::Error);
        }
        self.notificaciones.push(Notificacion { texto: texto.into(), tipo, creada: Instant::now() });
        if self.notificaciones.len() > 5 {
            self.notificaciones.remove(0);
        }
    }

    pub fn error(&mut self, titulo: &str, e: impl std::fmt::Display) {
        sonido::pedir(Sonido::Error);
        self.dialogo = Some(Dialogo::Mensaje { titulo: titulo.into(), texto: e.to_string(), error: true });
    }

    // ------------------------------------------------------------------
    //  Navegación y carga
    // ------------------------------------------------------------------

    pub fn navegar(&mut self, ctx: &egui::Context, destino: Ubicacion) {
        if let Ubicacion::Ruta(r) = &destino
            && !r.is_dir()
        {
            self.avisar(format!("No se puede abrir «{}»: no existe o no es una carpeta.", r.display()), TipoAviso::Error);
            return;
        }
        if destino == self.ubicacion {
            self.cargar(ctx, false);
            return;
        }
        self.historial.truncate(self.pos_historial + 1);
        self.historial.push(destino.clone());
        if self.historial.len() > 200 {
            self.historial.remove(0);
        }
        self.pos_historial = self.historial.len() - 1;
        self.ir(ctx, destino);
    }

    fn ir(&mut self, ctx: &egui::Context, destino: Ubicacion) {
        let anterior = self.ubicacion.clone();
        self.ubicacion = destino;
        self.seleccion.clear();
        self.ancla = None;
        self.foco = None;
        self.busqueda = None;
        self.editando_ruta = None;
        self.banda = None;
        // Al subir de carpeta, selecciona la carpeta de la que veníamos
        if let (Ubicacion::Ruta(a), Ubicacion::Ruta(n)) = (&anterior, &self.ubicacion)
            && a.parent() == Some(n.as_path())
        {
            self.seleccionar_al_cargar = Some(vec![a.clone()]);
        }
        self.entradas.clear();
        self.indice.clear();
        self.visibles.clear();
        self.visibles_sucios = true;
        self.animar_carga = true;
        self.cargar(ctx, true);
    }

    pub fn atras(&mut self, ctx: &egui::Context) {
        if self.pos_historial > 0 {
            self.pos_historial -= 1;
            let destino = self.historial[self.pos_historial].clone();
            self.ir(ctx, destino);
        }
    }

    pub fn adelante(&mut self, ctx: &egui::Context) {
        if self.pos_historial + 1 < self.historial.len() {
            self.pos_historial += 1;
            let destino = self.historial[self.pos_historial].clone();
            self.ir(ctx, destino);
        }
    }

    pub fn subir(&mut self, ctx: &egui::Context) {
        if let Some(padre) = self.ubicacion.ruta().and_then(|r| r.parent()).map(Path::to_path_buf) {
            self.navegar(ctx, Ubicacion::Ruta(padre));
        }
    }

    /// Vuelve a leer la ubicación actual. `nueva` indica que cambiamos de carpeta.
    pub fn cargar(&mut self, ctx: &egui::Context, nueva: bool) {
        self.generacion += 1;
        if nueva {
            self.cargando_desde = Some(Instant::now());
        }
        match &self.ubicacion {
            Ubicacion::Ruta(dir) => {
                if nueva {
                    let _ = self.vigia.send(ComandoVigia::Observar(Some(dir.clone())));
                }
                tareas::listar(
                    dir.clone(),
                    self.config.orden,
                    self.config.orden_descendente,
                    self.config.carpetas_primero,
                    self.generacion,
                    self.carpetas.clone(),
                    self.tx.clone(),
                    ctx.clone(),
                );
            }
            Ubicacion::Papelera => {
                if nueva {
                    let _ = self.vigia.send(ComandoVigia::Observar(None));
                }
                tareas::listar_papelera(self.generacion, self.tx.clone(), ctx.clone());
            }
        }
    }

    pub fn actualizar_visibles(&mut self) {
        if !self.visibles_sucios {
            return;
        }
        let consulta = self
            .busqueda
            .as_deref()
            .map(|b| crate::formato::plegar(b.trim()))
            .filter(|b| !b.is_empty());
        let ocultos = self.config.mostrar_ocultos || self.ubicacion == Ubicacion::Papelera;
        self.visibles = self
            .entradas
            .iter()
            .enumerate()
            .filter(|(_, e)| ocultos || !e.es_oculto)
            .filter(|(_, e)| consulta.as_ref().is_none_or(|q| e.plegado.contains(q.as_str())))
            .map(|(i, _)| i)
            .collect();
        self.visibles_sucios = false;
    }

    // ------------------------------------------------------------------
    //  Mensajes de los hilos
    // ------------------------------------------------------------------

    fn procesar_mensajes(&mut self, ctx: &egui::Context) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Mensaje::Listado { generacion, resultado, espacio } => {
                    if generacion != self.generacion {
                        continue;
                    }
                    self.cargando_desde = None;
                    self.espacio = espacio;
                    match resultado {
                        Ok(v) => {
                            self.entradas = v;
                            self.error_listado = None;
                        }
                        Err(e) => {
                            self.entradas.clear();
                            self.error_listado = Some(e.mensaje);
                        }
                    }
                    self.despues_de_cargar();
                }
                Mensaje::Papelera { generacion, elementos } => {
                    if generacion != self.generacion || self.ubicacion != Ubicacion::Papelera {
                        continue;
                    }
                    self.cargando_desde = None;
                    self.error_listado = None;
                    self.espacio = None;
                    self.papelera_vacia = elementos.is_empty();
                    self.originales_papelera = elementos.iter().map(|e| e.ruta_original.clone()).collect();
                    self.entradas = elementos
                        .into_iter()
                        .map(|e| {
                            let tipo = if e.es_carpeta { TipoEntrada::Carpeta } else { TipoEntrada::Archivo };
                            let estaba = e
                                .ruta_original
                                .parent()
                                .map(nucleo::contraer)
                                .unwrap_or_default();
                            Entrada {
                                plegado: crate::formato::plegar(&e.nombre),
                                nombre: e.nombre,
                                ruta: e.ruta,
                                tamano: e.tamano,
                                modificado: e.fecha,
                                modo: 0,
                                tipo,
                                categoria: e.categoria,
                                es_enlace: false,
                                es_oculto: false,
                                descripcion: format!("Estaba en {estaba}"),
                                estilo: None,
                            }
                        })
                        .collect();
                    self.despues_de_cargar();
                }
                Mensaje::CambioCarpeta => {
                    if matches!(self.ubicacion, Ubicacion::Ruta(_)) {
                        self.cargar(ctx, false);
                    }
                }
                Mensaje::CambioPapelera => {
                    self.papelera_vacia = nucleo::papelera_vacia();
                    if self.ubicacion == Ubicacion::Papelera {
                        self.cargar(ctx, false);
                    }
                }
                Mensaje::Dispositivos(d) => {
                    // Si la unidad donde estábamos desapareció, volvemos a casa
                    if let Ubicacion::Ruta(r) = &self.ubicacion {
                        let perdida = self.dispositivos.iter().any(|v| {
                            v.punto_montaje.as_ref().is_some_and(|p| r.starts_with(p))
                                && !d.iter().any(|n| n.punto_montaje == v.punto_montaje)
                        });
                        if perdida {
                            self.avisar("La unidad se desconectó.", TipoAviso::Info);
                            self.navegar(ctx, Ubicacion::Ruta(nucleo::dir_hogar()));
                        }
                    }
                    self.dispositivos = d;
                }
                Mensaje::TareaTerminada { id, resultado, destinos, tipo } => {
                    self.tareas.retain(|t| t.id != id);
                    match resultado {
                        Ok(texto) => {
                            sonido::pedir(Sonido::Listo);
                            self.avisar(texto, TipoAviso::Exito);
                            if matches!(tipo, TipoTarea::Copiar { .. } | TipoTarea::Mover { .. })
                                && let Some(dir) = self.ubicacion.ruta()
                            {
                                let aqui: Vec<PathBuf> =
                                    destinos.iter().filter(|d| d.parent() == Some(dir)).cloned().collect();
                                if !aqui.is_empty() {
                                    self.seleccionar_al_cargar = Some(aqui);
                                }
                            }
                        }
                        Err(e) if e.cancelado() => self.avisar("Operación cancelada.", TipoAviso::Info),
                        Err(e) => {
                            if let TipoTarea::Papelera { rutas } = &tipo
                                && e.codigo != 125
                            {
                                self.dialogo =
                                    Some(Dialogo::PapeleraFallo { rutas: rutas.clone(), mensaje: e.mensaje.clone() });
                            } else {
                                self.error("No se pudo terminar", e);
                            }
                        }
                    }
                    self.papelera_vacia = nucleo::papelera_vacia();
                    self.cargar(ctx, false);
                }
                Mensaje::Montaje { dispositivo, resultado, abrir, desmontar } => {
                    self.montando.remove(&dispositivo);
                    match resultado {
                        Ok(()) => {
                            if desmontar {
                                sonido::pedir(Sonido::Listo);
                                self.avisar("Ya puedes retirar la unidad con seguridad.", TipoAviso::Exito);
                            } else if abrir {
                                let d = nucleo::dispositivos(self.config.mostrar_discos_internos);
                                if let Some(p) = d
                                    .iter()
                                    .find(|x| x.dispositivo == dispositivo)
                                    .and_then(|x| x.punto_montaje.clone())
                                {
                                    self.navegar(ctx, Ubicacion::Ruta(p));
                                }
                                self.dispositivos = d;
                            }
                        }
                        Err(e) => self.error(if desmontar { "No se pudo expulsar" } else { "No se pudo montar" }, e),
                    }
                }
            }
        }
    }

    fn despues_de_cargar(&mut self) {
        if self.animar_carga {
            self.animar_carga = false;
            self.carga_animada = Some(Instant::now());
        }
        self.indice = self.entradas.iter().enumerate().map(|(i, e)| (e.ruta.clone(), i)).collect();
        let existentes = &self.indice;
        self.seleccion.retain(|r| existentes.contains_key(r));
        if let Some(sel) = self.seleccionar_al_cargar.take() {
            let validas: Vec<PathBuf> = sel.into_iter().filter(|r| existentes.contains_key(r)).collect();
            if !validas.is_empty() {
                self.seleccion = validas.iter().cloned().collect();
                self.foco = validas.first().cloned();
                self.ancla = self.foco.clone();
                self.desplazar_a_foco = true;
            }
        }
        if self.foco.as_ref().is_some_and(|f| !existentes.contains_key(f)) {
            self.foco = None;
        }
        self.visibles_sucios = true;
    }

    // ------------------------------------------------------------------
    //  Selección
    // ------------------------------------------------------------------

    pub fn entrada(&self, ruta: &Path) -> Option<&Entrada> {
        self.indice.get(ruta).and_then(|&i| self.entradas.get(i))
    }

    /// La selección, en el orden en que se ve.
    pub fn seleccionados(&self) -> Vec<PathBuf> {
        self.visibles
            .iter()
            .map(|&i| &self.entradas[i].ruta)
            .filter(|r| self.seleccion.contains(*r))
            .cloned()
            .collect()
    }

    pub fn indice_visible(&self, ruta: &Path) -> Option<usize> {
        self.visibles.iter().position(|&i| self.entradas[i].ruta == ruta)
    }

    pub fn seleccionar_solo(&mut self, ruta: &Path) {
        self.seleccion.clear();
        self.seleccion.insert(ruta.to_path_buf());
        self.ancla = Some(ruta.to_path_buf());
        self.foco = Some(ruta.to_path_buf());
    }

    pub fn seleccionar_rango(&mut self, hasta: &Path, sumar: bool) {
        let ancla = self.ancla.clone().unwrap_or_else(|| hasta.to_path_buf());
        let (Some(a), Some(b)) = (self.indice_visible(&ancla), self.indice_visible(hasta)) else {
            self.seleccionar_solo(hasta);
            return;
        };
        if !sumar {
            self.seleccion.clear();
        }
        let (i, j) = if a <= b { (a, b) } else { (b, a) };
        for k in i..=j {
            let r = self.entradas[self.visibles[k]].ruta.clone();
            self.seleccion.insert(r);
        }
        self.foco = Some(hasta.to_path_buf());
    }

    /// Lo que afectan las acciones: solo lo que se ve seleccionado.
    pub fn objetivo(&self) -> Vec<PathBuf> {
        self.seleccionados()
    }

    pub fn carpetas_seleccionadas(&self) -> Vec<PathBuf> {
        self.objetivo()
            .into_iter()
            .filter(|r| self.entrada(r).is_some_and(|e| e.es_carpeta()))
            .collect()
    }

    // ------------------------------------------------------------------
    //  Configuración
    // ------------------------------------------------------------------

    pub fn config_cambio(&mut self) {
        self.ultimo_cambio = Some(Instant::now());
    }

    fn guardar_si_hace_falta(&mut self, ctx: &egui::Context) {
        let Some(t) = self.ultimo_cambio else { return };
        if t.elapsed() < Duration::from_millis(400) {
            ctx.request_repaint_after(Duration::from_millis(450));
            return;
        }
        // Mientras se edita un tema, no guardamos el tema provisional
        if self.config.tema == crate::ajustes::ID_EDITOR {
            return;
        }
        self.ultimo_cambio = None;
        self.config.atajos = self.atajos.a_config();
        if self.config != self.config_guardada {
            match self.config.guardar(&self.rutas.config) {
                Ok(()) => self.config_guardada = self.config.clone(),
                Err(e) => self.avisar(format!("No se pudo guardar config.lua: {e}"), TipoAviso::Error),
            }
        }
    }

    pub fn recargar_temas(&mut self) {
        let (temas, avisos) = nucleo::cargar_temas(Some(&self.rutas.temas));
        self.temas = temas;
        self.apariencia_aplicada = None;
        if let Some(a) = avisos {
            self.avisar(format!("Temas: {a}"), TipoAviso::Error);
        }
    }

    pub fn recargar_config(&mut self, ctx: &egui::Context) {
        let carga = Config::cargar(&self.rutas.config);
        self.config = carga.config.clone();
        self.config_guardada = carga.config;
        let (atajos, avisos_atajos) = Atajos::de_config(&self.config.atajos);
        self.atajos = atajos;
        self.recargar_temas();
        let mut avisos: Vec<String> = carga.avisos.into_iter().collect();
        avisos.extend(avisos_atajos);
        if avisos.is_empty() {
            self.avisar("Configuración recargada.", TipoAviso::Exito);
        } else {
            self.dialogo = Some(Dialogo::AvisosConfig(avisos));
        }
        let _ = self
            .vigia
            .send(ComandoVigia::Dispositivos { incluir_internos: self.config.mostrar_discos_internos });
        self.visibles_sucios = true;
        self.cargar(ctx, false);
    }

    // ------------------------------------------------------------------
    //  Acciones
    // ------------------------------------------------------------------

    pub fn accion_de_atajo(&self, id: &str) -> Option<Accion> {
        let objetivo = self.objetivo();
        let uno = objetivo.first().cloned();
        let dir = self.ubicacion.ruta().map(Path::to_path_buf);
        let en_papelera = self.ubicacion == Ubicacion::Papelera;
        Some(match id {
            "atras" => Accion::Atras,
            "adelante" => Accion::Adelante,
            "subir" => Accion::Subir,
            "inicio" => Accion::Inicio,
            "recargar" => Accion::Recargar,
            "ubicacion" => Accion::EditarRuta,
            "buscar" => Accion::Buscar,
            "ocultos" => Accion::AlternarOcultos,
            "nueva_carpeta" if !en_papelera => Accion::NuevaCarpeta,
            "nuevo_archivo" if !en_papelera => Accion::NuevoArchivo,
            "renombrar" if !en_papelera && objetivo.len() == 1 => Accion::Renombrar(uno?),
            "copiar" if !en_papelera && !objetivo.is_empty() => Accion::Copiar(objetivo),
            "cortar" if !en_papelera && !objetivo.is_empty() => Accion::Cortar(objetivo),
            "pegar" if !en_papelera => Accion::Pegar(dir?),
            "duplicar" if !en_papelera && !objetivo.is_empty() => Accion::Duplicar(objetivo),
            "seleccionar_todo" => Accion::SeleccionarTodo,
            "papelera" if en_papelera && !objetivo.is_empty() => Accion::BorrarDePapelera(objetivo),
            "papelera" if !objetivo.is_empty() => Accion::Papelera(objetivo),
            "eliminar" if en_papelera && !objetivo.is_empty() => Accion::BorrarDePapelera(objetivo),
            "eliminar" if !objetivo.is_empty() => Accion::Eliminar(objetivo),
            "propiedades" if !en_papelera => {
                if objetivo.is_empty() { Accion::Propiedades(vec![dir?]) } else { Accion::Propiedades(objetivo) }
            }
            "personalizar" if !en_papelera => {
                let c = self.carpetas_seleccionadas();
                if c.is_empty() { Accion::Personalizar(vec![dir?]) } else { Accion::Personalizar(c) }
            }
            "agregar_lugar" if !en_papelera => {
                let c = self.carpetas_seleccionadas();
                Accion::AgregarLugar(c.into_iter().next().or(dir)?)
            }
            "vista_cuadricula" => Accion::CambiarVista(Vista::Cuadricula),
            "vista_lista" => Accion::CambiarVista(Vista::Lista),
            "acercar" => Accion::Zoom(8.0),
            "alejar" => Accion::Zoom(-8.0),
            "barra_lateral" => Accion::AlternarLateral,
            "terminal" => Accion::AbrirTerminal(dir?),
            "ajustes" => Accion::Ajustes,
            "nueva_ventana" => Accion::NuevaVentana,
            "cerrar" => Accion::Cerrar,
            _ => return None,
        })
    }

    pub fn ejecutar(&mut self, ctx: &egui::Context, accion: Accion) {
        // Cada acción con su sonido (al final del cuadro suena el más importante)
        match &accion {
            Accion::Navegar(_)
            | Accion::Atras
            | Accion::Adelante
            | Accion::Subir
            | Accion::Inicio
            | Accion::Abrir(_)
            | Accion::AbrirSeleccion
            | Accion::AbrirCon(_)
            | Accion::AbrirConPrograma { .. }
            | Accion::AbrirEnVentana(_)
            | Accion::AbrirTerminal(_)
            | Accion::Copiar(_)
            | Accion::Cortar(_)
            | Accion::Renombrar(_)
            | Accion::NuevaCarpeta
            | Accion::NuevoArchivo
            | Accion::Propiedades(_)
            | Accion::Personalizar(_)
            | Accion::CambiarVista(_)
            | Accion::AlternarLateral
            | Accion::Montar { .. }
            | Accion::BuscarActualizaciones
            | Accion::AcercaDe => sonido::pedir(Sonido::Clic),
            Accion::Papelera(_)
            | Accion::Eliminar(_)
            | Accion::BorrarDePapelera(_)
            | Accion::VaciarPapelera
            | Accion::Desmontar { .. }
            | Accion::QuitarLugar(_)
            | Accion::BorrarCache => sonido::pedir(Sonido::Retraccion),
            Accion::Estilo(..)
            | Accion::AgregarLugar(_)
            | Accion::Pegar(_)
            | Accion::PegarTexto(_)
            | Accion::Duplicar(_)
            | Accion::Soltar { .. }
            | Accion::TraerExternos(_)
            | Accion::Restaurar(_)
            | Accion::Actualizar(_) => sonido::pedir(Sonido::Efecto),
            Accion::Ajustes => sonido::pedir(if self.ajustes.is_some() { Sonido::Retraccion } else { Sonido::Clic }),
            _ => {}
        }
        match accion {
            Accion::Navegar(u) => self.navegar(ctx, u),
            Accion::Atras => self.atras(ctx),
            Accion::Adelante => self.adelante(ctx),
            Accion::Subir => self.subir(ctx),
            Accion::Inicio => self.navegar(ctx, Ubicacion::Ruta(nucleo::dir_hogar())),
            Accion::Recargar => {
                self.papelera_vacia = nucleo::papelera_vacia();
                self.cargar(ctx, false);
            }
            Accion::Abrir(ruta) => self.abrir(ctx, &ruta),
            Accion::AbrirSeleccion => {
                let objetivo = self.objetivo();
                if objetivo.len() == 1 {
                    self.abrir(ctx, &objetivo[0]);
                } else {
                    let archivos: Vec<PathBuf> = objetivo.into_iter().filter(|r| !r.is_dir()).collect();
                    for r in archivos.iter().take(20) {
                        if let Err(e) = nucleo::abrir(r) {
                            self.error("No se pudo abrir", e);
                            break;
                        }
                    }
                }
            }
            Accion::AbrirCon(rutas) => {
                if rutas.is_empty() {
                    return;
                }
                let tipo = if rutas.len() == 1 { self.entrada(&rutas[0]).map(|e| e.descripcion.clone()).unwrap_or_default() } else { String::new() };
                self.dialogo = Some(Dialogo::AbrirCon(Box::new(EstadoAbrirCon::nuevo(rutas, tipo))));
            }
            Accion::AbrirConPrograma { programa, rutas } => {
                if let Err(e) = nucleo::programa_lanzar(&programa, &rutas, &self.config.terminal) {
                    self.avisar(e.mensaje, TipoAviso::Error);
                }
            }
            Accion::AbrirEnVentana(ruta) => self.nueva_ventana(Some(&ruta)),
            Accion::AbrirTerminal(dir) => {
                if let Err(e) = nucleo::abrir_terminal(&self.config.terminal, &dir) {
                    self.error("No se pudo abrir la terminal", e);
                }
            }
            Accion::Copiar(rutas) => self.al_portapapeles(ctx, rutas, ModoPortapapeles::Copiar),
            Accion::Cortar(rutas) => self.al_portapapeles(ctx, rutas, ModoPortapapeles::Cortar),
            Accion::Pegar(destino) => self.pegar(ctx, destino, None),
            Accion::PegarTexto(texto) => {
                if let Some(dir) = self.ubicacion.ruta().map(Path::to_path_buf) {
                    self.pegar(ctx, dir, Some(texto));
                }
            }
            Accion::Duplicar(rutas) => {
                if let Some(dir) = rutas.first().and_then(|r| r.parent()).map(Path::to_path_buf) {
                    self.iniciar_tarea(ctx, TipoTarea::Copiar { origenes: rutas, destino: dir, conflicto: Conflicto::Renombrar });
                }
            }
            Accion::Renombrar(ruta) => {
                let nombre = tareas::nombre(&ruta);
                self.dialogo = Some(Dialogo::Renombrar { ruta, nombre, error: None, iniciado: false });
            }
            Accion::Papelera(rutas) => {
                if self.config.confirmar_papelera {
                    self.dialogo = Some(Dialogo::Confirmar { rutas, permanente: false });
                } else {
                    self.iniciar_tarea(ctx, TipoTarea::Papelera { rutas });
                }
            }
            Accion::Eliminar(rutas) => {
                if self.config.confirmar_eliminar {
                    self.dialogo = Some(Dialogo::Confirmar { rutas, permanente: true });
                } else {
                    self.iniciar_tarea(ctx, TipoTarea::Eliminar { rutas });
                }
            }
            Accion::Restaurar(rutas) => self.iniciar_tarea(ctx, TipoTarea::Restaurar { rutas }),
            Accion::BorrarDePapelera(rutas) => {
                self.dialogo = Some(Dialogo::ConfirmarBorrarPapelera(rutas));
            }
            Accion::VaciarPapelera => self.dialogo = Some(Dialogo::ConfirmarVaciar),
            Accion::Propiedades(rutas) => {
                if !rutas.is_empty() {
                    self.dialogo = Some(Dialogo::Propiedades(Box::new(EstadoPropiedades::nuevo(ctx, rutas))));
                }
            }
            Accion::CopiarRuta(rutas) => {
                let texto = rutas.iter().map(|r| r.to_string_lossy()).collect::<Vec<_>>().join("\n");
                ctx.copy_text(texto);
                self.avisar(if rutas.len() == 1 { "Ruta copiada." } else { "Rutas copiadas." }, TipoAviso::Info);
            }
            Accion::AgregarLugar(ruta) => self.agregar_lugar(&ruta),
            Accion::QuitarLugar(i) => {
                if i < self.config.lugares.len() {
                    let l = self.config.lugares.remove(i);
                    self.avisar(format!("Se quitó «{}» de Lugares.", l.nombre), TipoAviso::Info);
                    self.config_cambio();
                }
            }
            Accion::Estilo(rutas, cambio) => self.cambiar_estilo(&rutas, cambio),
            Accion::Personalizar(rutas) => {
                if !rutas.is_empty() {
                    let estilo = self.estilo_de(&rutas[0]).unwrap_or_default();
                    self.dialogo = Some(Dialogo::Personalizar(Box::new(EstadoPersonalizar::nuevo(rutas, estilo))));
                }
            }
            Accion::NuevaCarpeta => {
                if let Some(dir) = self.ubicacion.ruta() {
                    let nombre = nucleo::nombre_libre(dir, "Carpeta nueva", false);
                    self.dialogo = Some(Dialogo::Nuevo { carpeta: true, nombre, error: None, iniciado: false });
                }
            }
            Accion::NuevoArchivo => {
                if let Some(dir) = self.ubicacion.ruta() {
                    let nombre = nucleo::nombre_libre(dir, "Archivo nuevo.txt", false);
                    self.dialogo = Some(Dialogo::Nuevo { carpeta: false, nombre, error: None, iniciado: false });
                }
            }
            Accion::SeleccionarTodo => {
                self.seleccion = self.visibles.iter().map(|&i| self.entradas[i].ruta.clone()).collect();
            }
            Accion::AlternarOcultos => {
                self.config.mostrar_ocultos = !self.config.mostrar_ocultos;
                self.visibles_sucios = true;
                self.config_cambio();
                self.avisar(
                    if self.config.mostrar_ocultos { "Mostrando archivos ocultos." } else { "Archivos ocultos escondidos." },
                    TipoAviso::Info,
                );
            }
            Accion::CambiarVista(v) => {
                self.config.vista = v;
                self.desplazar_a_foco = true;
                self.config_cambio();
            }
            Accion::Ordenar(o) => {
                if self.config.orden == o {
                    self.config.orden_descendente = !self.config.orden_descendente;
                } else {
                    self.config.orden = o;
                    self.config.orden_descendente = matches!(o, Orden::Fecha | Orden::Tamano);
                }
                self.config_cambio();
                self.cargar(ctx, false);
            }
            Accion::OrdenDescendente(d) => {
                self.config.orden_descendente = d;
                self.config_cambio();
                self.cargar(ctx, false);
            }
            Accion::CarpetasPrimero(c) => {
                self.config.carpetas_primero = c;
                self.config_cambio();
                self.cargar(ctx, false);
            }
            Accion::Zoom(delta) => {
                self.config.tamano_iconos = (self.config.tamano_iconos + delta).clamp(40.0, 160.0);
                self.desplazar_a_foco = true;
                self.config_cambio();
            }
            Accion::AlternarLateral => {
                self.config.mostrar_barra_lateral = !self.config.mostrar_barra_lateral;
                self.config_cambio();
            }
            Accion::EditarRuta => {
                if let Some(r) = self.ubicacion.ruta() {
                    let mut texto = r.to_string_lossy().into_owned();
                    if texto != "/" {
                        texto.push('/');
                    }
                    self.editando_ruta = Some(texto);
                    self.enfocar_ruta = true;
                }
            }
            Accion::Buscar => {
                if self.busqueda.is_none() {
                    self.busqueda = Some(String::new());
                }
                self.enfocar_busqueda = true;
            }
            Accion::Ajustes => {
                if let Some(mut estado) = self.ajustes.take() {
                    if let Some(ed) = estado.editor.take() {
                        self.cancelar_editor(ed);
                    }
                } else {
                    self.ajustes = Some(EstadoAjustes::nuevo(self));
                    self.ajustes_desde = Instant::now();
                }
            }
            Accion::NuevaVentana => {
                let dir = self.ubicacion.ruta().map(Path::to_path_buf);
                self.nueva_ventana(dir.as_deref());
            }
            Accion::Cerrar => ctx.send_viewport_cmd(ViewportCommand::Close),
            Accion::Montar { dispositivo, abrir } => {
                if self.montando.insert(dispositivo.clone()) {
                    tareas::montaje(dispositivo, false, false, abrir, self.tx.clone(), ctx.clone());
                }
            }
            Accion::Desmontar { dispositivo, expulsar } => {
                // Si estamos dentro de la unidad, salimos antes de expulsar
                if let (Some(r), Some(p)) = (
                    self.ubicacion.ruta(),
                    self.dispositivos
                        .iter()
                        .find(|d| d.dispositivo == dispositivo)
                        .and_then(|d| d.punto_montaje.clone()),
                ) && r.starts_with(&p)
                {
                    self.navegar(ctx, Ubicacion::Ruta(nucleo::dir_hogar()));
                }
                if self.montando.insert(dispositivo.clone()) {
                    tareas::montaje(dispositivo, true, expulsar, false, self.tx.clone(), ctx.clone());
                }
            }
            Accion::Soltar { destino, rutas, copiar } => {
                let rutas: Vec<PathBuf> =
                    rutas.into_iter().filter(|r| r != &destino && r.parent() != Some(destino.as_path())).collect();
                if rutas.is_empty() {
                    return;
                }
                let copiar = copiar || !mismo_dispositivo(&rutas[0], &destino);
                self.preparar_operacion(ctx, rutas, destino, copiar);
            }
            Accion::TraerExternos(rutas) => {
                if let Some(dir) = self.ubicacion.ruta().map(Path::to_path_buf) {
                    self.preparar_operacion(ctx, rutas, dir, true);
                }
            }
            Accion::AcercaDe => self.dialogo = Some(Dialogo::AcercaDe),
            Accion::Tarea(t) => self.iniciar_tarea(ctx, t),
            Accion::BuscarActualizaciones => {
                self.busqueda_manual = true;
                self.aviso_actualizacion = None;
                self.actualizador.buscar(self.config.repo_actualizaciones.clone(), ctx.clone());
            }
            Accion::Actualizar(v) => self.actualizador.aplicar(*v, ctx.clone()),
            Accion::CancelarActualizacion => self.actualizador.cancelar(),
            Accion::Reiniciar => {
                if let Err(e) = actualizar::reiniciar(ctx, self.ubicacion.ruta()) {
                    self.error("No se pudo reiniciar Solar", e);
                }
            }
            Accion::BorrarCache => {
                if self.actualizador.ocupado() {
                    self.avisar("Espera a que termine la actualización.", TipoAviso::Info);
                    return;
                }
                match actualizar::borrar_cache() {
                    Ok(bytes) => {
                        self.iconos_programas.borrow_mut().vaciar();
                        self.menu_programas.replace(None);
                        self.tamano_cache = Some(0);
                        let texto = if bytes > 0 {
                            format!("Caché borrada: se liberaron {}.", crate::formato::tamano(bytes))
                        } else {
                            "La caché ya estaba vacía.".into()
                        };
                        self.avisar(texto, TipoAviso::Exito);
                    }
                    Err(e) => self.avisar(e, TipoAviso::Error),
                }
            }
        }
    }

    pub fn abrir(&mut self, ctx: &egui::Context, ruta: &Path) {
        if self.ubicacion == Ubicacion::Papelera {
            self.avisar("Restaura el elemento para poder abrirlo.", TipoAviso::Info);
            return;
        }
        if ruta.is_dir() {
            self.navegar(ctx, Ubicacion::Ruta(ruta.to_path_buf()));
        } else if let Err(e) = nucleo::abrir(ruta) {
            self.error("No se pudo abrir", e);
        }
    }

    fn nueva_ventana(&mut self, dir: Option<&Path>) {
        let Ok(exe) = std::env::current_exe() else { return };
        let mut argv = vec![exe.as_os_str().to_owned()];
        if let Some(d) = dir {
            argv.push(d.as_os_str().to_owned());
        }
        let refs: Vec<&std::ffi::OsStr> = argv.iter().map(|a| a.as_os_str()).collect();
        if let Err(e) = nucleo::lanzar(&refs, None) {
            self.error("No se pudo abrir otra ventana", e);
        }
    }

    fn al_portapapeles(&mut self, ctx: &egui::Context, rutas: Vec<PathBuf>, modo: ModoPortapapeles) {
        if rutas.is_empty() {
            return;
        }
        // También dejamos las rutas como texto: así puedes pegarlas en una terminal
        let texto = rutas.iter().map(|r| r.to_string_lossy()).collect::<Vec<_>>().join("\n");
        ctx.copy_text(texto.clone());
        let n = rutas.len();
        self.portapapeles = Some(Portapapeles { rutas, modo, texto });
        let verbo = if modo == ModoPortapapeles::Copiar { "copiado" } else { "cortado" };
        self.avisar(
            if n == 1 { format!("1 elemento {verbo}.") } else { format!("{} {verbo}s.", crate::formato::elementos(n)) },
            TipoAviso::Info,
        );
    }

    /// Pegar en `destino`. Si viene texto del portapapeles del sistema y no es
    /// lo que copiamos nosotros, intentamos leerlo como rutas (de otro gestor
    /// de archivos o de una terminal).
    fn pegar(&mut self, ctx: &egui::Context, destino: PathBuf, texto: Option<String>) {
        let propio = self
            .portapapeles
            .as_ref()
            .filter(|p| texto.as_ref().is_none_or(|t| t.trim() == p.texto.trim()))
            .cloned();
        if let Some(p) = propio {
            let rutas: Vec<PathBuf> = p.rutas.iter().filter(|r| r.exists() || r.is_symlink()).cloned().collect();
            if rutas.is_empty() {
                self.avisar("Lo que copiaste ya no existe.", TipoAviso::Error);
                return;
            }
            if p.modo == ModoPortapapeles::Cortar {
                self.portapapeles = None;
            }
            self.preparar_operacion(ctx, rutas, destino, p.modo == ModoPortapapeles::Copiar);
            return;
        }
        if let Some(t) = texto {
            let rutas: Vec<PathBuf> = t
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(|l| {
                    let l = l.strip_prefix("file://").map(decodificar_uri).unwrap_or_else(|| l.to_string());
                    nucleo::expandir(&l)
                })
                .collect();
            if !rutas.is_empty() && rutas.iter().all(|r| r.is_absolute() && (r.exists() || r.is_symlink())) {
                self.preparar_operacion(ctx, rutas, destino, true);
                return;
            }
        }
        self.avisar("No hay archivos para pegar.", TipoAviso::Info);
    }

    /// Revisa conflictos de nombre antes de copiar o mover.
    pub fn preparar_operacion(&mut self, ctx: &egui::Context, origenes: Vec<PathBuf>, destino: PathBuf, copiar: bool) {
        let nombres: Vec<String> = origenes
            .iter()
            .filter(|o| o.parent() != Some(destino.as_path()))
            .filter_map(|o| o.file_name().map(|n| (n, o)))
            .filter(|(n, _)| {
                let d = destino.join(n);
                d.exists() || d.is_symlink()
            })
            .map(|(n, _)| n.to_string_lossy().into_owned())
            .collect();
        if nombres.is_empty() {
            let t = if copiar {
                TipoTarea::Copiar { origenes, destino, conflicto: Conflicto::Renombrar }
            } else {
                TipoTarea::Mover { origenes, destino, conflicto: Conflicto::Renombrar }
            };
            self.iniciar_tarea(ctx, t);
        } else {
            self.dialogo = Some(Dialogo::Conflicto { origenes, destino, copiar, nombres });
        }
    }

    pub fn iniciar_tarea(&mut self, ctx: &egui::Context, tipo: TipoTarea) {
        let id = self.siguiente_tarea;
        self.siguiente_tarea += 1;
        let t = tareas::lanzar_tarea(id, tipo, self.carpetas.clone(), self.tx.clone(), ctx.clone());
        self.tareas.push(t);
    }

    pub fn cancelar_tarea(&mut self, id: u64) {
        if let Some(t) = self.tareas.iter().find(|t| t.id == id) {
            t.cancelar.store(true, Ordering::Relaxed);
        }
    }

    /// Posición de la carpeta en Lugares, si ya está ahí.
    pub fn indice_lugar(&self, ruta: &Path) -> Option<usize> {
        self.config.lugares.iter().position(|l| nucleo::expandir(&l.ruta) == ruta)
    }

    fn agregar_lugar(&mut self, ruta: &Path) {
        let corta = nucleo::contraer(ruta);
        if self.indice_lugar(ruta).is_some() {
            self.avisar("Esa carpeta ya está en Lugares.", TipoAviso::Info);
            return;
        }
        let icono = self.estilo_de(ruta).and_then(|e| e.icono).filter(|i| !i.starts_with("txt:")).unwrap_or_default();
        let nombre = tareas::nombre(ruta);
        self.config.lugares.push(Lugar { nombre: nombre.clone(), ruta: corta, icono });
        self.config_cambio();
        self.avisar(format!("«{nombre}» se añadió a Lugares."), TipoAviso::Exito);
    }

    pub fn estilo_de(&self, ruta: &Path) -> Option<EstiloCarpeta> {
        if let Some(e) = self.entrada(ruta) {
            return e.estilo.clone();
        }
        self.carpetas.lock().ok()?.obtener(ruta)
    }

    pub fn poner_estilo(&mut self, rutas: &[PathBuf], estilo: &EstiloCarpeta) {
        let mut error = None;
        if let Ok(mut db) = self.carpetas.lock() {
            for r in rutas {
                let res = if estilo.vacio() { db.quitar(r) } else { db.poner(r, estilo) };
                if let Err(e) = res {
                    error = Some(e);
                }
            }
        }
        for e in self.entradas.iter_mut().filter(|e| rutas.contains(&e.ruta)) {
            e.estilo = (!estilo.vacio()).then(|| estilo.clone());
        }
        // Un pequeño "salto" de la carpeta para que se note el cambio
        let ahora = Instant::now();
        for r in rutas {
            self.pop.insert(r.clone(), ahora);
        }
        sonido::pedir(Sonido::Efecto);
        if let Some(e) = error {
            self.avisar(format!("No se pudo guardar el estilo: {e}"), TipoAviso::Error);
        }
    }

    fn cambiar_estilo(&mut self, rutas: &[PathBuf], cambio: CambioEstilo) {
        for r in rutas {
            let mut e = self.estilo_de(r).unwrap_or_default();
            match &cambio {
                CambioEstilo::Color(c) => e.color = *c,
                CambioEstilo::Logotipo(l) => e.icono = l.clone(),
                CambioEstilo::ColorLogotipo(c) => e.color_icono = *c,
                CambioEstilo::Quitar => e = EstiloCarpeta::default(),
            }
            self.poner_estilo(std::slice::from_ref(r), &e);
        }
    }

    /// Avisos del actualizador (corre en otro hilo).
    fn revisar_actualizador(&mut self) {
        let en_pestana = self.ajustes.as_ref().is_some_and(|a| a.pestana == crate::ajustes::Pestana::Actualizaciones);
        match self.actualizador.estado() {
            actualizar::Estado::Disponible(v) => {
                let clave = format!("hay:{}", v.etiqueta);
                if self.aviso_actualizacion.as_deref() != Some(clave.as_str()) {
                    self.aviso_actualizacion = Some(clave);
                    if !en_pestana {
                        sonido::pedir(Sonido::Efecto);
                        self.avisar(
                            if v.rama.is_some() {
                                format!("Solar tiene {} en GitHub. Instálalos en Ajustes → Actualizaciones.", v.nombre)
                            } else {
                                format!("Hay una versión nueva de Solar: {}. Instálala en Ajustes → Actualizaciones.", v.etiqueta)
                            },
                            TipoAviso::Info,
                        );
                    }
                }
            }
            actualizar::Estado::Listo { version } => {
                let clave = format!("listo:{version}");
                if self.aviso_actualizacion.as_deref() != Some(clave.as_str()) {
                    self.aviso_actualizacion = Some(clave);
                    self.tamano_cache = None; // la compilación creció la caché
                    sonido::pedir(Sonido::Listo);
                    self.avisar(format!("Solar {version} quedó instalado. Reinícialo para usarlo."), TipoAviso::Exito);
                }
            }
            actualizar::Estado::Error(e) => {
                let clave = format!("error:{e}");
                if self.aviso_actualizacion.as_deref() != Some(clave.as_str()) {
                    self.aviso_actualizacion = Some(clave);
                    self.tamano_cache = None;
                    // La revisión automática al abrir falla en silencio (sin internet, etc.)
                    if self.busqueda_manual && !en_pestana {
                        self.avisar(e, TipoAviso::Error);
                    } else if self.busqueda_manual {
                        sonido::pedir(Sonido::Error);
                    }
                }
            }
            actualizar::Estado::AlDia if self.busqueda_manual && self.aviso_actualizacion.as_deref() != Some("aldia") => {
                self.aviso_actualizacion = Some("aldia".into());
                if !en_pestana {
                    self.avisar(format!("Ya tienes lo más nuevo ({}).", actualizar::version_completa()), TipoAviso::Info);
                }
            }
            _ => {}
        }
    }

    fn actualizar_titulo(&mut self, ctx: &egui::Context) {
        let titulo = match &self.ubicacion {
            Ubicacion::Papelera => "Papelera — Solar".to_string(),
            Ubicacion::Ruta(r) => {
                let nombre = if *r == nucleo::dir_hogar() { "Carpeta personal".into() } else { tareas::nombre(r) };
                format!("{nombre} — Solar")
            }
        };
        if titulo != self.titulo {
            ctx.send_viewport_cmd(ViewportCommand::Title(titulo.clone()));
            self.titulo = titulo;
        }
    }
}

fn mismo_dispositivo(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::symlink_metadata(a), std::fs::metadata(b)) {
        (Ok(x), Ok(y)) => x.dev() == y.dev(),
        _ => true,
    }
}

fn decodificar_uri(s: &str) -> String {
    let b = s.as_bytes();
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(a), Some(c)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push(a * 16 + c);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn buscar_tema<'a>(temas: &'a [Tema], id: &str) -> &'a Tema {
    temas
        .iter()
        .find(|t| t.id == id)
        .or_else(|| {
            let actual = nucleo::tema_id_actual(id);
            temas.iter().find(|t| t.id == actual)
        })
        .or_else(|| temas.first())
        .expect("siempre hay temas integrados")
}

/// Curva suave para animaciones (sale rápido y frena al final).
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

impl eframe::App for Solar {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        sonido::configurar(self.config.sonidos, self.config.volumen);
        self.procesar_mensajes(&ctx);
        self.revisar_actualizador();
        self.aplicar_apariencia(&ctx);
        self.actualizar_titulo(&ctx);
        self.actualizar_visibles();
        self.maximizada = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

        let mut acciones: Vec<Accion> = Vec::new();
        // Con un menú abierto, las teclas son del menú (Esc solo lo cierra)
        let libre = self.dialogo.is_none()
            && self.ajustes.as_ref().is_none_or(|a| a.capturando.is_none())
            && !ctx.egui_wants_keyboard_input()
            && !egui::Popup::is_any_open(&ctx);
        if libre {
            // Si llegó texto pegado del sistema, lo usamos (puede traer rutas
            // copiadas en otro gestor de archivos o en una terminal)
            let pegado = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Paste(t) => Some(t.clone()),
                    _ => None,
                })
            });
            for id in self.atajos.detectar(&ctx) {
                match (self.accion_de_atajo(&id), &pegado) {
                    (Some(Accion::Pegar(_)), Some(t)) => acciones.push(Accion::PegarTexto(t.clone())),
                    (Some(a), _) => acciones.push(a),
                    _ => {}
                }
            }
            if self.ajustes.is_some() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                // Esc cierra Ajustes (y no toca la selección de atrás)
                acciones.push(Accion::Ajustes);
            } else {
                self.teclado_vista(&ctx, &mut acciones);
            }
        } else if self.dialogo.is_none() && ctx.egui_wants_keyboard_input() {
            // Escape cierra la búsqueda o la edición de ruta
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                if self.editando_ruta.is_some() {
                    self.editando_ruta = None;
                } else if self.busqueda.is_some() {
                    self.busqueda = None;
                    self.visibles_sucios = true;
                }
            }
        }
        ctx.input(|i| {
            if i.pointer.button_pressed(egui::PointerButton::Extra1) {
                acciones.push(Accion::Atras);
            }
            if i.pointer.button_pressed(egui::PointerButton::Extra2) {
                acciones.push(Accion::Adelante);
            }
            let soltados: Vec<PathBuf> = i
                .raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect();
            if !soltados.is_empty() {
                acciones.push(Accion::TraerExternos(soltados));
            }
        });

        self.barra_superior(ui, &mut acciones);
        self.barra_estado(ui, &mut acciones);
        if self.config.barra_ubicacion == nucleo::Posicion::Abajo {
            self.panel_ubicacion_abajo(ui, &mut acciones);
        }
        self.barra_lateral(ui, &mut acciones);
        self.vista_central(ui, &mut acciones);

        self.dibujar_arrastre(&ctx);
        self.mostrar_notificaciones(&ctx);
        self.ventana_tareas(&ctx);
        self.ventana_ajustes(&ctx, &mut acciones);
        self.mostrar_dialogo(&ctx, &mut acciones);
        if self.config.barra_titulo_integrada && !self.maximizada {
            self.bordes_ventana(&ctx);
        }

        for a in acciones {
            self.ejecutar(&ctx, a);
        }
        self.guardar_si_hace_falta(&ctx);
        sonido::despachar();

        if !self.tareas.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        if self.cargando_desde.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(ed) = self.ajustes.as_mut().and_then(|a| a.editor.take()) {
            self.cancelar_editor(ed);
        }
        self.config.atajos = self.atajos.a_config();
        if self.config != self.config_guardada {
            let _ = self.config.guardar(&self.rutas.config);
        }
        for t in &self.tareas {
            t.cancelar.store(true, Ordering::Relaxed);
        }
    }
}
