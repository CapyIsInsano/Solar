//! Arrastrar y soltar en Wayland con wl_data_device, en un hilo propio con
//! su cola de eventos sobre la misma conexión que usa la ventana.

use eframe::egui;
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::sync::mpsc;

use smithay_client_toolkit as sctk;

use sctk::data_device_manager::data_device::{DataDevice, DataDeviceHandler};
use sctk::data_device_manager::data_offer::{DataOfferHandler, DragOffer};
use sctk::data_device_manager::data_source::{DataSourceHandler, DragSource};
use sctk::data_device_manager::{DataDeviceManagerState, WritePipe};
use sctk::reexports::calloop::channel::{self, Channel, Sender};
use sctk::reexports::calloop::EventLoop;
use sctk::reexports::calloop_wayland_source::WaylandSource;
use sctk::reexports::client::backend::{Backend, ObjectId};
use sctk::reexports::client::globals::registry_queue_init;
use sctk::reexports::client::protocol::wl_data_device::WlDataDevice;
use sctk::reexports::client::protocol::wl_data_device_manager::DndAction;
use sctk::reexports::client::protocol::wl_data_source::WlDataSource;
use sctk::reexports::client::protocol::wl_compositor::WlCompositor;
use sctk::reexports::client::protocol::wl_shm;
use sctk::reexports::client::protocol::wl_pointer::WlPointer;
use sctk::reexports::client::protocol::wl_seat::WlSeat;
use sctk::reexports::client::protocol::wl_surface::WlSurface;
use sctk::reexports::client::{Connection, Dispatch, Proxy, QueueHandle};
use sctk::registry::{ProvidesRegistryState, RegistryState};
use sctk::seat::pointer::{PointerData, PointerEvent, PointerEventKind, PointerHandler};
use sctk::seat::{Capability, SeatHandler, SeatState};
use sctk::shm::slot::{Buffer, SlotPool};
use sctk::shm::{Shm, ShmHandler};
use sctk::{
    delegate_data_device, delegate_pointer, delegate_registry, delegate_seat, delegate_shm,
    registry_handlers,
};

use super::icono::{self, Icono};
use super::{Crudo, TIPOS};

enum Orden {
    Iniciar(Vec<PathBuf>, Icono),
}

pub struct Wayland {
    tx: Sender<Orden>,
}

