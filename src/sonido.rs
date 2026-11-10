//! Efectos de sonido (los mismos de Capyro). Los MP3 van dentro del
//! programa; el núcleo en C los decodifica y los toca.
//!
//! Durante un cuadro se pueden pedir varios sonidos (un botón que además
//! dispara una acción); al final del cuadro solo suena el más importante,
//! para que no se encimen.

use crate::nucleo;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Sonido {
    /// Botones, menús, abrir cosas
    Clic = 0,
    /// Cerrar ventanas y paneles, mandar a la papelera
    Retraccion = 1,
    /// Algo cambió de aspecto: color, logotipo, tema
    Efecto = 2,
    /// Terminó una operación larga
    Listo = 3,
    /// Al abrir Solar
    Inicio = 4,
    /// Algo salió mal
    Error = 5,
}

const ARCHIVOS: [(Sonido, &[u8]); 6] = [
    (Sonido::Clic, include_bytes!("../recursos/sonidos/click.mp3")),
    (Sonido::Retraccion, include_bytes!("../recursos/sonidos/retraccion.mp3")),
    (Sonido::Efecto, include_bytes!("../recursos/sonidos/efecto.mp3")),
    (Sonido::Listo, include_bytes!("../recursos/sonidos/listo.mp3")),
    (Sonido::Inicio, include_bytes!("../recursos/sonidos/inicio.mp3")),
    (Sonido::Error, include_bytes!("../recursos/sonidos/error.mp3")),
];

static ACTIVOS: AtomicBool = AtomicBool::new(true);
static VOLUMEN: AtomicU32 = AtomicU32::new(0x3F19_999A); // 0.6
static CARGADOS: AtomicBool = AtomicBool::new(false);
/// Ya se mandaron a cargar (con los sonidos apagados no se decodifican:
/// ni CPU al abrir ni memoria, hasta que los prendas)
static INICIADO: AtomicBool = AtomicBool::new(false);
static PENDIENTE: Mutex<Option<Sonido>> = Mutex::new(None);
static ULTIMO: Mutex<Option<(Sonido, Instant)>> = Mutex::new(None);

/// Decodifica los sonidos (en un hilo aparte: tarda unos milisegundos).
pub fn iniciar(con_bienvenida: bool) {
    if !ACTIVOS.load(Ordering::Relaxed) || INICIADO.swap(true, Ordering::AcqRel) {
        return;
    }
    std::thread::Builder::new()
        .name("solar-sonidos".into())
        .spawn(move || {
            for (s, datos) in ARCHIVOS {
                if let Err(e) = nucleo::sonido_cargar(s as i32, datos) {
                    eprintln!("solar: no se pudo leer el sonido {s:?}: {}", e.mensaje);
                }
            }
            CARGADOS.store(true, Ordering::Release);
            if con_bienvenida {
                tocar_ya(Sonido::Inicio);
            }
        })
        .ok();
}

/// Solo guarda si están prendidos y el volumen (sin cargar nada).
pub fn ajustar(activos: bool, volumen: f32) {
    ACTIVOS.store(activos, Ordering::Relaxed);
    VOLUMEN.store(volumen.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}

pub fn configurar(activos: bool, volumen: f32) {
    ajustar(activos, volumen);
    if activos && !INICIADO.load(Ordering::Relaxed) {
        // Se prendieron desde Ajustes: cargarlos ahora (sin bienvenida)
        iniciar(false);
    }
}

fn volumen() -> f32 {
    f32::from_bits(VOLUMEN.load(Ordering::Relaxed))
}

/// Pide un sonido para este cuadro (suena el más importante al final).
pub fn pedir(s: Sonido) {
    if !ACTIVOS.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(mut p) = PENDIENTE.lock() {
        *p = Some(p.map_or(s, |a| a.max(s)));
    }
}

/// Toca lo pedido durante el cuadro. Se llama una vez al final de cada cuadro.
pub fn despachar() {
    let s = PENDIENTE.lock().ok().and_then(|mut p| p.take());
    if let Some(s) = s {
        tocar_ya(s);
    }
}

/// Toca ahora mismo (respeta el interruptor y el volumen).
pub fn tocar_ya(s: Sonido) {
    if !ACTIVOS.load(Ordering::Relaxed) || !CARGADOS.load(Ordering::Acquire) {
        return;
    }
    // El mismo sonido dos veces casi juntas (doble clic) suena una sola vez
    if let Ok(mut u) = ULTIMO.lock() {
        if let Some((anterior, cuando)) = *u
            && anterior == s
            && cuando.elapsed() < Duration::from_millis(70)
        {
            return;
        }
        *u = Some((s, Instant::now()));
    }
    // Los clics van un poco más suaves que los avisos
    let factor = match s {
        Sonido::Clic | Sonido::Retraccion => 0.75,
        _ => 1.0,
    };
    nucleo::sonido_reproducir(s as i32, volumen() * factor);
}

/// Para el botón «Probar» de Ajustes.
pub fn probar() {
    let antes = ACTIVOS.swap(true, Ordering::Relaxed);
    tocar_ya(Sonido::Listo);
    ACTIVOS.store(antes, Ordering::Relaxed);
}
