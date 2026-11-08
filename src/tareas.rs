//! Trabajo en segundo plano: listar carpetas, copiar/mover/borrar, vigilar
//! cambios y memorias USB, y calcular tamaños. Nada de esto congela la
//! ventana: los hilos mandan mensajes a la interfaz.

use crate::nucleo::{self, Carpetas, Conflicto, Dispositivo, ElementoPapelera, Entrada, Espacio, Orden, Progreso};
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub enum Mensaje {
    Listado {
        generacion: u64,
        resultado: Result<Vec<Entrada>, nucleo::ErrorSolar>,
        espacio: Option<Espacio>,
    },
    Papelera {
        generacion: u64,
        elementos: Vec<ElementoPapelera>,
    },
    CambioCarpeta,
    CambioPapelera,
    Dispositivos(Vec<Dispositivo>),
    TareaTerminada {
        id: u64,
        resultado: Result<String, nucleo::ErrorSolar>,
        destinos: Vec<PathBuf>,
        tipo: TipoTarea,
    },
    Montaje {
        dispositivo: String,
        resultado: Result<(), nucleo::ErrorSolar>,
        abrir: bool,
        desmontar: bool,
    },
}

// ----------------------------------------------------------------------
//  Listados
// ----------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn listar(
    dir: PathBuf,
    orden: Orden,
    descendente: bool,
    carpetas_primero: bool,
    generacion: u64,
    carpetas: Arc<Mutex<Carpetas>>,
    tx: Sender<Mensaje>,
    ctx: egui::Context,
) {
    thread::Builder::new()
        .name("solar-listar".into())
        .spawn(move || {
            let resultado = nucleo::listar(&dir, orden, descendente, carpetas_primero).map(|mut v| {
                if let Ok(db) = carpetas.lock()
                    && db.cantidad() > 0
                {
                    for e in v.iter_mut().filter(|e| e.es_carpeta()) {
                        e.estilo = db.obtener(&e.ruta);
                    }
                }
                v
            });
            let espacio = nucleo::espacio(&dir);
            let _ = tx.send(Mensaje::Listado { generacion, resultado, espacio });
            ctx.request_repaint();
        })
        .ok();
}

pub fn listar_papelera(generacion: u64, tx: Sender<Mensaje>, ctx: egui::Context) {
    thread::Builder::new()
        .name("solar-papelera".into())
        .spawn(move || {
            let elementos = nucleo::papelera_listar();
            let _ = tx.send(Mensaje::Papelera { generacion, elementos });
            ctx.request_repaint();
        })
        .ok();
}

// ----------------------------------------------------------------------
//  Vigía: cambios en la carpeta, la papelera y los dispositivos
// ----------------------------------------------------------------------

pub enum ComandoVigia {
    Observar(Option<PathBuf>),
    Dispositivos { incluir_internos: bool },
}

pub fn iniciar_vigia(tx: Sender<Mensaje>, ctx: egui::Context, incluir_internos: bool) -> Sender<ComandoVigia> {
    let (tx_cmd, rx_cmd) = std::sync::mpsc::channel();
    thread::Builder::new()
        .name("solar-vigia".into())
        .spawn(move || vigia(rx_cmd, tx, ctx, incluir_internos))
        .ok();
    tx_cmd
}

