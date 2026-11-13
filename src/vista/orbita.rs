//! Órbita: la carpeta abierta va en el centro (como el sol) y lo que tiene
//! adentro le da vueltas en anillos, todo con su ícono de siempre (la carpeta
//! con su color y logotipo, la hoja del color de su tipo). Los archivos más pesados se ven
//! un poco más grandes. Se puede elegir el orden (del sol hacia afuera), si
//! giran y qué tan rápido, y si las órbitas se ven con electricidad.
//!
//! Con la rueda se acerca y se aleja; con el botón de la rueda se mueve.
//! Al pasar el mouse por un archivo, todo se detiene para poder atinarle.
//! (Por dentro se les sigue diciendo «planetas».)

use super::{Disposicion, Visual, texto_ajustado};
use crate::app::{Accion, Solar, Ubicacion};
use crate::iconos::{self, Logotipo};
use crate::nucleo::{Entrada, EstiloCarpeta, OrdenOrbita, TipoEntrada};
use crate::tema::{Paleta, con_alfa};
use crate::ui_util::{boton_icono, boton_icono_estado};
use crate::formato;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Key, Margin, Painter, Pos2, Rect, RichText, Sense,
    Shape, Stroke, Ui, Vec2, pos2, vec2,
};
use egui_phosphor::regular as ph;
use std::f32::consts::TAU;
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

/// Una vuelta del anillo de adentro tarda esto a velocidad 1.
const SEGUNDOS_POR_VUELTA: f32 = 48.0;
/// Las órbitas pueden ser elipses para aprovechar pantallas anchas.
const ELIPSE_MAX: f32 = 1.45;
/// Lo alto del nombre del sol (dos renglones con su fondito).
const ALTO_NOMBRE_SOL: f32 = 40.0;
/// Dónde termina el ícono de la carpeta del centro, hacia abajo (en radios).
const BAJO_RAIZ: f32 = 0.82;
/// El sol nunca se ve más chico que esto (en carpetas enormes todo se
/// encoge, pero el sol tiene que seguir viéndose).
const SOL_MINIMO: f32 = 16.0;

#[derive(Clone, Debug)]
pub(crate) enum CambioOrbita {
    Girar(bool),
    Velocidad(f32),
    Orden(OrdenOrbita),
    Electricidad(bool),
    Revolver,
    Centrar,
}

#[derive(Clone, PartialEq)]
struct ClaveOrden {
    version: u64,
    orden: OrdenOrbita,
    semilla: u64,
    carpetas_primero: bool,
}

/// Lo que la órbita recuerda entre cuadros.
pub(crate) struct EstadoOrbita {
    zoom: f32,
    pan: Vec2,
    /// Segundos de giro acumulados (ya multiplicados por la velocidad)
    fase: f32,
    /// Reloj para la electricidad (avanza aunque no giren)
    reloj: f32,
    ultimo: Option<Instant>,
    /// 0 = gira, 1 = quieto (se suaviza)
    pausa: f32,
    semilla: u64,
    carpeta: Option<Ubicacion>,
    orden: Option<(ClaveOrden, Arc<Vec<u32>>)>,
    /// Dónde estaba cada planeta (para que se deslicen al reordenar)
    ultimas: Vec<Vec2>,
    transicion: Option<(Instant, Vec<Vec2>)>,
    arrastrando: bool,
}

impl Default for EstadoOrbita {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            fase: 0.0,
            reloj: 0.0,
            ultimo: None,
            pausa: 0.0,
            semilla: 1,
            carpeta: None,
            orden: None,
            ultimas: Vec::new(),
            transicion: None,
            arrastrando: false,
        }
    }
}

struct Anillo {
    /// Radio en el mundo (sin escala)
    radio: f32,
    desde: usize,
    cuantos: usize,
}

#[derive(Clone, Copy)]
struct Planeta {
    /// Respecto al sol, en pantalla
    centro: Vec2,
    radio: f32,
    anillo: u16,
    angulo: f32,
    /// Lo ancho que puede ser su nombre sin chocar con los vecinos
    etiqueta: f32,
}

/// La órbita lista para dibujar en este cuadro.
pub(crate) struct Orbita {
    n: usize,
    /// Centro del sol en coordenadas del contenido
    centro: Pos2,
    /// Esquina del área en pantalla
    origen: Pos2,
    area: Vec2,
    ex: f32,
    ey: f32,
    /// El acercamiento (el nombre del sol se encoge al alejarse)
    zoom: f32,
    /// Radio del sol en pantalla
    radio_sol: f32,
    anillos: Vec<Anillo>,
    planetas: Vec<Planeta>,
    /// posición en el acomodo -> índice visible
    por_lugar: Arc<Vec<u32>>,
    /// índice visible -> posición en el acomodo
    lugar_de: Vec<u32>,
    etiquetas: bool,
    ancho_etiqueta: f32,
    electricidad: bool,
    reloj: f32,
    /// La carpeta del centro (como el sol): su color y logotipo
    raiz: EstiloCarpeta,
    papelera: bool,
    nombre_sol: String,
    detalle_sol: String,
}

