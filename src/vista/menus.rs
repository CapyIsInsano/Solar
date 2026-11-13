//! Menús del clic derecho y el menú «Vista» (como el de Windows: tamaños de
//! íconos, lista, mosaicos y órbita).

use super::info_vista;
use crate::app::{Accion, CambioEstilo, Solar, Ubicacion};
use crate::dialogos;
use crate::nucleo::{self, Afinidad, Orden, Programa, Vista};
use crate::ui_util;
use crate::formato;
use eframe::egui::{self, RichText, ScrollArea, Ui, vec2};
use egui_phosphor::regular as ph;
use std::path::PathBuf;
use std::rc::Rc;

/// Tamaños de íconos del menú Vista (como «Íconos grandes» de Windows).
pub(crate) const TAMANOS: [(&str, f32); 4] =
    [("Íconos muy grandes", 144.0), ("Íconos grandes", 104.0), ("Íconos medianos", 72.0), ("Íconos pequeños", 48.0)];

impl Solar {
    pub(crate) fn opcion(&self, ui: &mut Ui, glifo: &str, texto: &str, atajo: &str) -> bool {
        let r = ui.add(egui::Button::new(format!("{glifo}   {texto}")).shortcut_text(RichText::new(atajo).small().color(self.paleta.texto_tenue)));
        if r.clicked() {
            ui.close();
            true
        } else {
            false
        }
    }

    /// Programas para el submenú «Abrir con» (se calculan una vez por selección).
    fn programas_menu(&self, archivos: &[PathBuf]) -> Rc<Vec<Programa>> {
        if let Some((rutas, lista)) = self.menu_programas.borrow().as_ref()
            && rutas.as_slice() == archivos
        {
            return lista.clone();
        }
        let lista = Rc::new(nucleo::programas_para(archivos, false).map(|l| l.programas).unwrap_or_default());
        self.menu_programas.replace(Some((archivos.to_vec(), lista.clone())));
        lista
    }

    fn submenu_abrir_con(&self, ui: &mut Ui, archivos: Vec<PathBuf>, acciones: &mut Vec<Accion>) {
        if archivos.is_empty() {
            return;
        }
        ui.menu_button(format!("{}   Abrir con", ph::ARROW_SQUARE_OUT), |ui| {
            ui.set_min_width(240.0);
            let lista = self.programas_menu(&archivos);
            let mut hubo = false;
            for prog in lista.iter().filter(|p| p.afinidad > Afinidad::Otro).take(10) {
                hubo = true;
                let mut texto = prog.nombre.clone();
                if prog.afinidad == Afinidad::Predeterminado {
                    texto.push_str("  (predeterminado)");
                }
                let textura = self.iconos_programas.borrow_mut().textura(ui.ctx(), &prog.icono);
                let boton = match &textura {
                    Some(t) => egui::Button::image_and_text(egui::Image::new(t).fit_to_exact_size(vec2(18.0, 18.0)), texto),
                    None => egui::Button::new(format!("{}   {texto}", dialogos::glifo_programa(prog))),
                };
                let mut r = ui.add(boton);
                if prog.terminal {
                    r = r.on_hover_text("Se abre en la terminal");
                }
                if r.clicked() {
                    acciones.push(Accion::AbrirConPrograma { programa: Box::new(prog.clone()), rutas: archivos.clone() });
                    ui.close();
                }
            }
            if !hubo {
                ui.label(RichText::new("Ningún programa dice abrir esto").color(self.paleta.texto_tenue));
            }
            ui.separator();
            if self.opcion(ui, ph::DOTS_THREE_CIRCLE, "Otro programa…", "") {
                acciones.push(Accion::AbrirCon(archivos.clone()));
            }
        });
    }

    /// «Añadir a Lugares» o, si la carpeta ya está ahí, «Quitar de Lugares».
    fn opcion_lugar(&self, ui: &mut Ui, ruta: &std::path::Path, atajo: &str, acciones: &mut Vec<Accion>) {
        match self.indice_lugar(ruta) {
            Some(i) => {
                if self.opcion(ui, ph::STAR_HALF, "Quitar de Lugares", "") {
                    acciones.push(Accion::QuitarLugar(i));
                }
            }
            None => {
                if self.opcion(ui, ph::STAR, "Añadir a Lugares", atajo) {
                    acciones.push(Accion::AgregarLugar(ruta.to_path_buf()));
                }
            }
        }
    }

