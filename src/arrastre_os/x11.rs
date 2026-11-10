//! Arrastrar archivos hacia otras ventanas en X11 (protocolo XDND, versión 5)
//! con una conexión aparte. Recibir lo hace winit; aquí solo se pregunta
//! dónde quedó el puntero.

use eframe::egui;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ClientMessageEvent, ConnectionExt as _, CreateWindowAux, EventMask, GrabMode, GrabStatus,
    KeyButMask, PropMode, SELECTION_NOTIFY_EVENT, SelectionNotifyEvent, SelectionRequestEvent, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::{CURRENT_TIME, NONE};

use super::{Crudo, TIPOS};

/// Para soltar el «agarre» del puntero que tiene winit desde el clic
struct Xlib {
    xlib: x11_dl::xlib::Xlib,
    xi: Option<x11_dl::xinput2::XInput2>,
    display: *mut x11_dl::xlib::Display,
}

pub struct X11 {
    conn: Arc<RustConnection>,
    ventana: Window,
    propia: Window,
    despertar: Atom,
    tx: mpsc::Sender<Vec<PathBuf>>,
    xlib: Option<Box<Xlib>>,
}

#[derive(Clone, Copy)]
struct Atomos {
    aware: Atom,
    seleccion: Atom,
    enter: Atom,
    leave: Atom,
    position: Atom,
    status: Atom,
    drop: Atom,
    copiar: Atom,
    lista: Atom,
    targets: Atom,
    despertar: Atom,
    tipos: [Atom; 4],
}

impl X11 {
    pub fn nuevo(
        display: Option<*mut std::ffi::c_void>,
        ventana: u32,
        eventos: mpsc::Sender<Crudo>,
        ctx: egui::Context,
    ) -> Option<Self> {
        let (conn, pantalla) = x11rb::connect(None).ok()?;
        let conn = Arc::new(conn);
        let raiz = conn.setup().roots.get(pantalla)?.root;
        // Se piden todos juntos y luego se esperan (una sola ida y vuelta)
        let nombres = [
            "XdndAware", "XdndSelection", "XdndEnter", "XdndLeave", "XdndPosition", "XdndStatus", "XdndDrop",
            "XdndActionCopy", "XdndTypeList", "TARGETS", "_SOLAR_DESPERTAR", TIPOS[0], TIPOS[1], TIPOS[2], TIPOS[3],
        ];
        let pedidos: Vec<_> = nombres.iter().map(|n| conn.intern_atom(false, n.as_bytes()).ok()).collect();
        let mut v = Vec::with_capacity(nombres.len());
        for p in pedidos {
            v.push(p?.reply().ok()?.atom);
        }
        let a = Atomos {
            aware: v[0],
            seleccion: v[1],
            enter: v[2],
            leave: v[3],
            position: v[4],
            status: v[5],
            drop: v[6],
            copiar: v[7],
            lista: v[8],
            targets: v[9],
            despertar: v[10],
            tipos: [v[11], v[12], v[13], v[14]],
        };
        // Ventana invisible: dueña de la selección y buzón de los mensajes XDND
        let propia = conn.generate_id().ok()?;
        conn.create_window(0, propia, raiz, -10, -10, 1, 1, 0, WindowClass::INPUT_ONLY, 0, &CreateWindowAux::new())
            .ok()?;
        conn.change_property32(PropMode::REPLACE, propia, a.lista, AtomEnum::ATOM, &a.tipos).ok()?;
        let cursor = cursor_mano(&conn).unwrap_or(NONE);
        conn.flush().ok()?;

        let (tx, rx) = mpsc::channel();
        let mut hilo = Hilo {
            conn: conn.clone(),
            raiz,
            propia,
            a,
            cursor,
            rx,
            eventos,
            ctx,
            rutas: Vec::new(),
            activo: false,
            destino: None,
            aceptado: false,
            esperando: false,
            pendiente: None,
            soltar_pendiente: None,
        };
        std::thread::Builder::new().name("solar-arrastre".into()).spawn(move || hilo.correr()).ok()?;

        let xlib = display.and_then(|d| {
            let xlib = x11_dl::xlib::Xlib::open().ok()?;
            Some(Box::new(Xlib { xlib, xi: x11_dl::xinput2::XInput2::open().ok(), display: d.cast() }))
        });
        Some(Self { conn, ventana, propia, despertar: a.despertar, tx, xlib })
    }