/// Número pseudoaleatorio estable en [0, 1).
fn azar(a: u32, b: u32, c: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77) ^ c.wrapping_mul(0xC2B2_AE3D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

fn hash_texto(t: &str) -> u64 {
    t.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3))
}

impl Solar {
    /// El acomodo de los planetas (del sol hacia afuera), guardado mientras
    /// no cambien los archivos ni el orden.
    fn acomodo(&mut self) -> Arc<Vec<u32>> {
        let av = *self.av();
        let clave = ClaveOrden {
            version: self.version_visibles,
            orden: av.orbita_orden,
            semilla: self.orbita.semilla,
            carpetas_primero: av.carpetas_primero,
        };
        if let Some((c, v)) = &self.orbita.orden
            && *c == clave
        {
            return v.clone();
        }
        // Mismos archivos, otro orden: que se deslicen a su lugar nuevo
        if let Some((c, _)) = &self.orbita.orden
            && c.version == clave.version
            && self.orbita.ultimas.len() == self.visibles.len()
            && self.config.animaciones
        {
            self.orbita.transicion = Some((Instant::now(), self.orbita.ultimas.clone()));
        }
        let n = self.visibles.len();
        let mut v: Vec<u32> = (0..n as u32).collect();
        let entradas = &self.entradas;
        let visibles = &self.visibles;
        let e = |k: u32| &entradas[visibles[k as usize]];
        match av.orbita_orden {
            OrdenOrbita::GrandePrimero => v.sort_by_key(|&k| std::cmp::Reverse(peso(e(k)))),
            OrdenOrbita::ChicoPrimero => v.sort_by_key(|&k| peso(e(k))),
            OrdenOrbita::Nombre => v.sort_by(|&a, &b| e(a).plegado().cmp(e(b).plegado())),
            OrdenOrbita::Tipo => v.sort_by(|&a, &b| {
                (e(a).categoria as u8, e(a).plegado()).cmp(&(e(b).categoria as u8, e(b).plegado()))
            }),
            OrdenOrbita::Fecha => v.sort_by_key(|&k| std::cmp::Reverse(e(k).modificado)),
            OrdenOrbita::Aleatorio => {
                let lugar = self.ubicacion.ruta().map(|r| hash_texto(&r.to_string_lossy())).unwrap_or(3);
                let mut s = (self.orbita.semilla ^ lugar).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
                for i in (1..v.len()).rev() {
                    s ^= s >> 12;
                    s ^= s << 25;
                    s ^= s >> 27;
                    let j = (s.wrapping_mul(0x2545F4914F6CDD1D) % (i as u64 + 1)) as usize;
                    v.swap(i, j);
                }
            }
        }
        if av.carpetas_primero && av.orbita_orden != OrdenOrbita::Aleatorio {
            // Las carpetas, más cerca del sol (sin perder el orden de cada grupo)
            v.sort_by_key(|&k| !e(k).es_carpeta());
        }
        let v = Arc::new(v);
        self.orbita.orden = Some((clave, v.clone()));
        v
    }