    fn opcion_peligro(&self, ui: &mut Ui, glifo: &str, texto: &str, atajo: &str) -> bool {
        let r = ui.add(
            egui::Button::new(RichText::new(format!("{glifo}   {texto}")).color(self.paleta.peligro))
                .shortcut_text(RichText::new(atajo).small().color(self.paleta.texto_tenue)),
        );
        if r.clicked() {
            ui.close();
            true
        } else {
            false
        }
    }

    pub(crate) fn menu_elemento(&self, ui: &mut Ui, ruta: &std::path::Path, es_carpeta: bool, acciones: &mut Vec<Accion>) {
        ui.set_min_width(250.0);
        let sel = {
            let s = self.seleccionados();
            if s.is_empty() { vec![ruta.to_path_buf()] } else { s }
        };
        let n = sel.len();
        let carpetas: Vec<PathBuf> = sel.iter().filter(|r| self.entrada(r).is_some_and(|e| e.es_carpeta())).cloned().collect();
        let todas_carpetas = carpetas.len() == n;
        let a = |id: &str| self.atajos.texto(id);

        if n == 1 {
            if self.opcion(ui, ph::FOLDER_OPEN, "Abrir", "Enter") {
                acciones.push(Accion::Abrir(ruta.to_path_buf()));
            }
            if es_carpeta {
                if self.opcion(ui, ph::BROWSERS, "Abrir en pestaña nueva", "Clic con la rueda") {
                    acciones.push(Accion::AbrirEnPestana(ruta.to_path_buf()));
                }
                if self.opcion(ui, ph::APP_WINDOW, "Abrir en ventana nueva", "") {
                    acciones.push(Accion::AbrirEnVentana(ruta.to_path_buf()));
                }
                if self.opcion(ui, ph::TERMINAL_WINDOW, "Abrir terminal aquí", "") {
                    acciones.push(Accion::AbrirTerminal(ruta.to_path_buf()));
                }
            } else {
                self.submenu_abrir_con(ui, sel.clone(), acciones);
            }
        } else {
            if self.opcion(ui, ph::FOLDER_OPEN, &format!("Abrir {}", formato::elementos(n)), "Enter") {
                acciones.push(Accion::AbrirSeleccion);
            }
            if todas_carpetas && n <= 12 && self.opcion(ui, ph::BROWSERS, "Abrir cada una en una pestaña", "") {
                for c in &carpetas {
                    acciones.push(Accion::AbrirEnPestana(c.clone()));
                }
            }
            if !todas_carpetas {
                self.submenu_abrir_con(ui, sel.iter().filter(|r| !carpetas.contains(r)).cloned().collect(), acciones);
            }
        }
        ui.separator();
        if self.opcion(ui, ph::SCISSORS, "Cortar", &a("cortar")) {
            acciones.push(Accion::Cortar(sel.clone()));
        }
        if self.opcion(ui, ph::COPY, "Copiar", &a("copiar")) {
            acciones.push(Accion::Copiar(sel.clone()));
        }
        if n == 1 && es_carpeta && self.portapapeles.is_some() && self.opcion(ui, ph::CLIPBOARD, "Pegar dentro", "") {
            acciones.push(Accion::Pegar(ruta.to_path_buf()));
        }
        if self.opcion(ui, ph::COPY_SIMPLE, "Duplicar", &a("duplicar")) {
            acciones.push(Accion::Duplicar(sel.clone()));
        }
        if n == 1 && self.opcion(ui, ph::PENCIL_SIMPLE, "Renombrar…", &a("renombrar")) {
            acciones.push(Accion::Renombrar(ruta.to_path_buf()));
        }

        if todas_carpetas {
            ui.separator();
            self.menus_estilo(ui, &carpetas, acciones);
        }

        ui.separator();
        if n == 1 && es_carpeta {
            self.opcion_lugar(ui, ruta, &a("agregar_lugar"), acciones);
        }
        if self.opcion(ui, ph::LINK, if n == 1 { "Copiar ruta" } else { "Copiar rutas" }, "") {
            acciones.push(Accion::CopiarRuta(sel.clone()));
        }
        ui.separator();
        if self.opcion(ui, ph::TRASH, "Mover a la papelera", &a("papelera")) {
            acciones.push(Accion::Papelera(sel.clone()));
        }
        if self.opcion_peligro(ui, ph::X_CIRCLE, "Eliminar para siempre…", &a("eliminar")) {
            acciones.push(Accion::Eliminar(sel.clone()));
        }
        ui.separator();
        if self.opcion(ui, ph::INFO, "Propiedades", &a("propiedades")) {
            acciones.push(Accion::Propiedades(sel));
        }
    }