    pub fn iniciar(&mut self, rutas: Vec<PathBuf>) -> bool {
        // winit tiene el puntero agarrado desde el clic: hay que soltarlo
        // para que lo agarre la otra conexión
        if let Some(x) = &self.xlib {
            unsafe {
                (x.xlib.XUngrabPointer)(x.display, 0);
                if let Some(xi) = &x.xi {
                    let mut id = 0;
                    if (xi.XIGetClientPointer)(x.display, 0, &mut id) != 0 {
                        (xi.XIUngrabDevice)(x.display, id, 0);
                    }
                }
                (x.xlib.XSync)(x.display, 0);
            }
        }
        if self.tx.send(rutas).is_err() {
            return false;
        }
        let ev = ClientMessageEvent::new(32, self.propia, self.despertar, [0u32; 5]);
        self.conn.send_event(false, self.propia, EventMask::NO_EVENT, ev).is_ok() && self.conn.flush().is_ok()
    }

    /// Dónde está el puntero dentro de la ventana (en píxeles).
    pub fn puntero(&self) -> Option<(f64, f64)> {
        let r = self.conn.query_pointer(self.ventana).ok()?.reply().ok()?;
        Some((r.win_x as f64, r.win_y as f64))
    }
}

fn cursor_mano(c: &RustConnection) -> Option<u32> {
    let fuente = c.generate_id().ok()?;
    c.open_font(fuente, b"cursor").ok()?;
    let cursor = c.generate_id().ok()?;
    // XC_hand2 = 60 (y su máscara, 61)
    c.create_glyph_cursor(cursor, fuente, fuente, 60, 61, 0, 0, 0, 0xffff, 0xffff, 0xffff).ok()?;
    c.close_font(fuente).ok()?;
    Some(cursor)
}

struct Hilo {
    conn: Arc<RustConnection>,
    raiz: Window,
    propia: Window,
    a: Atomos,
    cursor: u32,
    rx: mpsc::Receiver<Vec<PathBuf>>,
    eventos: mpsc::Sender<Crudo>,
    ctx: egui::Context,
    /// Lo que se ofrece (se queda después de soltar: el destino lo pide luego)
    rutas: Vec<PathBuf>,
    activo: bool,
    destino: Option<(Window, u32)>,
    aceptado: bool,
    /// Ya mandamos una posición y falta la respuesta
    esperando: bool,
    pendiente: Option<(i16, i16, u32)>,
    soltar_pendiente: Option<u32>,
}

impl Hilo {
    fn correr(&mut self) {
        let mut cola: VecDeque<Event> = VecDeque::new();
        loop {
            if cola.is_empty() {
                match self.conn.wait_for_event() {
                    Ok(e) => cola.push_back(e),
                    Err(_) => return,
                }
            }
            while let Ok(Some(e)) = self.conn.poll_for_event() {
                cola.push_back(e);
            }
            while let Ok(rutas) = self.rx.try_recv() {
                self.empezar(rutas);
            }
            let Some(e) = cola.pop_front() else { continue };
            match e {
                Event::MotionNotify(m) if self.activo => {
                    // Solo importa el último movimiento
                    if matches!(cola.front(), Some(Event::MotionNotify(_))) {
                        continue;
                    }
                    self.mover(m.root_x, m.root_y, m.time);
                }
                Event::ButtonRelease(b) if self.activo => self.soltar(b.time),
                Event::ClientMessage(c) if c.type_ == self.a.status => {
                    let d = c.data.as_data32();
                    if self.destino.is_some_and(|(w, _)| w == d[0]) {
                        self.aceptado = d[1] & 1 == 1;
                        self.esperando = false;
                        if let Some(t) = self.soltar_pendiente.take() {
                            self.terminar(t);
                        } else if let Some((x, y, t)) = self.pendiente.take() {
                            self.mover(x, y, t);
                        }
                    }
                }
                Event::SelectionRequest(s) => self.responder(s),
                _ => {}
            }
            let _ = self.conn.flush();
        }
    }

    fn avisar(&self, c: Crudo) {
        let _ = self.eventos.send(c);
        self.ctx.request_repaint();
    }

    fn mandar(&self, a: Window, tipo: Atom, datos: [u32; 5]) {
        let ev = ClientMessageEvent::new(32, a, tipo, datos);
        let _ = self.conn.send_event(false, a, EventMask::NO_EVENT, ev);
    }