    /// Prepara la órbita de este cuadro: lee la rueda y el arrastre, hace
    /// girar a los planetas y calcula dónde queda cada uno.
    pub(crate) fn preparar_orbita(&mut self, ui: &mut Ui, area: Rect, _acciones: &mut Vec<Accion>) -> Orbita {
        let ctx = ui.ctx().clone();
        let av = *self.av();
        let animar = self.config.animaciones;
        // Carpeta nueva: vista centrada otra vez
        if self.orbita.carpeta.as_ref() != Some(&self.ubicacion) {
            self.orbita.carpeta = Some(self.ubicacion.clone());
            self.orbita.zoom = 1.0;
            self.orbita.pan = Vec2::ZERO;
            self.orbita.transicion = None;
        }
        let ahora = Instant::now();
        let dt = self.orbita.ultimo.map(|t| (ahora - t).as_secs_f32()).unwrap_or(0.0).min(0.1);
        self.orbita.ultimo = Some(ahora);

        let acomodo = self.acomodo();
        let n = self.visibles.len();

        // --- Geometría (en unidades del mundo) ---
        let maximo = self
            .visibles
            .iter()
            .map(|&i| &self.entradas[i])
            .filter(|e| e.tipo == TipoEntrada::Archivo)
            .map(|e| e.tamano)
            .max()
            .unwrap_or(0);
        let log_max = ((maximo as f32) + 1.0).ln().max(1.0);
        let d = (av.tamano_iconos * 0.7).clamp(26.0, 120.0);
        let sol = (d * 0.95).clamp(34.0, 90.0);
        let separacion = d + 34.0;
        let paso = d + 30.0;
        let disponer = |primero: f32| -> (Vec<Anillo>, f32) {
            let mut anillos: Vec<Anillo> = Vec::new();
            let mut puestos = 0;
            while puestos < n {
                let radio = primero + anillos.len() as f32 * separacion;
                let cabe = ((TAU * radio / paso).floor() as usize).max(6);
                let cuantos = cabe.min(n - puestos);
                anillos.push(Anillo { radio, desde: puestos, cuantos });
                puestos += cuantos;
            }
            let extension = anillos.last().map(|a| a.radio).unwrap_or(sol) + d / 2.0 + 26.0;
            (anillos, extension)
        };
        // Escala para que quepa todo, con órbitas un poco ovaladas si sobra ancho
        let medio = area.size() / 2.0 - vec2(16.0, 24.0);
        let escala = |extension: f32| -> (f32, f32) {
            let ajuste_y = (medio.y.max(60.0) / extension).clamp(0.12, 1.25);
            let ajuste_x = (medio.x.max(60.0) / extension).clamp(0.12, 1.25 * ELIPSE_MAX);
            let ey0 = ajuste_y.min(ajuste_x);
            (ajuste_x.min(ey0 * ELIPSE_MAX), ey0)
        };
        // El primer anillo deja lugar abajo del sol para su nombre (que no se
        // encoge con la escala), así los planetas no le pasan por encima
        let mut primero = sol + separacion * 0.8 + 8.0;
        let (mut anillos, mut extension) = disponer(primero);
        for _ in 0..3 {
            let (_, ey0) = escala(extension);
            let hace_falta = sol.max(SOL_MINIMO / ey0) * BAJO_RAIZ + d / 2.0 + (ALTO_NOMBRE_SOL + 14.0) / ey0;
            if hace_falta <= primero + 0.5 {
                break;
            }
            primero = hace_falta + 2.0;
            (anillos, extension) = disponer(primero);
        }
        let (ex0, ey0) = escala(extension);

        // --- Rueda: acercar y alejar (hacia donde está el puntero) ---
        let puntero = ctx.pointer_hover_pos().filter(|p| area.contains(*p));
        let centro_area = area.center();
        if let Some(p) = puntero
            && self.dialogo.is_none()
            && !egui::Popup::is_any_open(&ctx)
        {
            let rueda = ctx.input(|i| i.smooth_scroll_delta.y);
            if rueda.abs() > 0.1 {
                let antes = self.orbita.zoom;
                let despues = (antes * (rueda * 0.0018).exp()).clamp(0.4, 12.0);
                let rel = p - centro_area - self.orbita.pan;
                self.orbita.pan = p - centro_area - rel * (despues / antes);
                self.orbita.zoom = despues;
            }
        }
        // Botón de la rueda: mover la vista
        let (medio_abajo, delta) = ctx.input(|i| (i.pointer.middle_down(), i.pointer.delta()));
        if medio_abajo && (self.orbita.arrastrando || puntero.is_some()) {
            self.orbita.arrastrando = true;
            self.orbita.pan += delta;
            ctx.set_cursor_icon(CursorIcon::Grabbing);
        } else {
            self.orbita.arrastrando = false;
        }

        let (ex, ey) = (ex0 * self.orbita.zoom, ey0 * self.orbita.zoom);
        let radio_sol = (sol * ey).max(SOL_MINIMO);
        let centro = (centro_area - area.min).to_pos2() + self.orbita.pan;

        // --- Giro (se detiene con el mouse encima o mientras arrastras) ---
        self.orbita.reloj += if animar { dt } else { 0.0 };
        let velocidad_sol = TAU / SEGUNDOS_POR_VUELTA;
        let mut planetas = Vec::with_capacity(n);
        let mut lugar_de = vec![0u32; n];
        let calcular = |fase: f32, planetas: &mut Vec<Planeta>, lugar_de: &mut Vec<u32>| {
            planetas.clear();
            planetas.resize(n, Planeta { centro: Vec2::ZERO, radio: 0.0, anillo: 0, angulo: 0.0, etiqueta: 0.0 });
            for (ia, a) in anillos.iter().enumerate() {
                let omega = velocidad_sol * (primero / a.radio).powf(1.5);
                let hueco = TAU * a.radio * ex.min(ey) / a.cuantos.max(1) as f32;
                let etiqueta = (hueco * 0.92).clamp(56.0, 170.0);
                let inicio = ia as f32 * 2.399_963; // ángulo dorado: los anillos no se alinean
                for j in 0..a.cuantos {
                    let lugar = a.desde + j;
                    let k = acomodo[lugar] as usize;
                    let ang = inicio + TAU * j as f32 / a.cuantos as f32 - omega * fase;
                    let e = &self.entradas[self.visibles[k]];
                    // Los archivos más pesados se ven un poco más grandes
                    let factor = match e.tipo {
                        TipoEntrada::Archivo => 0.72 + 0.28 * ((e.tamano as f32 + 1.0).ln() / log_max).clamp(0.0, 1.0),
                        _ => 0.95,
                    };
                    planetas[k] = Planeta {
                        centro: vec2(ang.cos() * a.radio * ex, ang.sin() * a.radio * ey),
                        radio: (d / 2.0 * factor * ey).max(1.5),
                        anillo: ia as u16,
                        angulo: ang,
                        etiqueta,
                    };
                    lugar_de[k] = lugar as u32;
                }
            }
        };
        calcular(self.orbita.fase, &mut planetas, &mut lugar_de);

        // ¿El puntero está sobre un planeta o el sol?
        let encima = puntero.is_some_and(|p| {
            let rel = p - (area.min + centro.to_vec2());
            rel.length() < radio_sol || planetas.iter().any(|pl| (pl.centro - rel).length() < pl.radio + 4.0)
        });
        let ocupado = encima
            || self.banda.is_some()
            || egui::DragAndDrop::has_any_payload(&ctx)
            || egui::Popup::is_any_open(&ctx)
            || self.dialogo.is_some()
            || ctx.input(|i| i.pointer.any_down());
        let objetivo = if ocupado { 1.0 } else { 0.0 };
        self.orbita.pausa += (objetivo - self.orbita.pausa) * (dt * 7.0).min(1.0);
        let gira = av.orbita_girar && animar && n > 0;
        if gira {
            let antes = self.orbita.fase;
            self.orbita.fase += dt * av.orbita_velocidad * (1.0 - self.orbita.pausa);
            if (self.orbita.fase - antes).abs() > 1e-6 {
                calcular(self.orbita.fase, &mut planetas, &mut lugar_de);
            }
        }

        // Al reordenar, los planetas se deslizan de donde estaban
        if let Some((desde, viejas)) = &self.orbita.transicion {
            let t = crate::app::ease(desde.elapsed().as_secs_f32() / 0.55);
            if t >= 1.0 || viejas.len() != n {
                self.orbita.transicion = None;
            } else {
                for (pl, v) in planetas.iter_mut().zip(viejas) {
                    let antes = vec2(v.x * ex, v.y * ey);
                    pl.centro = antes + (pl.centro - antes) * t;
                }
                ctx.request_repaint();
            }
        }
        self.orbita.ultimas.clear();
        self.orbita.ultimas.extend(planetas.iter().map(|p| vec2(p.centro.x / ex.max(1e-3), p.centro.y / ey.max(1e-3))));

        // El foco del teclado se mueve fuera de la vista: la vista lo sigue
        let mut centro = centro;
        if self.desplazar_a_foco
            && let Some(k) = self.foco.clone().and_then(|f| self.indice_visible(&f))
            && let Some(pl) = planetas.get(k)
        {
            let pos = centro + pl.centro;
            let dentro = Rect::from_min_size(Pos2::ZERO, area.size()).shrink(70.0);
            let mut mover = Vec2::ZERO;
            if pos.x < dentro.left() {
                mover.x = dentro.left() - pos.x;
            } else if pos.x > dentro.right() {
                mover.x = dentro.right() - pos.x;
            }
            if pos.y < dentro.top() {
                mover.y = dentro.top() - pos.y;
            } else if pos.y > dentro.bottom() {
                mover.y = dentro.bottom() - pos.y;
            }
            self.orbita.pan += mover;
            centro += mover;
        }

        let electricidad = av.orbita_electricidad && n > 0;

        // Mientras algo se mueva, seguir dibujando (a unos 30 cuadros por
        // segundo basta y ahorra batería; con muchísimos planetas, a 20)
        let moviendose = gira && self.orbita.pausa < 0.999;
        if animar && (moviendose || electricidad || self.orbita.pausa > 0.001 && self.orbita.pausa < 0.999) {
            let ms = if n > 1500 { 50 } else { 33 };
            ctx.request_repaint_after(std::time::Duration::from_millis(ms));
        }

        // El centro: la carpeta abierta, con su color y logotipo (la carpeta
        // personal lleva la casita si no le pusiste otro)
        let (raiz, nombre_sol) = match &self.ubicacion {
            Ubicacion::Papelera => (EstiloCarpeta { color: Some(Color32::from_rgb(0x94, 0xA3, 0xB8)), ..Default::default() }, "Papelera".to_string()),
            Ubicacion::Ruta(r) => {
                let mut estilo = self.estilo_de(r).unwrap_or_default();
                let hogar = *r == crate::nucleo::dir_hogar();
                if hogar && estilo.icono.is_none() {
                    estilo.icono = Some("casa".to_string());
                }
                let nombre = if hogar { "Carpeta personal".to_string() } else { crate::tareas::nombre(r) };
                (estilo, nombre)
            }
        };
        let papelera = self.ubicacion == Ubicacion::Papelera;
        let etiquetas = d / 2.0 * ey >= 16.0;
        Orbita {
            n,
            centro,
            origen: area.min,
            area: area.size(),
            ex,
            ey,
            zoom: self.orbita.zoom,
            radio_sol,
            ancho_etiqueta: (paso * ey).clamp(50.0, 150.0),
            anillos,
            planetas,
            por_lugar: acomodo,
            lugar_de,
            etiquetas,
            electricidad,
            reloj: self.orbita.reloj,
            raiz,
            papelera,
            nombre_sol,
            detalle_sol: formato::elementos(n),
        }
    }

