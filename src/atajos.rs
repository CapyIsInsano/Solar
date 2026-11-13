//! Atajos de teclado: se leen de config.lua ("Ctrl+Shift+N") y se pueden
//! cambiar en Ajustes → Atajos.

use eframe::egui::{self, Key, Modifiers};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Combinacion {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub tecla: Key,
}

impl Combinacion {
    pub fn parsear(texto: &str) -> Option<Self> {
        let texto = texto.trim();
        if texto.is_empty() {
            return None;
        }
        let mut c = Combinacion { ctrl: false, shift: false, alt: false, tecla: Key::Escape };
        let partes: Vec<&str> = texto.split('+').map(str::trim).collect();
        // "Ctrl++" -> la última parte vacía es la tecla '+'
        let (mods, tecla) = if texto.ends_with("++") {
            (&partes[..partes.len().saturating_sub(2)], "Plus")
        } else {
            (&partes[..partes.len() - 1], *partes.last()?)
        };
        for m in mods {
            match m.to_lowercase().as_str() {
                "ctrl" | "control" | "ctl" | "cmd" | "super" => c.ctrl = true,
                "shift" | "mayus" | "mayús" => c.shift = true,
                "alt" | "option" => c.alt = true,
                "" => {}
                _ => return None,
            }
        }
        c.tecla = tecla_de_nombre(tecla)?;
        Some(c)
    }

    /// Texto para config.lua: "Ctrl+Shift+N".
    pub fn texto(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        s.push_str(self.tecla.name());
        s
    }

    /// Texto bonito para la interfaz: "Ctrl + ←".
    pub fn mostrar(&self) -> String {
        let mut partes: Vec<String> = Vec::new();
        if self.ctrl {
            partes.push("Ctrl".into());
        }
        if self.shift {
            partes.push("Shift".into());
        }
        if self.alt {
            partes.push("Alt".into());
        }
        partes.push(
            match self.tecla {
                Key::ArrowLeft => egui_phosphor::regular::ARROW_LEFT.into(),
                Key::ArrowRight => egui_phosphor::regular::ARROW_RIGHT.into(),
                Key::ArrowUp => egui_phosphor::regular::ARROW_UP.into(),
                Key::ArrowDown => egui_phosphor::regular::ARROW_DOWN.into(),
                Key::Plus => "+".into(),
                Key::Minus => "−".into(),
                Key::Comma => ",".into(),
                Key::Delete => "Supr".into(),
                Key::Enter => "Enter".into(),
                Key::Home => "Inicio".into(),
                Key::Backspace => "Retroceso".into(),
                Key::Space => "Espacio".into(),
                Key::Tab => "Tab".into(),
                otra => otra.name().into(),
            },
        );
        partes.join(" + ")
    }

    pub fn coincide(&self, tecla: Key, m: Modifiers) -> bool {
        let tecla_igual = self.tecla == tecla || (self.tecla == Key::Plus && tecla == Key::Equals);
        tecla_igual && self.ctrl == (m.ctrl || m.command) && self.shift == m.shift && self.alt == m.alt
    }

    pub fn de_evento(tecla: Key, m: Modifiers) -> Self {
        Self { ctrl: m.ctrl || m.command, shift: m.shift, alt: m.alt, tecla }
    }
}

fn tecla_de_nombre(n: &str) -> Option<Key> {
    let alias = match n.to_lowercase().as_str() {
        "izquierda" | "left" => "ArrowLeft",
        "derecha" | "right" => "ArrowRight",
        "arriba" | "up" => "ArrowUp",
        "abajo" | "down" => "ArrowDown",
        "supr" | "suprimir" | "del" => "Delete",
        "inicio" => "Home",
        "fin" => "End",
        "intro" | "return" => "Enter",
        "esc" | "escape" => "Escape",
        "espacio" | "space" => "Space",
        "retroceso" => "Backspace",
        "mas" | "más" | "plus" => "Plus",
        "menos" | "minus" => "Minus",
        "coma" | "comma" => "Comma",
        _ => n,
    };
    Key::from_name(alias).or_else(|| Key::from_name(&alias.to_uppercase()))
}

pub struct Accion {
    pub id: &'static str,
    pub nombre: &'static str,
    pub grupo: &'static str,
}

const NAV: &str = "Navegación";
const ARCH: &str = "Archivos";
const VIS: &str = "Vista";
const PES: &str = "Pestañas y espacios";
const VEN: &str = "Ventana";