fn vigia(rx: Receiver<ComandoVigia>, tx: Sender<Mensaje>, ctx: egui::Context, mut incluir_internos: bool) {
    let mut vigilante = nucleo::Vigilante::nuevo();
    let mut previos: Option<Vec<Dispositivo>> = None;
    let mut ultimo = Instant::now() - Duration::from_secs(60);
    loop {
        let mut forzar = false;
        loop {
            match rx.try_recv() {
                Ok(ComandoVigia::Observar(ruta)) => {
                    if let Some(v) = vigilante.as_mut() {
                        v.observar(ruta.as_deref());
                    }
                }
                Ok(ComandoVigia::Dispositivos { incluir_internos: i }) => {
                    incluir_internos = i;
                    forzar = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        let mut cambios = match vigilante.as_mut() {
            Some(v) => v.esperar(250),
            None => {
                thread::sleep(Duration::from_millis(250));
                0
            }
        };
        if cambios > 0 && cambios & nucleo::CAMBIO_CARPETA != 0 {
            // Juntamos ráfagas (p. ej. al copiar muchos archivos)
            if let Some(v) = vigilante.as_mut() {
                let fin = Instant::now() + Duration::from_millis(150);
                while Instant::now() < fin {
                    let c = v.esperar(50);
                    if c > 0 {
                        cambios |= c;
                    }
                }
            }
        }
        let mut avisar = false;
        if cambios > 0 && cambios & nucleo::CAMBIO_CARPETA != 0 {
            avisar |= tx.send(Mensaje::CambioCarpeta).is_ok();
        }
        if cambios > 0 && cambios & nucleo::CAMBIO_PAPELERA != 0 {
            avisar |= tx.send(Mensaje::CambioPapelera).is_ok();
        }
        let montajes = cambios > 0 && cambios & nucleo::CAMBIO_MONTAJES != 0;
        if forzar || montajes || ultimo.elapsed() > Duration::from_secs(2) {
            ultimo = Instant::now();
            let d = nucleo::dispositivos(incluir_internos);
            if previos.as_ref() != Some(&d) {
                previos = Some(d.clone());
                if tx.send(Mensaje::Dispositivos(d)).is_err() {
                    return;
                }
                avisar = true;
            }
        }
        if avisar {
            ctx.request_repaint();
        }
    }
}

// ----------------------------------------------------------------------
//  Operaciones con progreso
// ----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum TipoTarea {
    Copiar { origenes: Vec<PathBuf>, destino: PathBuf, conflicto: Conflicto },
    Mover { origenes: Vec<PathBuf>, destino: PathBuf, conflicto: Conflicto },
    Papelera { rutas: Vec<PathBuf> },
    Eliminar { rutas: Vec<PathBuf> },
    Restaurar { rutas: Vec<PathBuf> },
    BorrarDePapelera { rutas: Vec<PathBuf> },
    VaciarPapelera,
}

impl TipoTarea {
    pub fn titulo(&self) -> String {
        let n = |v: &Vec<PathBuf>| {
            if v.len() == 1 {
                format!("«{}»", nombre(&v[0]))
            } else {
                crate::formato::elementos(v.len())
            }
        };
        match self {
            Self::Copiar { origenes, destino, .. } => format!("Copiando {} a «{}»", n(origenes), nombre(destino)),
            Self::Mover { origenes, destino, .. } => format!("Moviendo {} a «{}»", n(origenes), nombre(destino)),
            Self::Papelera { rutas } => format!("Moviendo {} a la papelera", n(rutas)),
            Self::Eliminar { rutas } => format!("Eliminando {}", n(rutas)),
            Self::Restaurar { rutas } => format!("Restaurando {}", n(rutas)),
            Self::BorrarDePapelera { rutas } => format!("Eliminando {} de la papelera", n(rutas)),
            Self::VaciarPapelera => "Vaciando la papelera".into(),
        }
    }
}

pub fn nombre(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string_lossy().into_owned())
}

#[derive(Default, Clone)]
pub struct EstadoTarea {
    pub progreso: Progreso,
}

pub struct Tarea {
    pub id: u64,
    pub tipo: TipoTarea,
    pub titulo: String,
    pub estado: Arc<Mutex<EstadoTarea>>,
    pub cancelar: Arc<AtomicBool>,
    pub inicio: Instant,
}

impl Tarea {
    pub fn fraccion(&self) -> Option<f32> {
        let p = self.estado.lock().ok()?.progreso.clone();
        if p.bytes_total > 0 {
            Some((p.bytes_hechos as f64 / p.bytes_total as f64) as f32)
        } else if p.total > 0 {
            Some((p.hechos as f64 / p.total as f64) as f32)
        } else {
            None
        }
    }
}

pub fn lanzar_tarea(
    id: u64,
    tipo: TipoTarea,
    carpetas: Arc<Mutex<Carpetas>>,
    tx: Sender<Mensaje>,
    ctx: egui::Context,
) -> Tarea {
    let estado = Arc::new(Mutex::new(EstadoTarea::default()));
    let cancelar = Arc::new(AtomicBool::new(false));
    let tarea = Tarea {
        id,
        titulo: tipo.titulo(),
        tipo: tipo.clone(),
        estado: estado.clone(),
        cancelar: cancelar.clone(),
        inicio: Instant::now(),
    };
    thread::Builder::new()
        .name("solar-tarea".into())
        .spawn(move || {
            let mut ultimo_repintado = Instant::now();
            let mut progreso = |p: &Progreso| {
                if let Ok(mut e) = estado.lock() {
                    e.progreso = p.clone();
                }
                if ultimo_repintado.elapsed() > Duration::from_millis(100) {
                    ultimo_repintado = Instant::now();
                    ctx.request_repaint();
                }
                !cancelar.load(Ordering::Relaxed)
            };
            let mut destinos: Vec<PathBuf> = Vec::new();
            let resultado = ejecutar(&tipo, &mut progreso, &mut destinos, &carpetas);
            let _ = tx.send(Mensaje::TareaTerminada { id, resultado, destinos, tipo });
            ctx.request_repaint();
        })
        .ok();
    tarea
}

fn plural(n: usize, uno: &str, varios: &str) -> String {
    if n == 1 { format!("1 {uno}") } else { format!("{} {varios}", crate::formato::miles(n as u64)) }
}

fn ejecutar(
    tipo: &TipoTarea,
    progreso: &mut dyn FnMut(&Progreso) -> bool,
    destinos: &mut Vec<PathBuf>,
    carpetas: &Arc<Mutex<Carpetas>>,
) -> Result<String, nucleo::ErrorSolar> {
    match tipo {
        TipoTarea::Copiar { origenes, destino, conflicto } => {
            let mut al_terminar = |o: &Path, d: &Path| {
                destinos.push(d.to_path_buf());
                // El estilo de la carpeta viaja con la copia
                if let Ok(mut db) = carpetas.lock() {
                    db.copiar(o, d);
                }
            };
            nucleo::copiar(origenes, destino, *conflicto, progreso, &mut al_terminar)?;
            Ok(format!("Se copió {} a «{}»", plural(origenes.len(), "elemento", "elementos"), nombre(destino)))
        }
        TipoTarea::Mover { origenes, destino, conflicto } => {
            let mut al_terminar = |o: &Path, d: &Path| {
                destinos.push(d.to_path_buf());
                if o != d
                    && let Ok(mut db) = carpetas.lock()
                {
                    db.renombrar(o, d);
                }
            };
            nucleo::mover(origenes, destino, *conflicto, progreso, &mut al_terminar)?;
            Ok(format!("Se movió {} a «{}»", plural(origenes.len(), "elemento", "elementos"), nombre(destino)))
        }
        TipoTarea::Papelera { rutas } => {
            let mut nada = |_: &Path, _: &Path| {};
            nucleo::papelera_enviar(rutas, progreso, &mut nada)?;
            Ok(format!("{} a la papelera", plural(rutas.len(), "elemento movido", "elementos movidos")))
        }
        TipoTarea::Eliminar { rutas } => {
            let mut al_terminar = |o: &Path, _: &Path| {
                if let Ok(mut db) = carpetas.lock() {
                    db.olvidar_arbol(o);
                }
            };
            nucleo::eliminar(rutas, progreso, &mut al_terminar)?;
            Ok(format!("Se eliminó {} para siempre", plural(rutas.len(), "elemento", "elementos")))
        }
        TipoTarea::Restaurar { rutas } => {
            for (i, r) in rutas.iter().enumerate() {
                let p = Progreso { hechos: i as u64, total: rutas.len() as u64, actual: nombre(r), ..Default::default() };
                if !progreso(&p) {
                    return Err(nucleo::ErrorSolar::de_codigo(125));
                }
                destinos.push(nucleo::papelera_restaurar(r)?);
            }
            Ok(format!("Se restauró {}", plural(rutas.len(), "elemento", "elementos")))
        }
        TipoTarea::BorrarDePapelera { rutas } => {
            for (i, r) in rutas.iter().enumerate() {
                let p = Progreso { hechos: i as u64, total: rutas.len() as u64, actual: nombre(r), ..Default::default() };
                if !progreso(&p) {
                    return Err(nucleo::ErrorSolar::de_codigo(125));
                }
                nucleo::papelera_borrar(r)?;
            }
            Ok(format!("Se eliminó {} para siempre", plural(rutas.len(), "elemento", "elementos")))
        }
        TipoTarea::VaciarPapelera => {
            nucleo::papelera_vaciar(progreso)?;
            Ok("La papelera quedó vacía".into())
        }
    }
}

// ----------------------------------------------------------------------
//  Montar / expulsar y tamaño de carpetas
// ----------------------------------------------------------------------

pub fn montaje(dispositivo: String, desmontar: bool, expulsar: bool, abrir: bool, tx: Sender<Mensaje>, ctx: egui::Context) {
    thread::Builder::new()
        .name("solar-montaje".into())
        .spawn(move || {
            let resultado =
                if desmontar { nucleo::desmontar(&dispositivo, expulsar) } else { nucleo::montar(&dispositivo) };
            let _ = tx.send(Mensaje::Montaje { dispositivo, resultado, abrir, desmontar });
            ctx.request_repaint();
        })
        .ok();
}

#[derive(Default, Clone, Copy)]
pub struct Conteo {
    pub bytes: u64,
    pub archivos: u64,
    pub carpetas: u64,
    pub terminado: bool,
}

pub fn calcular_tamano(rutas: Vec<PathBuf>, salida: Arc<Mutex<Conteo>>, cancelar: Arc<AtomicBool>, ctx: egui::Context) {
    thread::Builder::new()
        .name("solar-tamano".into())
        .spawn(move || {
            let mut total = Conteo::default();
            let mut ultimo = Instant::now();
            for r in &rutas {
                let base = total;
                let res = nucleo::tamano_total(r, |bytes, _| {
                    if ultimo.elapsed() > Duration::from_millis(120) {
                        ultimo = Instant::now();
                        if let Ok(mut s) = salida.lock() {
                            s.bytes = base.bytes + bytes;
                        }
                        ctx.request_repaint();
                    }
                    !cancelar.load(Ordering::Relaxed)
                });
                match res {
                    Ok((b, a, c)) => {
                        let es_dir = r.is_dir();
                        total.bytes += b;
                        total.archivos += a;
                        total.carpetas += c + es_dir as u64;
                    }
                    Err(_) if cancelar.load(Ordering::Relaxed) => return,
                    Err(_) => {}
                }
            }
            total.terminado = true;
            if let Ok(mut s) = salida.lock() {
                *s = total;
            }
            ctx.request_repaint();
        })
        .ok();
}