impl Wayland {
    pub fn nuevo(
        display: *mut std::ffi::c_void,
        superficie: *mut std::ffi::c_void,
        eventos: mpsc::Sender<Crudo>,
        ctx: egui::Context,
    ) -> Option<Self> {
        // La conexión es la de winit: los objetos de la ventana se pueden usar aquí
        let backend = unsafe { Backend::from_foreign_display(display.cast()) };
        let conn = Connection::from_backend(backend);
        let id = unsafe { ObjectId::from_ptr(WlSurface::interface(), superficie.cast()) }.ok()?;
        let superficie = WlSurface::from_id(&conn, id).ok()?;
        let (tx, rx) = channel::channel();
        let (listo_tx, listo_rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("solar-arrastre".into())
            .spawn(move || hilo(conn, superficie, rx, eventos, ctx, listo_tx))
            .ok()?;
        // Esperar a saber si el compositor tiene lo necesario
        listo_rx.recv().ok().filter(|&ok| ok)?;
        Some(Self { tx })
    }

    pub fn iniciar(&mut self, rutas: Vec<PathBuf>, icono: Icono) -> bool {
        self.tx.send(Orden::Iniciar(rutas, icono)).is_ok()
    }
}

struct Asiento {
    seat: WlSeat,
    dispositivo: Option<DataDevice>,
    puntero: Option<WlPointer>,
}

struct Destino {
    tipo: Option<String>,
    soltado: bool,
}

struct Estado {
    registry: RegistryState,
    seats: SeatState,
    gestor: DataDeviceManagerState,
    asientos: Vec<Asiento>,
    /// Asiento y número de serie del último clic (el compositor lo exige)
    ultimo_clic: Option<(WlSeat, u32)>,
    superficie: WlSurface,
    conn: Connection,
    qh: QueueHandle<Estado>,
    origen: Option<(DragSource, Vec<PathBuf>)>,
    /// Para el dibujito que sigue al puntero (si el compositor deja)
    compositor: Option<WlCompositor>,
    shm: Option<Shm>,
    icono: Option<(WlSurface, SlotPool, Buffer)>,
    destino: Option<Destino>,
    eventos: mpsc::Sender<Crudo>,
    ctx: egui::Context,
}

fn hilo(
    conn: Connection,
    superficie: WlSurface,
    rx: Channel<Orden>,
    eventos: mpsc::Sender<Crudo>,
    ctx: egui::Context,
    listo: mpsc::Sender<bool>,
) {
    let Ok((globales, cola)) = registry_queue_init::<Estado>(&conn) else {
        let _ = listo.send(false);
        return;
    };
    let qh = cola.handle();
    let Ok(gestor) = DataDeviceManagerState::bind(&globales, &qh) else {
        let _ = listo.send(false);
        return;
    };
    let Ok(mut bucle) = EventLoop::<Estado>::try_new() else {
        let _ = listo.send(false);
        return;
    };
    let seats = SeatState::new(&globales, &qh);
    let mut estado = Estado {
        registry: RegistryState::new(&globales),
        asientos: Vec::new(),
        seats,
        gestor,
        ultimo_clic: None,
        superficie,
        conn: conn.clone(),
        qh: qh.clone(),
        origen: None,
        compositor: globales.bind(&qh, 1..=4, ()).ok(),
        shm: Shm::bind(&globales, &qh).ok(),
        icono: None,
        destino: None,
        eventos,
        ctx,
    };
    let ya: Vec<WlSeat> = estado.seats.seats().collect();
    for s in ya {
        estado.agregar_asiento(&qh, s);
    }
    let h = bucle.handle();
    let ok = h
        .insert_source(rx, |e, _, estado: &mut Estado| {
            if let channel::Event::Msg(Orden::Iniciar(rutas, icono)) = e {
                estado.iniciar(rutas, icono);
            }
        })
        .is_ok()
        && WaylandSource::new(conn, cola).insert(h).is_ok();
    let _ = listo.send(ok);
    if !ok {
        return;
    }
    while bucle.dispatch(None, &mut estado).is_ok() {}
}

impl Estado {
    fn agregar_asiento(&mut self, qh: &QueueHandle<Self>, seat: WlSeat) {
        if self.asientos.iter().any(|a| a.seat == seat) {
            return;
        }
        let dispositivo = Some(self.gestor.get_data_device(qh, &seat));
        self.asientos.push(Asiento { seat, dispositivo, puntero: None });
    }

    fn avisar(&self, c: Crudo) {
        let _ = self.eventos.send(c);
        self.ctx.request_repaint();
    }

    fn iniciar(&mut self, rutas: Vec<PathBuf>, icono: Icono) {
        let qh = self.qh.clone();
        let Some((seat, serial)) = self.ultimo_clic.clone() else {
            self.avisar(Crudo::PropioTermino);
            return;
        };
        let Some(dispositivo) =
            self.asientos.iter().find(|a| a.seat == seat).and_then(|a| a.dispositivo.as_ref()).map(|d| d.inner().clone())
        else {
            self.avisar(Crudo::PropioTermino);
            return;
        };
        let fuente = self.gestor.create_drag_and_drop_source(&qh, TIPOS, DndAction::Copy | DndAction::Move);
        self.quitar_icono();
        let superficie_icono = self.compositor.as_ref().map(|c| c.create_surface(&qh, ()));
        dispositivo.start_drag(Some(fuente.inner()), &self.superficie, superficie_icono.as_ref(), serial);
        if let Some(s) = superficie_icono {
            self.pintar_icono(s, icono);
        }
        // Si había otro (no debería), se cancela al soltarlo
        self.origen = Some((fuente, rutas));
        let _ = self.conn.flush();
    }