    /// Lo que se puede cambiar de la órbita sin ir a Ajustes.
    pub(crate) fn superposicion_orbita(&mut self, ui: &mut Ui, p: &Paleta, area: Rect, acciones: &mut Vec<Accion>) {
        let av = *self.av();
        let esquina = pos2(area.right() - 14.0, area.top() + 12.0);
        egui::Area::new(ui.id().with("barra_orbita"))
            .pivot(Align2::RIGHT_TOP)
            .fixed_pos(esquina)
            .show(ui.ctx(), |ui| self.barra_orbita(ui, p, &av, acciones));
    }

    fn barra_orbita(&mut self, ui: &mut Ui, p: &Paleta, av: &crate::nucleo::AjustesVista, acciones: &mut Vec<Accion>) {
        Frame::new()
            .fill(con_alfa(p.elevado, 0.92))
            .stroke(Stroke::new(1.0, con_alfa(p.borde, 0.8)))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::symmetric(6, 3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let r = boton_icono_estado(ui, p, ph::ARROWS_CLOCKWISE, "Que giren alrededor de la carpeta", true, av.orbita_girar);
                    if r.clicked() {
                        acciones.push(Accion::Orbita(CambioOrbita::Girar(!av.orbita_girar)));
                    }
                    if av.orbita_girar {
                        let mut v = av.orbita_velocidad;
                        ui.spacing_mut().slider_width = 70.0;
                        let s = ui
                            .add(egui::Slider::new(&mut v, 0.1..=4.0).logarithmic(true).show_value(false))
                            .on_hover_text("Velocidad del giro");
                        if s.changed() {
                            acciones.push(Accion::Orbita(CambioOrbita::Velocidad(v)));
                        }
                        ui.label(RichText::new(format!("×{v:.1}")).size(12.0).color(p.texto_tenue));
                    }
                    ui.add_space(4.0);
                    let orden = av.orbita_orden;
                    let menu = ui.menu_button(RichText::new(format!("{}  {}", ph::SORT_ASCENDING, corto(orden))).size(13.0), |ui| {
                        ui.set_min_width(230.0);
                        ui.label(RichText::new("Del centro hacia afuera").small().color(p.texto_tenue));
                        for o in OrdenOrbita::TODOS {
                            let g = if o == orden { ph::CHECK } else { " " };
                            if ui.button(format!("{g}   {}", o.nombre())).clicked() {
                                acciones.push(Accion::Orbita(CambioOrbita::Orden(o)));
                                ui.close();
                            }
                        }
                    });
                    menu.response.on_hover_text("Qué va más cerca del centro");
                    if orden == OrdenOrbita::Aleatorio && boton_icono(ui, p, ph::SHUFFLE, "Revolver", true).clicked() {
                        acciones.push(Accion::Orbita(CambioOrbita::Revolver));
                    }
                    let r = boton_icono_estado(ui, p, ph::LIGHTNING, "Órbitas con electricidad", true, av.orbita_electricidad);
                    if r.clicked() {
                        acciones.push(Accion::Orbita(CambioOrbita::Electricidad(!av.orbita_electricidad)));
                    }
                    let centrado = self.orbita.zoom == 1.0 && self.orbita.pan == Vec2::ZERO;
                    if boton_icono(ui, p, ph::CROSSHAIR_SIMPLE, "Centrar (la rueda acerca; el botón de la rueda mueve)", !centrado).clicked() {
                        acciones.push(Accion::Orbita(CambioOrbita::Centrar));
                    }
                });
            });
    }

    pub(crate) fn cambiar_orbita(&mut self, c: CambioOrbita) {
        match c {
            CambioOrbita::Centrar => {
                self.orbita.zoom = 1.0;
                self.orbita.pan = Vec2::ZERO;
                return;
            }
            CambioOrbita::Revolver => {
                self.orbita.semilla = self.orbita.semilla.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                return;
            }
            _ => {}
        }
        self.cambiar_av(|a| match c {
            CambioOrbita::Girar(g) => a.orbita_girar = g,
            CambioOrbita::Velocidad(v) => a.orbita_velocidad = v.clamp(0.1, 4.0),
            CambioOrbita::Orden(o) => a.orbita_orden = o,
            CambioOrbita::Electricidad(e) => a.orbita_electricidad = e,
            CambioOrbita::Revolver | CambioOrbita::Centrar => {}
        });
    }
}

