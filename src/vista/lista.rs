//! Lista (detalles): una fila por archivo con tamaño, tipo y fecha; las
//! columnas se esconden si la ventana es angosta.

use super::{Disposicion, Visual, texto_ajustado, vecino_rejilla};
use crate::app::{Accion, Solar};
use crate::nucleo::{Entrada, Orden, TipoEntrada};
use crate::tema::{Paleta, con_alfa};
use crate::formato;
use eframe::egui::{
    Align2, CornerRadius, CursorIcon, FontId, Key, Painter, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};
use egui_phosphor::regular as ph;
use std::ops::Range;

pub(crate) const ALTO_FILA: f32 = 30.0;
const MARGEN: f32 = 6.0;

pub(crate) struct Lista {
    n: usize,
    ancho: f32,
    por_pagina: usize,
}

impl Lista {
    pub fn nueva(n: usize, ancho_disponible: f32, alto_visible: f32) -> Self {
        Self { n, ancho: ancho_disponible - 24.0, por_pagina: ((alto_visible / ALTO_FILA) as usize).max(1) }
    }

    /// Ancho de cada columna (las que no caben no se muestran).
    fn columnas(ancho: f32) -> (f32, Option<f32>, Option<f32>, Option<f32>) {
        let tam = (ancho >= 420.0).then_some(96.0);
        let fecha = (ancho >= 520.0).then_some(150.0);
        let tipo = (ancho >= 700.0).then_some(180.0);
        let nombre = ancho - tam.unwrap_or(0.0) - fecha.unwrap_or(0.0) - tipo.unwrap_or(0.0);
        (nombre, tam, tipo, fecha)
    }
}

impl Disposicion for Lista {
    fn tamano(&self) -> Vec2 {
        vec2(self.ancho + 24.0, self.n as f32 * ALTO_FILA + MARGEN * 2.0)
    }

    fn rect(&self, k: usize) -> Rect {
        Rect::from_min_size(pos2(12.0, MARGEN + k as f32 * ALTO_FILA), vec2(self.ancho, ALTO_FILA))
    }

    fn rango(&self, visible: Rect) -> Range<usize> {
        let primera = ((visible.top() - MARGEN) / ALTO_FILA).floor().max(0.0) as usize;
        let ultima = (((visible.bottom() - MARGEN) / ALTO_FILA).ceil().max(0.0) as usize + 1).min(self.n);
        primera.min(self.n)..ultima
    }

    fn vecino(&self, actual: Option<usize>, tecla: Key) -> Option<usize> {
        vecino_rejilla(actual, tecla, self.n, 1, self.por_pagina)
    }

    fn zona(&self, rect: Rect) -> Rect {
        rect
    }

    fn caja_banda(&self, k: usize) -> Rect {
        self.rect(k).shrink2(vec2(8.0, 4.0))
    }

    /// En la lista, las filas entran desde un poco a la izquierda.
    fn al_aparecer(&self, rect: Rect, t: f32) -> Rect {
        rect.translate(vec2(-(1.0 - t) * 12.0 * 0.8, 0.0))
    }

    fn pintar(&self, app: &Solar, painter: &Painter, e: &Entrada, rect: Rect, v: &Visual) {
        let p = &app.paleta;
        let radio = CornerRadius::same(6);
        if v.seleccionado {
            painter.rect(rect, radio, p.seleccion, Stroke::new(1.0, p.seleccion_borde), StrokeKind::Inside);
        } else if v.soltar_aqui {
            painter.rect(rect, radio, con_alfa(p.acento, 0.18), Stroke::new(2.0, p.acento), StrokeKind::Inside);
        } else {
            if v.indice % 2 == 1 {
                painter.rect_filled(rect, radio, con_alfa(p.texto, 0.025));
            }
            if v.hover > 0.0 {
                painter.rect_filled(rect, radio, con_alfa(p.hover, v.hover * p.hover.a() as f32 / 255.0));
            }
        }
        let lado = 24.0 * (1.0 + 0.2 * v.salto);
        let icono = Rect::from_center_size(pos2(rect.left() + 17.0 + 2.0 * v.hover, rect.center().y), vec2(lado, lado));
        app.pintar_icono(painter, icono, e, v.alfa);

        let (nombre, tam, tipo, fecha) = Self::columnas(rect.width());
        let color = if v.alfa < 1.0 { con_alfa(p.texto, 0.75) } else { p.texto };
        let g = texto_ajustado(painter, &e.nombre, nombre - 44.0, 1, 13.5, color, false);
        painter.galley(pos2(rect.left() + 34.0, rect.center().y - g.size().y / 2.0), g, color);
        let mut x = rect.left() + nombre;
        if let Some(w) = tam {
            let texto = if e.tipo == TipoEntrada::Archivo { formato::tamano(e.tamano) } else { "—".into() };
            painter.text(pos2(x + w - 10.0, rect.center().y), Align2::RIGHT_CENTER, texto, FontId::proportional(12.5), p.texto_tenue);
            x += w;
        }
        if let Some(w) = tipo {
            let g = texto_ajustado(painter, &e.descripcion, w - 16.0, 1, 12.5, p.texto_tenue, false);
            painter.galley(pos2(x + 8.0, rect.center().y - g.size().y / 2.0), g, p.texto_tenue);
            x += w;
        }
        if fecha.is_some() {
            painter.text(pos2(x + 8.0, rect.center().y), Align2::LEFT_CENTER, formato::fecha(e.modificado), FontId::proportional(12.5), p.texto_tenue);
        }
    }

    /// Los títulos de las columnas (clic para ordenar).
    fn encabezado(&self, app: &Solar, ui: &mut Ui, p: &Paleta, acciones: &mut Vec<Accion>) {
        let ancho = ui.available_width() - 24.0;
        let (nombre, tam, tipo, fecha) = Self::columnas(ancho);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
        ui.painter().line_segment(
            [rect.left_bottom() + vec2(12.0, 0.0), rect.right_bottom() - vec2(12.0, 0.0)],
            Stroke::new(1.0, con_alfa(p.borde, 0.8)),
        );
        let av = app.av();
        let mut x = rect.left() + 12.0;
        let columnas = [
            (Some(nombre), "Nombre", Orden::Nombre, false),
            (tam, "Tamaño", Orden::Tamano, true),
            (tipo, "Tipo", Orden::Tipo, false),
            (fecha, "Modificado", Orden::Fecha, false),
        ];
        for (ancho_col, titulo, orden, derecha) in columnas {
            let Some(w) = ancho_col else { continue };
            let caja = Rect::from_min_size(pos2(x, rect.top()), vec2(w, rect.height()));
            let resp = ui.interact(caja, ui.id().with(("columna", titulo)), Sense::click());
            let activa = av.orden == orden;
            let flecha = if !activa {
                ""
            } else if av.orden_descendente {
                ph::CARET_DOWN
            } else {
                ph::CARET_UP
            };
            let color = if activa || resp.hovered() { p.texto } else { p.texto_tenue };
            let texto = format!("{titulo} {flecha}");
            let (pos, ancla) = if derecha {
                (pos2(caja.right() - 10.0, caja.center().y), Align2::RIGHT_CENTER)
            } else {
                (pos2(caja.left() + if titulo == "Nombre" { 34.0 } else { 8.0 }, caja.center().y), Align2::LEFT_CENTER)
            };
            ui.painter().text(pos, ancla, texto, FontId::proportional(12.5), color);
            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                acciones.push(Accion::Ordenar(orden));
            }
            x += w;
        }
    }
}