    /// El icono va pegado al puntero desde su esquina de arriba a la
    /// izquierda (el dibujo deja aire ahí para que no lo tape la flecha).
    fn pintar_icono(&mut self, s: WlSurface, icono: Icono) {
        let lado = (icono::LADO * icono::ESCALA) as i32;
        let Some(mut pool) = self.shm.as_ref().and_then(|shm| SlotPool::new((lado * lado * 4) as usize, shm).ok()) else {
            s.destroy();
            return;
        };
        let Ok((buffer, lienzo)) = pool.create_buffer(lado, lado, lado * 4, wl_shm::Format::Argb8888) else {
            s.destroy();
            return;
        };
        let px = icono::dibujar(icono);
        let n = lienzo.len().min(px.len());
        lienzo[..n].copy_from_slice(&px[..n]);
        s.set_buffer_scale(icono::ESCALA as i32);
        if buffer.attach_to(&s).is_ok() {
            s.damage_buffer(0, 0, lado, lado);
        }
        s.commit();
        let _ = self.conn.flush();
        self.icono = Some((s, pool, buffer));
    }

    fn quitar_icono(&mut self) {
        if let Some((s, _, _)) = self.icono.take() {
            s.destroy();
        }
    }

    fn terminar_propio(&mut self, fuente: &WlDataSource) {
        if self.origen.as_ref().is_some_and(|(f, _)| f.inner() == fuente) {
            self.origen = None;
            self.quitar_icono();
            self.avisar(Crudo::PropioTermino);
        }
    }

    fn oferta(&self, dispositivo: &WlDataDevice) -> Option<DragOffer> {
        self.asientos
            .iter()
            .filter_map(|a| a.dispositivo.as_ref())
            .find(|d| d.inner() == dispositivo)
            .and_then(|d| d.data().drag_offer())
    }
}

impl SeatHandler for Estado {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_seat(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: WlSeat) {
        self.agregar_asiento(qh, seat);
    }

    fn new_capability(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: WlSeat, c: Capability) {
        if c == Capability::Pointer {
            let p = self.seats.get_pointer(qh, &seat).ok();
            if let Some(a) = self.asientos.iter_mut().find(|a| a.seat == seat) {
                a.puntero = p;
            }
        }
    }

    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, seat: WlSeat, c: Capability) {
        if c == Capability::Pointer
            && let Some(a) = self.asientos.iter_mut().find(|a| a.seat == seat)
            && let Some(p) = a.puntero.take()
            && p.version() >= 3
        {
            p.release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, seat: WlSeat) {
        self.asientos.retain(|a| a.seat != seat);
    }
}

impl PointerHandler for Estado {
    fn pointer_frame(&mut self, _: &Connection, _: &QueueHandle<Self>, puntero: &WlPointer, eventos: &[PointerEvent]) {
        let Some(seat) = puntero.data::<PointerData>().map(|d| d.seat().clone()) else { return };
        for e in eventos {
            if let PointerEventKind::Press { serial, .. } = e.kind
                && e.surface == self.superficie
            {
                self.ultimo_clic = Some((seat.clone(), serial));
            }
        }
    }
}

impl DataDeviceHandler for Estado {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, d: &WlDataDevice, x: f64, y: f64, s: &WlSurface) {
        if s != &self.superficie {
            return;
        }
        let Some(oferta) = self.oferta(d) else { return };
        let tipo = oferta.with_mime_types(|tipos| {
            TIPOS.iter().find(|t| tipos.iter().any(|m| m == *t)).map(|t| t.to_string())
        });
        oferta.accept_mime_type(oferta.serial, tipo.clone());
        if tipo.is_some() {
            oferta.set_actions(DndAction::Copy | DndAction::Move, DndAction::Move);
        } else {
            oferta.set_actions(DndAction::empty(), DndAction::empty());
        }
        let _ = self.conn.flush();
        let sirve = tipo.is_some();
        self.destino = Some(Destino { tipo, soltado: false });
        if sirve {
            self.avisar(Crudo::Encima(x, y));
        }
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataDevice) {
        if let Some(d) = self.destino.take()
            && !d.soltado
            && d.tipo.is_some()
        {
            self.avisar(Crudo::Salio);
        }
    }