fn corto(o: OrdenOrbita) -> &'static str {
    match o {
        OrdenOrbita::GrandePrimero => "Grandes",
        OrdenOrbita::ChicoPrimero => "Chicos",
        OrdenOrbita::Nombre => "Nombre",
        OrdenOrbita::Tipo => "Tipo",
        OrdenOrbita::Fecha => "Recientes",
        OrdenOrbita::Aleatorio => "Al azar",
    }
}

/// Cuánto «pesa» un archivo para ordenarlo por tamaño.
fn peso(e: &Entrada) -> u64 {
    if e.tipo == TipoEntrada::Archivo { e.tamano } else { 0 }
}

impl Orbita {
    fn electrico(&self, p: &Paleta) -> (Color32, Color32) {
        if p.oscuro {
            (Color32::from_rgb(0xE0, 0xF2, 0xFE), Color32::from_rgb(0x38, 0xBD, 0xF8))
        } else {
            (Color32::from_rgb(0x03, 0x69, 0xA1), Color32::from_rgb(0x38, 0xBD, 0xF8))
        }
    }
}

impl Disposicion for Orbita {
    fn desplazable(&self) -> bool {
        false
    }

    fn tamano(&self) -> Vec2 {
        self.area
    }

    fn rect(&self, k: usize) -> Rect {
        let p = self.planetas[k];
        Rect::from_center_size(self.centro + p.centro, Vec2::splat(p.radio * 2.0))
    }