pub const ACCIONES: &[Accion] = &[
    Accion { id: "atras", nombre: "Atrás", grupo: NAV },
    Accion { id: "adelante", nombre: "Adelante", grupo: NAV },
    Accion { id: "subir", nombre: "Carpeta superior", grupo: NAV },
    Accion { id: "inicio", nombre: "Carpeta personal", grupo: NAV },
    Accion { id: "recargar", nombre: "Recargar", grupo: NAV },
    Accion { id: "ubicacion", nombre: "Escribir una ruta", grupo: NAV },
    Accion { id: "buscar", nombre: "Buscar en esta carpeta", grupo: NAV },
    Accion { id: "nueva_carpeta", nombre: "Nueva carpeta", grupo: ARCH },
    Accion { id: "nuevo_archivo", nombre: "Nuevo archivo", grupo: ARCH },
    Accion { id: "renombrar", nombre: "Renombrar", grupo: ARCH },
    Accion { id: "copiar", nombre: "Copiar", grupo: ARCH },
    Accion { id: "cortar", nombre: "Cortar", grupo: ARCH },
    Accion { id: "pegar", nombre: "Pegar", grupo: ARCH },
    Accion { id: "duplicar", nombre: "Duplicar", grupo: ARCH },
    Accion { id: "seleccionar_todo", nombre: "Seleccionar todo", grupo: ARCH },
    Accion { id: "papelera", nombre: "Mover a la papelera", grupo: ARCH },
    Accion { id: "eliminar", nombre: "Eliminar para siempre", grupo: ARCH },
    Accion { id: "propiedades", nombre: "Propiedades", grupo: ARCH },
    Accion { id: "personalizar", nombre: "Personalizar carpeta (color y logotipo)", grupo: ARCH },
    Accion { id: "agregar_lugar", nombre: "Añadir a Lugares", grupo: ARCH },
    Accion { id: "ocultos", nombre: "Mostrar archivos ocultos", grupo: VIS },
    Accion { id: "vista_cuadricula", nombre: "Vista de cuadrícula", grupo: VIS },
    Accion { id: "vista_lista", nombre: "Vista de lista (detalles)", grupo: VIS },
    Accion { id: "vista_mosaicos", nombre: "Vista de mosaicos", grupo: VIS },
    Accion { id: "vista_orbita", nombre: "Vista Órbita", grupo: VIS },
    Accion { id: "acercar", nombre: "Íconos más grandes", grupo: VIS },
    Accion { id: "alejar", nombre: "Íconos más pequeños", grupo: VIS },
    Accion { id: "barra_lateral", nombre: "Mostrar u ocultar la barra lateral", grupo: VIS },
    Accion { id: "nueva_pestana", nombre: "Pestaña nueva", grupo: PES },
    Accion { id: "cerrar_pestana", nombre: "Cerrar pestaña", grupo: PES },
    Accion { id: "siguiente_pestana", nombre: "Pestaña siguiente", grupo: PES },
    Accion { id: "anterior_pestana", nombre: "Pestaña anterior", grupo: PES },
    Accion { id: "reabrir_pestana", nombre: "Reabrir la última pestaña cerrada", grupo: PES },
    Accion { id: "espacios", nombre: "Menú de espacios de trabajo", grupo: PES },
    Accion { id: "terminal", nombre: "Abrir terminal aquí", grupo: VEN },
    Accion { id: "ajustes", nombre: "Ajustes", grupo: VEN },
    Accion { id: "nueva_ventana", nombre: "Nueva ventana", grupo: VEN },
    Accion { id: "cerrar", nombre: "Cerrar Solar", grupo: VEN },
];

pub const GRUPOS: [&str; 5] = [NAV, ARCH, VIS, PES, VEN];

pub fn accion(id: &str) -> Option<&'static Accion> {
    ACCIONES.iter().find(|a| a.id == id)
}

/// Acciones que tiene sentido repetir si se deja la tecla presionada.
fn repetible(id: &str) -> bool {
    matches!(id, "acercar" | "alejar" | "atras" | "adelante")
}

#[derive(Clone)]
pub struct Atajos {
    pub lista: Vec<(String, Option<Combinacion>)>,
}

impl Atajos {
    /// Convierte los atajos de config.lua. Devuelve avisos de los que no se entienden.
    pub fn de_config(atajos: &[(String, String)]) -> (Self, Vec<String>) {
        let mut avisos = Vec::new();
        let mut lista: Vec<(String, Option<Combinacion>)> = atajos
            .iter()
            .map(|(id, teclas)| {
                let c = Combinacion::parsear(teclas);
                if c.is_none() && !teclas.trim().is_empty() {
                    avisos.push(format!("No entiendo el atajo «{teclas}» de «{id}».",));
                }
                (id.clone(), c)
            })
            .collect();
        // Al actualizar Solar, un atajo nuevo puede chocar con uno que tú ya
        // habías cambiado (por ejemplo Ctrl+T para la terminal): gana el tuyo
        // y el nuevo se queda sin atajo
        let defecto = crate::nucleo::atajos_defecto();
        let es_defecto = |id: &str, c: &Combinacion| defecto.iter().any(|(d, t)| d == id && Combinacion::parsear(t).as_ref() == Some(c));
        for i in 0..lista.len() {
            let (id, Some(c)) = lista[i].clone() else { continue };
            if !es_defecto(&id, &c) {
                continue;
            }
            let tuyo = lista.iter().find(|(otro, oc)| *otro != id && oc.as_ref() == Some(&c) && !es_defecto(otro, &c));
            if let Some((otro, _)) = tuyo {
                let nombre = |x: &str| accion(x).map(|a| a.nombre.to_string()).unwrap_or_else(|| x.to_string());
                avisos.push(format!(
                    "«{}» se quedó sin atajo porque {} ya es «{}». Puedes darle otro en Ajustes › Atajos.",
                    nombre(&id),
                    c.texto(),
                    nombre(otro)
                ));
                lista[i].1 = None;
            }
        }
        (Self { lista }, avisos)
    }