    fn motion(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataDevice, x: f64, y: f64) {
        if self.destino.as_ref().is_some_and(|d| d.tipo.is_some()) {
            self.avisar(Crudo::Encima(x, y));
        }
    }

    fn selection(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataDevice) {}

    fn drop_performed(&mut self, _: &Connection, _: &QueueHandle<Self>, d: &WlDataDevice) {
        let Some(destino) = self.destino.as_mut() else { return };
        destino.soltado = true;
        let tipo = destino.tipo.clone();
        let Some(oferta) = self.oferta(d) else { return };
        let Some(tipo) = tipo else {
            oferta.destroy();
            return;
        };
        let pos = (oferta.x, oferta.y);
        let copiar = oferta.selected_action == DndAction::Copy;
        let Ok(tubo) = oferta.receive(tipo) else {
            oferta.destroy();
            self.avisar(Crudo::Salio);
            return;
        };
        let _ = self.conn.flush();
        // Leer en otro hilo: si el arrastre es nuestro, este mismo hilo es el
        // que tiene que mandar los datos
        let (conn, eventos, ctx) = (self.conn.clone(), self.eventos.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let texto = super::leer_de(OwnedFd::from(tubo)).unwrap_or_default();
            let rutas = super::rutas_de(&texto);
            if !oferta.selected_action.is_empty() {
                oferta.finish();
            }
            oferta.destroy();
            let _ = conn.flush();
            let _ = eventos.send(if rutas.is_empty() { Crudo::Salio } else { Crudo::Soltado { rutas, pos, copiar } });
            ctx.request_repaint();
        });
    }
}

impl DataOfferHandler for Estado {
    fn source_actions(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &mut DragOffer, _: DndAction) {}

    fn selected_action(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &mut DragOffer, _: DndAction) {}
}

impl DataSourceHandler for Estado {
    fn accept_mime(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataSource, _: Option<String>) {}

    fn send_request(&mut self, _: &Connection, _: &QueueHandle<Self>, f: &WlDataSource, tipo: String, tubo: WritePipe) {
        let Some((_, rutas)) = self.origen.as_ref().filter(|(o, _)| o.inner() == f) else { return };
        if let Some(datos) = super::datos_para(&tipo, rutas) {
            super::escribir_en(OwnedFd::from(tubo), datos);
        }
    }

    fn cancelled(&mut self, _: &Connection, _: &QueueHandle<Self>, f: &WlDataSource) {
        self.terminar_propio(f);
    }

    fn dnd_dropped(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataSource) {}

    fn dnd_finished(&mut self, _: &Connection, _: &QueueHandle<Self>, f: &WlDataSource) {
        self.terminar_propio(f);
    }

    fn action(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlDataSource, _: DndAction) {}
}

impl ProvidesRegistryState for Estado {
    registry_handlers![SeatState];

    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
}

delegate_registry!(Estado);
delegate_seat!(Estado);
delegate_pointer!(Estado);
delegate_data_device!(Estado);

// El compositor y la superficie del icono no mandan nada que nos importe
impl Dispatch<WlCompositor, ()> for Estado {
    fn event(_: &mut Self, _: &WlCompositor, _: <WlCompositor as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlSurface, ()> for Estado {
    fn event(_: &mut Self, _: &WlSurface, _: <WlSurface as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl ShmHandler for Estado {
    fn shm_state(&mut self) -> &mut Shm {
        self.shm.as_mut().expect("solo se pide si existe")
    }
}

delegate_shm!(Estado);