    fn rango(&self, _visible: Rect) -> Range<usize> {
        0..self.n
    }

    fn zona(&self, rect: Rect) -> Rect {
        rect.expand(2.0)
    }

    fn caja_banda(&self, k: usize) -> Rect {
        let r = self.rect(k);
        r.shrink(r.width() * 0.3)
    }

    /// Al abrir la carpeta, los planetas salen disparados del sol.
    fn al_aparecer(&self, rect: Rect, t: f32) -> Rect {
        let sol = self.origen + self.centro.to_vec2();
        let c = sol + (rect.center() - sol) * t;
        Rect::from_center_size(c, rect.size() * (0.35 + 0.65 * t))
    }

    fn vecino(&self, actual: Option<usize>, tecla: Key) -> Option<usize> {
        if self.n == 0 {
            return None;
        }
        let Some(k) = actual else { return Some(self.por_lugar[0] as usize) };
        let lugar = self.lugar_de[k] as usize;
        let ir = |l: usize| Some(self.por_lugar[l.min(self.n - 1)] as usize);
        match tecla {
            Key::ArrowRight => ir((lugar + 1) % self.n),
            Key::ArrowLeft => ir((lugar + self.n - 1) % self.n),
            Key::Home => ir(0),
            Key::End => ir(self.n - 1),
            Key::ArrowDown | Key::ArrowUp | Key::PageDown | Key::PageUp => {
                let actual = self.planetas[k];
                let ia = actual.anillo as isize + if matches!(tecla, Key::ArrowDown | Key::PageDown) { 1 } else { -1 };
                let a = self.anillos.get(usize::try_from(ia).ok()?)?;
                // El del otro anillo con el ángulo más parecido
                let dif = |x: f32| {
                    let d = (x - actual.angulo).rem_euclid(TAU);
                    d.min(TAU - d)
                };
                let mejor = (a.desde..a.desde + a.cuantos)
                    .min_by(|&x, &y| {
                        let px = self.planetas[self.por_lugar[x] as usize].angulo;
                        let py = self.planetas[self.por_lugar[y] as usize].angulo;
                        dif(px).total_cmp(&dif(py))
                    })?;
                ir(mejor)
            }
            _ => None,
        }
    }