    pub fn de(&self, id: &str) -> Option<Combinacion> {
        self.lista.iter().find(|(a, _)| a == id).and_then(|(_, c)| *c)
    }

    pub fn texto(&self, id: &str) -> String {
        self.de(id).map(|c| c.mostrar()).unwrap_or_default()
    }

    /// Acciones activadas este cuadro. egui convierte Ctrl+C/X/V en eventos
    /// Copy/Cut/Paste (sin evento de tecla), así que los traducimos.
    pub fn detectar(&self, ctx: &egui::Context) -> Vec<String> {
        let mut activadas: Vec<String> = Vec::new();
        ctx.input(|i| {
            let m = i.modifiers;
            for ev in &i.events {
                let (tecla, mods, repetida) = match ev {
                    egui::Event::Key { key, pressed: true, modifiers, repeat, .. } => (*key, *modifiers, *repeat),
                    egui::Event::Copy => (Key::C, Modifiers { ctrl: true, ..m }, false),
                    egui::Event::Cut => (Key::X, Modifiers { ctrl: true, ..m }, false),
                    egui::Event::Paste(_) => (Key::V, Modifiers { ctrl: true, ..m }, false),
                    _ => continue,
                };
                for (id, c) in &self.lista {
                    if let Some(c) = c
                        && c.coincide(tecla, mods)
                        && (!repetida || repetible(id))
                        && !activadas.contains(id)
                    {
                        activadas.push(id.clone());
                    }
                }
            }
        });
        activadas
    }

    /// Otra acción que ya usa esta combinación.
    pub fn conflicto(&self, id: &str, c: &Combinacion) -> Option<String> {
        self.lista
            .iter()
            .find(|(otro, oc)| otro != id && oc.as_ref() == Some(c))
            .map(|(otro, _)| otro.clone())
    }

    pub fn a_config(&self) -> Vec<(String, String)> {
        self.lista.iter().map(|(id, c)| (id.clone(), c.map(|c| c.texto()).unwrap_or_default())).collect()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn parsear_y_mostrar() {
        let c = Combinacion::parsear("Ctrl+Shift+N").unwrap();
        assert!(c.ctrl && c.shift && !c.alt && c.tecla == Key::N);
        assert_eq!(c.texto(), "Ctrl+Shift+N");
        let a = Combinacion::parsear("alt+left").unwrap();
        assert_eq!(a.tecla, Key::ArrowLeft);
        assert_eq!(a.mostrar(), format!("Alt + {}", egui_phosphor::regular::ARROW_LEFT));
        assert_eq!(Combinacion::parsear("Ctrl++").unwrap().tecla, Key::Plus);
        assert_eq!(Combinacion::parsear("Ctrl+Plus").unwrap().texto(), "Ctrl+Plus");
        assert_eq!(Combinacion::parsear("F2").unwrap().tecla, Key::F2);
        assert_eq!(Combinacion::parsear("Supr").unwrap().tecla, Key::Delete);
        assert!(Combinacion::parsear("Hiper+X").is_none());
        assert!(Combinacion::parsear("").is_none());
    }

    #[test]
    fn todas_las_acciones_tienen_atajo_por_defecto() {
        let defecto = crate::nucleo::atajos_defecto();
        for a in ACCIONES {
            let (_, t) = defecto.iter().find(|(id, _)| id == a.id).expect(a.id);
            assert!(Combinacion::parsear(t).is_some(), "{} -> {t}", a.id);
        }
        assert_eq!(defecto.len(), ACCIONES.len());
        // Ninguna combinación repetida entre los de fábrica
        let (a, avisos) = Atajos::de_config(&defecto);
        assert!(avisos.is_empty(), "{avisos:?}");
        for (id, c) in &a.lista {
            assert!(a.conflicto(id, c.as_ref().unwrap()).is_none(), "{id}");
        }
    }

    #[test]
    fn un_atajo_nuevo_no_le_gana_al_tuyo() {
        // Como un config.lua de antes de las pestañas con la terminal en Ctrl+T
        let mut config = crate::nucleo::atajos_defecto();
        for (id, t) in config.iter_mut() {
            if id == "terminal" {
                *t = "Ctrl+T".into();
            }
        }
        let (a, avisos) = Atajos::de_config(&config);
        assert_eq!(a.texto("terminal"), Combinacion::parsear("Ctrl+T").unwrap().mostrar());
        assert!(a.de("nueva_pestana").is_none());
        assert_eq!(avisos.len(), 1);
        assert!(avisos[0].contains("Pestaña nueva") && avisos[0].contains("Abrir terminal aquí"), "{}", avisos[0]);
    }
}