    fn empezar(&mut self, rutas: Vec<PathBuf>) {
        self.rutas = rutas;
        self.destino = None;
        self.aceptado = false;
        self.esperando = false;
        self.pendiente = None;
        self.soltar_pendiente = None;
        let _ = self.conn.set_selection_owner(self.propia, self.a.seleccion, CURRENT_TIME);
        let mascara = EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION;
        let mut agarrado = false;
        for _ in 0..25 {
            let r = self
                .conn
                .grab_pointer(false, self.raiz, mascara, GrabMode::ASYNC, GrabMode::ASYNC, NONE, self.cursor, CURRENT_TIME)
                .ok()
                .and_then(|c| c.reply().ok());
            if r.is_some_and(|r| r.status == GrabStatus::SUCCESS) {
                agarrado = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(8));
        }
        if !agarrado {
            self.avisar(Crudo::PropioTermino);
            return;
        }
        self.activo = true;
        match self.conn.query_pointer(self.raiz).ok().and_then(|c| c.reply().ok()) {
            // Ya soltaron el botón mientras tanto
            Some(q) if !q.mask.contains(KeyButMask::BUTTON1) => self.soltar(CURRENT_TIME),
            Some(q) => self.mover(q.root_x, q.root_y, CURRENT_TIME),
            None => self.soltar(CURRENT_TIME),
        }
    }

    /// La ventana (con XdndAware) que está bajo el puntero, y su versión.
    fn buscar(&self) -> Option<(Window, u32)> {
        let mut w = self.raiz;
        for _ in 0..24 {
            let hijo = self.conn.query_pointer(w).ok()?.reply().ok()?.child;
            if hijo == NONE || hijo == self.propia {
                return None;
            }
            let v = self
                .conn
                .get_property(false, hijo, self.a.aware, AtomEnum::ATOM, 0, 1)
                .ok()
                .and_then(|c| c.reply().ok())
                .and_then(|r| r.value32().and_then(|mut i| i.next()));
            if let Some(v) = v
                && v >= 3
            {
                return Some((hijo, v.min(5)));
            }
            w = hijo;
        }
        None
    }

    fn mover(&mut self, x: i16, y: i16, t: u32) {
        let nuevo = self.buscar();
        if nuevo.map(|d| d.0) != self.destino.map(|d| d.0) {
            if let Some((w, _)) = self.destino {
                self.mandar(w, self.a.leave, [self.propia, 0, 0, 0, 0]);
            }
            self.destino = nuevo;
            self.aceptado = false;
            self.esperando = false;
            self.pendiente = None;
            if let Some((w, v)) = nuevo {
                let t = self.a.tipos;
                // bit 0: hay más de tres tipos (están en XdndTypeList)
                self.mandar(w, self.a.enter, [self.propia, (v << 24) | 1, t[0], t[1], t[2]]);
            }
        }
        if let Some((w, _)) = self.destino {
            if self.esperando {
                self.pendiente = Some((x, y, t));
            } else {
                let xy = ((x as u16 as u32) << 16) | y as u16 as u32;
                self.mandar(w, self.a.position, [self.propia, 0, xy, t, self.a.copiar]);
                self.esperando = true;
            }
        }
    }

    fn soltar(&mut self, t: u32) {
        let _ = self.conn.ungrab_pointer(CURRENT_TIME);
        self.activo = false;
        self.pendiente = None;
        self.avisar(Crudo::PropioTermino);
        if self.esperando && self.destino.is_some() {
            // Primero que conteste si acepta
            self.soltar_pendiente = Some(t);
        } else {
            self.terminar(t);
        }
    }

    fn terminar(&mut self, t: u32) {
        if let Some((w, _)) = self.destino.take() {
            if self.aceptado {
                self.mandar(w, self.a.drop, [self.propia, 0, t, 0, 0]);
            } else {
                self.mandar(w, self.a.leave, [self.propia, 0, 0, 0, 0]);
            }
        }
    }

    fn responder(&self, s: SelectionRequestEvent) {
        let prop = if s.property == NONE { s.target } else { s.property };
        let mut ok = false;
        if s.selection == self.a.seleccion && !self.rutas.is_empty() {
            if s.target == self.a.targets {
                let mut lista = vec![self.a.targets];
                lista.extend_from_slice(&self.a.tipos);
                ok = self.conn.change_property32(PropMode::REPLACE, s.requestor, prop, AtomEnum::ATOM, &lista).is_ok();
            } else if let Some(i) = self.a.tipos.iter().position(|&t| t == s.target)
                && let Some(d) = super::datos_para(TIPOS[i], &self.rutas)
            {
                ok = self.conn.change_property8(PropMode::REPLACE, s.requestor, prop, s.target, &d).is_ok();
            }
        }
        let ev = SelectionNotifyEvent {
            response_type: SELECTION_NOTIFY_EVENT,
            sequence: 0,
            time: s.time,
            requestor: s.requestor,
            selection: s.selection,
            target: s.target,
            property: if ok { prop } else { NONE },
        };
        let _ = self.conn.send_event(false, s.requestor, EventMask::NO_EVENT, ev);
    }
}
