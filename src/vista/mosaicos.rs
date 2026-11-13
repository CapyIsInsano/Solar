//! Mosaicos: ícono a la izquierda y, a un lado, el nombre, el tipo y el
//! tamaño (como los «Mosaicos» de Windows).

use super::{Disposicion, Visual, texto_ajustado, vecino_rejilla};
use crate::app::Solar;
use crate::nucleo::{Entrada, TipoEntrada};
use crate::tema::con_alfa;
use crate::formato;
use eframe::egui::{CornerRadius, Key, Painter, Rect, Stroke, StrokeKind, Vec2, pos2, vec2};
use std::ops::Range;

const MARGEN: f32 = 12.0;

pub(crate) struct Mosaicos {
    n: usize,
    cols: usize,
    ancho: f32,
    alto: f32,
    icono: f32,
    por_pagina: usize,
}

impl Mosaicos {
    pub fn nuevos(n: usize, ancho_disponible: f32, alto_visible: f32, tamano_iconos: f32) -> Self {
        let icono = (tamano_iconos * 0.66).round().clamp(32.0, 80.0);
        let alto = (icono + 22.0).max(64.0);
        let ancho_min = 250.0 + icono * 0.5;
        let disponible = (ancho_disponible - MARGEN * 2.0).max(ancho_min.min(ancho_disponible - MARGEN * 2.0).max(120.0));
        let cols = ((disponible / ancho_min).floor() as usize).max(1);
        Self { n, cols, ancho: disponible / cols as f32, alto, icono, por_pagina: ((alto_visible / alto) as usize).max(1) }
    }
}

impl Disposicion for Mosaicos {
    fn tamano(&self) -> Vec2 {
        vec2(self.cols as f32 * self.ancho + MARGEN * 2.0, self.n.div_ceil(self.cols) as f32 * self.alto + MARGEN * 2.0)
    }

    fn rect(&self, k: usize) -> Rect {
        let (f, c) = (k / self.cols, k % self.cols);
        Rect::from_min_size(pos2(MARGEN + c as f32 * self.ancho, MARGEN + f as f32 * self.alto), vec2(self.ancho, self.alto))
    }

    fn rango(&self, visible: Rect) -> Range<usize> {
        let primera = ((visible.top() - MARGEN) / self.alto).floor().max(0.0) as usize;
        let ultima = ((visible.bottom() - MARGEN) / self.alto).ceil().max(0.0) as usize + 1;
        (primera * self.cols).min(self.n)..(ultima * self.cols).min(self.n)
    }

    fn vecino(&self, actual: Option<usize>, tecla: Key) -> Option<usize> {
        vecino_rejilla(actual, tecla, self.n, self.cols, self.por_pagina)
    }

    fn al_aparecer(&self, rect: Rect, t: f32) -> Rect {
        rect.translate(vec2(0.0, (1.0 - t) * 8.0))
    }

    fn pintar(&self, app: &Solar, painter: &Painter, e: &Entrada, rect: Rect, v: &Visual) {
        let p = &app.paleta;
        let caja = rect.shrink(4.0);
        let radio = CornerRadius::same(10);
        if v.seleccionado {
            painter.rect(caja, radio, p.seleccion, Stroke::new(1.0, p.seleccion_borde), StrokeKind::Inside);
        } else if v.soltar_aqui {
            painter.rect(caja, radio, con_alfa(p.acento, 0.18), Stroke::new(2.0, p.acento), StrokeKind::Inside);
        } else if v.hover > 0.0 {
            painter.rect_filled(caja, radio, con_alfa(p.hover, v.hover * p.hover.a() as f32 / 255.0));
        }
        let escala = 1.0 + 0.06 * v.hover - 0.05 * v.presion + 0.14 * v.salto;
        let lado = self.icono * escala;
        let icono = Rect::from_center_size(pos2(caja.left() + 10.0 + self.icono / 2.0, caja.center().y), vec2(lado, lado));
        app.pintar_icono(painter, icono, e, v.alfa);

        let x = caja.left() + 20.0 + self.icono;
        let ancho = (caja.right() - 10.0 - x).max(20.0);
        let color = if v.alfa < 1.0 { con_alfa(p.texto, 0.75) } else { p.texto };
        let nombre = texto_ajustado(painter, &e.nombre, ancho, 1, 13.5, color, false);
        let tipo = texto_ajustado(painter, &e.descripcion, ancho, 1, 12.0, p.texto_tenue, false);
        let detalle = match e.tipo {
            TipoEntrada::Archivo => format!("{} · {}", formato::tamano(e.tamano), formato::fecha(e.modificado)),
            _ => formato::fecha(e.modificado),
        };
        let detalle = texto_ajustado(painter, &detalle, ancho, 1, 12.0, p.texto_tenue, false);
        let alto = nombre.size().y + tipo.size().y + detalle.size().y + 4.0;
        let mut y = caja.center().y - alto / 2.0;
        let (h1, h2) = (nombre.size().y, tipo.size().y);
        painter.galley(pos2(x, y), nombre, color);
        y += h1 + 2.0;
        painter.galley(pos2(x, y), tipo, p.texto_tenue);
        y += h2 + 2.0;
        painter.galley(pos2(x, y), detalle, p.texto_tenue);
    }
}