    fn pintar(&self, app: &Solar, painter: &Painter, e: &Entrada, rect: Rect, v: &Visual) {
        let p = &app.paleta;
        let c = rect.center();
        // Medio lado del ícono (crece con el mouse, se encoge al presionar)
        let r = rect.width() / 2.0 * (1.0 + 0.12 * v.hover - 0.06 * v.presion + 0.25 * v.salto);
        let base = match e.tipo {
            TipoEntrada::Carpeta => e.estilo.as_ref().and_then(|s| s.color).unwrap_or(p.carpeta),
            TipoEntrada::EnlaceRoto => Color32::from_rgb(0x6B, 0x72, 0x80),
            _ => iconos::color_categoria(e.categoria),
        };
        let alfa = v.alfa;
        if r < 5.0 {
            // Tan lejos que es polvo de estrellas (del color de su tipo)
            painter.circle_filled(c, r.max(1.2), con_alfa(base, 0.9 * alfa));
            return;
        }
        // El ícono ocupa un poco menos que su caja
        let borde = r * 1.08 + 4.0;
        // Con el color del texto para que se note sobre íconos de cualquier
        // color (una carpeta naranja seleccionada con el acento naranja no se veía)
        if v.soltar_aqui {
            painter.circle_filled(c, borde + 4.0, con_alfa(p.acento, 0.28));
            painter.circle_stroke(c, borde + 4.0, Stroke::new(2.5, con_alfa(p.texto, 0.9)));
        }
        if v.seleccionado {
            painter.circle_filled(c, borde + 2.0, con_alfa(p.acento, 0.24));
            // Una mira que gira despacio
            let giro = self.reloj * 0.6;
            for q in 0..4 {
                let a0 = giro + q as f32 * TAU / 4.0 + 0.2;
                let a1 = a0 + TAU / 4.0 - 0.4;
                let arco: Vec<Pos2> = (0..=10)
                    .map(|s| {
                        let a = a0 + (a1 - a0) * s as f32 / 10.0;
                        c + vec2(a.cos(), a.sin()) * borde
                    })
                    .collect();
                painter.add(Shape::line(arco, Stroke::new(2.0, con_alfa(p.texto, 0.9 * alfa.max(0.6)))));
            }
        }
        if v.hover > 0.0 {
            painter.circle_filled(c, r * 1.35, con_alfa(base, 0.16 * v.hover));
        }
        // El mismo ícono de las otras vistas: la carpeta con su color y
        // logotipo, o la hoja del color de su tipo (con su enlace si lo es)
        app.pintar_icono(painter, Rect::from_center_size(c, Vec2::splat(r * 2.0)), e, alfa);
        // El nombre abajo, con un fondito para leerlo sobre las órbitas
        if self.etiquetas || v.hover > 0.5 || v.seleccionado {
            let color = if alfa < 1.0 { con_alfa(p.texto, 0.75) } else { p.texto };
            let ancho = self.planetas.get(v.indice).map(|p| p.etiqueta).unwrap_or(self.ancho_etiqueta);
            let g = texto_ajustado(painter, &e.nombre, ancho, 2, 12.0, color, true);
            let y = c.y + r * 0.9 + 3.0;
            let caja = Rect::from_center_size(pos2(c.x, y + g.size().y / 2.0), g.rect.size() + vec2(10.0, 2.0));
            let fondo = if v.seleccionado { p.seleccion } else { con_alfa(p.fondo, 0.72) };
            painter.rect_filled(caja, CornerRadius::same(6), fondo);
            painter.galley(pos2(c.x - g.rect.center().x, caja.min.y + 1.0), g, color);
        }
    }

    /// Las órbitas y la carpeta del centro (detrás de lo que gira).
    fn decorar(&self, app: &Solar, ui: &mut Ui, area: Rect, acciones: &mut Vec<Accion>) {
        let p = &app.paleta;
        let painter = ui.painter_at(area);
        let c = self.origen + self.centro.to_vec2();
        let (nucleo, brillo) = self.electrico(p);

        // Órbitas (la electricidad, más delgada si quedan muy juntas)
        let grueso = match self.anillos.get(..2) {
            Some([a, b]) => ((b.radio - a.radio) * self.ey / 50.0).clamp(0.35, 1.0),
            _ => 1.0,
        };
        for (i, a) in self.anillos.iter().enumerate() {
            let (rx, ry) = (a.radio * self.ex, a.radio * self.ey);
            if self.electricidad {
                // Los quiebres del zigzag dependen del tamaño real del anillo
                // (no del acercamiento), así no se revuelven al hacer zoom. Si
                // en pantalla quedan muy juntos, se usa uno de cada 2, 4 u 8
                let en_mundo = (((TAU * a.radio / 8.0) / 8.0).round() as usize).max(6) * 8;
                let perimetro = TAU * ((rx * rx + ry * ry) / 2.0).sqrt();
                let mut salto = 1;
                while salto < 8 && perimetro / (en_mundo / salto) as f32 <= 5.0 {
                    salto *= 2;
                }
                let e = Electricidad {
                    t: self.reloj,
                    i: i as u32,
                    puntos: en_mundo / salto,
                    salto: salto as u32,
                    amplitud: (4.0 * self.ey).clamp(1.2, 10.0),
                    grueso,
                    nucleo,
                    brillo,
                };
                anillo_electrico(&painter, c, rx, ry, &e);
            } else {
                elipse(&painter, c, rx, ry, Stroke::new(1.0, con_alfa(p.texto_tenue, 0.22)));
            }
        }

        // La carpeta del centro, grande y con un resplandor suave de su color
        let rs = self.radio_sol;
        let color = self.raiz.color.unwrap_or(p.carpeta);
        for i in 0..4 {
            painter.circle_filled(c, rs * (1.0 + i as f32 * 0.22), con_alfa(color, 0.07 - i as f32 * 0.015));
        }
        let logo = if self.papelera { Logotipo::Glifo(ph::TRASH) } else { Logotipo::de(self.raiz.icono.as_deref()) };
        iconos::carpeta(&painter, Rect::from_center_size(c, Vec2::splat(rs * 2.0)), color, logo, self.raiz.color_icono, 1.0);
        // Nombre y cuántos elementos tiene (más chico si se aleja la vista)
        let z = self.zoom.clamp(0.5, 1.0);
        let ancho_max = (self.anillos.first().map(|a| a.radio * self.ex).unwrap_or(rs * 3.0) * 1.3).max(120.0);
        let g = texto_ajustado(&painter, &self.nombre_sol, ancho_max, 1, 14.5 * z, p.texto, false);
        let g2 = painter.layout_no_wrap(self.detalle_sol.clone(), FontId::proportional(12.0 * z), p.texto_tenue);
        let alto = g.size().y + g2.size().y;
        let ancho = g.size().x.max(g2.size().x) + 16.0 * z;
        let caja = Rect::from_center_size(pos2(c.x, c.y + rs * BAJO_RAIZ + 6.0 * z + alto / 2.0), vec2(ancho, alto + 6.0 * z));
        painter.rect_filled(caja, CornerRadius::same(8), con_alfa(p.fondo, 0.7));
        let (w1, w2, h1) = (g.size().x, g2.size().x, g.size().y);
        painter.galley(pos2(c.x - w1 / 2.0, caja.top() + 3.0 * z), g, p.texto);
        painter.galley(pos2(c.x - w2 / 2.0, caja.top() + 4.0 * z + h1), g2, p.texto_tenue);

        // Doble clic en el sol: subir una carpeta. Clic derecho: menú de la carpeta
        let resp = ui.interact(Rect::from_center_size(c, Vec2::splat(rs * 2.0)), ui.id().with("sol"), Sense::click());
        let resp = resp.on_hover_text(format!("{} · doble clic para subir", self.nombre_sol));
        if resp.double_clicked() {
            acciones.push(Accion::Subir);
        }
        resp.context_menu(|ui| app.menu_fondo(ui, acciones));
    }
}