    /// Submenús "Color de carpeta" y "Logotipo" (y el diálogo completo).
    pub(crate) fn menus_estilo(&self, ui: &mut Ui, carpetas: &[PathBuf], acciones: &mut Vec<Accion>) {
        let p = &self.paleta;
        let actual = carpetas.first().and_then(|r| self.estilo_de(r)).unwrap_or_default();
        ui.menu_button(format!("{}   Color de carpeta", ph::PALETTE), |ui| {
            ui.set_min_width(250.0);
            ui.label(RichText::new("Colores listos").small().color(p.texto_tenue));
            if let Some(c) = ui_util::rejilla_colores(ui, p, actual.color, 7, 11.0) {
                acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::Color(Some(c))));
                ui.close();
            }
            ui.separator();
            if self.opcion(ui, ph::EYEDROPPER, "Color personalizado…", "") {
                acciones.push(Accion::Personalizar(carpetas.to_vec()));
            }
            if actual.color.is_some() && self.opcion(ui, ph::ARROW_COUNTER_CLOCKWISE, "Usar el color del tema", "") {
                acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::Color(None)));
            }
        });
        ui.menu_button(format!("{}   Logotipo", ph::SMILEY), |ui| {
            ui.set_min_width(300.0);
            ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                if let Some(id) = ui_util::rejilla_logotipos(ui, p, actual.icono.as_deref(), "", 8, 32.0) {
                    acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::Logotipo(Some(id.to_string()))));
                    ui.close();
                }
            });
            ui.separator();
            ui.menu_button(format!("{}   Color del logotipo", ph::DROP), |ui| {
                ui.set_min_width(250.0);
                if let Some(c) = ui_util::rejilla_colores(ui, p, actual.color_icono, 7, 11.0) {
                    acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::ColorLogotipo(Some(c))));
                    ui.close();
                }
                ui.separator();
                if self.opcion(ui, ph::MAGIC_WAND, "Automático (según la carpeta)", "") {
                    acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::ColorLogotipo(None)));
                }
            });
            if self.opcion(ui, ph::TEXT_AA, "Texto o emoji…", "") {
                acciones.push(Accion::Personalizar(carpetas.to_vec()));
            }
            if actual.icono.is_some() && self.opcion(ui, ph::PROHIBIT, "Quitar logotipo", "") {
                acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::Logotipo(None)));
            }
        });
        if self.opcion(ui, ph::SPARKLE, "Personalizar carpeta…", &self.atajos.texto("personalizar")) {
            acciones.push(Accion::Personalizar(carpetas.to_vec()));
        }
        let alguna = carpetas.iter().any(|r| self.estilo_de(r).is_some_and(|e| !e.vacio()));
        if alguna && self.opcion(ui, ph::ARROW_COUNTER_CLOCKWISE, "Quitar personalización", "") {
            acciones.push(Accion::Estilo(carpetas.to_vec(), CambioEstilo::Quitar));
        }
    }

    /// El menú «Vista»: tamaños de íconos, lista, mosaicos y órbita.
    pub(crate) fn menu_vistas(&self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        ui.set_min_width(250.0);
        let vista = self.vista();
        let tam = self.av().tamano_iconos;
        // El tamaño más parecido al actual lleva la palomita
        let cercano = TAMANOS
            .iter()
            .min_by(|a, b| (a.1 - tam).abs().total_cmp(&(b.1 - tam).abs()))
            .map(|t| t.1)
            .unwrap_or(72.0);
        for (nombre, lado) in TAMANOS {
            let activo = vista == Vista::Cuadricula && lado == cercano;
            let atajo = if lado == 72.0 { self.atajos.texto("vista_cuadricula") } else { String::new() };
            let glifo = if activo { ph::CHECK } else { ph::SQUARES_FOUR };
            if self.opcion(ui, glifo, nombre, &atajo) {
                acciones.push(Accion::CambiarVista(Vista::Cuadricula));
                acciones.push(Accion::FijarTamano(lado));
            }
        }
        ui.separator();
        for v in [Vista::Lista, Vista::Mosaicos, Vista::Orbita] {
            let (glifo, nombre, id) = info_vista(v);
            let g = if vista == v { ph::CHECK } else { glifo };
            if self.opcion(ui, g, nombre, &self.atajos.texto(id)) {
                acciones.push(Accion::CambiarVista(v));
            }
        }
        ui.separator();
        let ocultos = self.av().mostrar_ocultos;
        if self.opcion(ui, if ocultos { ph::CHECK } else { ph::EYE_SLASH }, "Mostrar archivos ocultos", &self.atajos.texto("ocultos")) {
            acciones.push(Accion::AlternarOcultos);
        }
    }

    pub(crate) fn menu_fondo(&self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        ui.set_min_width(250.0);
        let a = |id: &str| self.atajos.texto(id);
        if self.ubicacion == Ubicacion::Papelera {
            if self.opcion(ui, ph::SELECTION_ALL, "Seleccionar todo", &a("seleccionar_todo")) {
                acciones.push(Accion::SeleccionarTodo);
            }
            if !self.entradas.is_empty() && self.opcion_peligro(ui, ph::BROOM, "Vaciar papelera…", "") {
                acciones.push(Accion::VaciarPapelera);
            }
            return;
        }
        let Some(dir) = self.ubicacion.ruta().map(PathBuf::from) else { return };
        if self.opcion(ui, ph::FOLDER_PLUS, "Nueva carpeta…", &a("nueva_carpeta")) {
            acciones.push(Accion::NuevaCarpeta);
        }
        if self.opcion(ui, ph::FILE_PLUS, "Nuevo archivo…", &a("nuevo_archivo")) {
            acciones.push(Accion::NuevoArchivo);
        }
        if self.portapapeles.is_some() && self.opcion(ui, ph::CLIPBOARD, "Pegar", &a("pegar")) {
            acciones.push(Accion::Pegar(dir.clone()));
        }
        ui.separator();
        if self.opcion(ui, ph::SELECTION_ALL, "Seleccionar todo", &a("seleccionar_todo")) {
            acciones.push(Accion::SeleccionarTodo);
        }
        ui.menu_button(format!("{}   Ordenar por", ph::SORT_ASCENDING), |ui| {
            let av = self.av();
            for o in Orden::TODOS {
                let marca = if av.orden == o { ph::CHECK } else { " " };
                if self.opcion(ui, marca, o.nombre(), "") {
                    acciones.push(Accion::Ordenar(o));
                }
            }
            ui.separator();
            let desc = av.orden_descendente;
            if self.opcion(ui, if desc { ph::CHECK } else { " " }, "Al revés", "") {
                acciones.push(Accion::OrdenDescendente(!desc));
            }
            let cp = av.carpetas_primero;
            if self.opcion(ui, if cp { ph::CHECK } else { " " }, "Carpetas primero", "") {
                acciones.push(Accion::CarpetasPrimero(!cp));
            }
        });
        let (glifo, _, _) = info_vista(self.vista());
        ui.menu_button(format!("{glifo}   Vista"), |ui| self.menu_vistas(ui, acciones));
        ui.separator();
        self.menus_estilo(ui, std::slice::from_ref(&dir), acciones);
        ui.separator();
        if self.opcion(ui, ph::BROWSERS, "Abrir en pestaña nueva", &a("nueva_pestana")) {
            acciones.push(Accion::AbrirEnPestana(dir.clone()));
        }
        if self.opcion(ui, ph::TERMINAL_WINDOW, "Abrir terminal aquí", &a("terminal")) {
            acciones.push(Accion::AbrirTerminal(dir.clone()));
        }
        self.opcion_lugar(ui, &dir, &a("agregar_lugar"), acciones);
        if self.opcion(ui, ph::INFO, "Propiedades de esta carpeta", "") {
            acciones.push(Accion::Propiedades(vec![dir]));
        }
    }

    pub(crate) fn menu_papelera(&self, ui: &mut Ui, acciones: &mut Vec<Accion>) {
        ui.set_min_width(230.0);
        let sel = self.seleccionados();
        if sel.is_empty() {
            return;
        }
        if self.opcion(ui, ph::ARROW_COUNTER_CLOCKWISE, "Restaurar", "") {
            acciones.push(Accion::Restaurar(sel.clone()));
        }
        if self.opcion_peligro(ui, ph::X_CIRCLE, "Eliminar para siempre…", &self.atajos.texto("eliminar")) {
            acciones.push(Accion::BorrarDePapelera(sel));
        }
    }
}