fn elipse(painter: &Painter, c: Pos2, rx: f32, ry: f32, trazo: Stroke) {
    let n = ((rx.max(ry) * 0.45) as usize).clamp(48, 360);
    let puntos: Vec<Pos2> = (0..n).map(|i| {
        let a = i as f32 / n as f32 * TAU;
        c + vec2(a.cos() * rx, a.sin() * ry)
    })
    .collect();
    painter.add(Shape::closed_line(puntos, trazo));
}

/// Cómo se ve la electricidad de un anillo.
#[derive(Clone, Copy)]
struct Electricidad {
    t: f32,
    /// Número del anillo
    i: u32,
    /// Cuántos quiebres tiene el zigzag en toda la vuelta
    puntos: usize,
    /// Cada cuántos quiebres «del mundo» se toma uno
    salto: u32,
    /// Qué tanto se sale de la órbita (en pantalla)
    amplitud: f32,
    /// 1 = normal; menos si los anillos quedan muy juntos
    grueso: f32,
    nucleo: Color32,
    brillo: Color32,
}

/// Qué tanto se sale el punto `q` del anillo en el momento `t` (de -1 a 1).
/// Cada punto va de un valor al azar a otro sin brincos, y cada uno a su
/// propio ritmo: el rayo tiembla todo el tiempo, pero nunca parpadea.
fn temblor(q: u32, anillo: u32, t: f32) -> f32 {
    let s = t * 2.4 + azar(q, anillo, 7) * 4.0;
    let k = s.floor();
    let f = s - k;
    let f = f * f * (3.0 - 2.0 * f);
    let a = azar(q, anillo, k as u32) * 2.0 - 1.0;
    let b = azar(q, anillo, k as u32 + 1) * 2.0 - 1.0;
    a + (b - a) * f
}

/// Una órbita con electricidad: un rayo que la recorre completa y siempre
/// está encendido, con otro hilo más fino trenzado encima.
fn anillo_electrico(painter: &Painter, c: Pos2, rx: f32, ry: f32, e: &Electricidad) {
    let Electricidad { t, i, puntos, salto, amplitud, grueso, nucleo, brillo } = *e;
    // Un resplandor parejo debajo
    elipse(painter, c, rx, ry, Stroke::new(10.0 * grueso, con_alfa(brillo, 0.06)));
    let hilo = |otro: u32, fuerza: f32| -> Vec<Pos2> {
        (0..puntos)
            .map(|q| {
                let a = q as f32 / puntos as f32 * TAU;
                // Zonas que se sacuden un poco más, recorriendo el anillo despacio
                let oleada = (0.5 + 0.5 * (2.0 * a - t * 0.9 + i as f32 * 1.3).sin()).powi(3);
                let d = amplitud * fuerza * (0.75 + 0.5 * oleada) * temblor(q as u32 * salto, i.wrapping_mul(2).wrapping_add(otro), t);
                let dir = vec2(a.cos(), a.sin());
                // Hacia afuera de la elipse
                let normal = vec2(dir.x * ry, dir.y * rx).normalized();
                c + vec2(dir.x * rx, dir.y * ry) + normal * d
            })
            .collect()
    };
    let principal = hilo(0, 1.0);
    painter.add(Shape::closed_line(principal.clone(), Stroke::new(3.4 * grueso.max(0.5), con_alfa(brillo, 0.40))));
    painter.add(Shape::closed_line(principal, Stroke::new(1.3, con_alfa(nucleo, 0.95))));
    // (con los anillos muy juntos, el segundo hilo ya no se distingue)
    if grueso >= 0.6 {
        painter.add(Shape::closed_line(hilo(1, 1.3), Stroke::new(0.9, con_alfa(brillo, 0.6))));
    }
}
