//! Cuadrícula: íconos grandes con el nombre abajo (el tamaño se elige en el
//! menú Vista o con Ctrl + rueda).

use super::{Disposicion, Visual, pintar_galley_centrado, texto_ajustado, vecino_rejilla};
use crate::app::Solar;
use crate::nucleo::Entrada;
use crate::tema::con_alfa;
use eframe::egui::{CornerRadius, Key, Painter, Rect, Stroke, StrokeKind, Vec2, pos2, vec2};
use std::ops::Range;

const MARGEN: f32 = 12.0;

pub(crate) struct Cuadricula {
    n: usize,
    cols: usize,
    ancho: f32,
    alto: f32,
    icono: f32,
    por_pagina: usize,
}

impl Cuadricula {
    pub fn nueva(n: usize, ancho_disponible: f32, alto_visible: f32, tamano_iconos: f32) -> Self {
        let icono = tamano_iconos.round();
        let ancho_min = (icono + 46.0).max(96.0);
        let alto = icono + 60.0;
        let disponible = (ancho_disponible - MARGEN * 2.0).max(ancho_min);
        let cols = ((disponible / ancho_min).floor() as usize).max(1);
        Self { n, cols, ancho: disponible / cols as f32, alto, icono, por_pagina: ((alto_visible / alto) as usize).max(1) }
    }
}

impl Disposicion for Cuadricula {
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

    fn pintar(&self, app: &Solar, painter: &Painter, e: &Entrada, rect: Rect, v: &Visual) {
        let p = &app.paleta;
        let caja = rect.shrink(5.0);
        let radio = CornerRadius::same(10);
        if v.seleccionado {
            painter.rect(caja, radio, p.seleccion, Stroke::new(1.0, p.seleccion_borde), StrokeKind::Inside);
        } else if v.soltar_aqui {
            painter.rect(caja, radio, con_alfa(p.acento, 0.18), Stroke::new(2.0, p.acento), StrokeKind::Inside);
        } else if v.hover > 0.0 {
            painter.rect_filled(caja, radio, con_alfa(p.hover, v.hover * p.hover.a() as f32 / 255.0));
        }
        // Se levanta con el mouse, se encoge al presionar y salta al personalizarla
        let escala = 1.0 + 0.05 * v.hover - 0.05 * v.presion + 0.14 * v.salto;
        let icono = self.icono;
        let rect_icono = Rect::from_center_size(
            pos2(rect.center().x, rect.top() + 10.0 + icono / 2.0 - 3.0 * v.hover),
            vec2(icono * escala, icono * escala),
        );
        app.pintar_icono(painter, rect_icono, e, v.alfa);
        let color = if v.alfa < 1.0 { con_alfa(p.texto, 0.75) } else { p.texto };
        let g = texto_ajustado(painter, &e.nombre, rect.width() - 14.0, 2, 13.0, color, true);
        pintar_galley_centrado(painter, rect.center().x, rect.top() + 16.0 + icono, g, color);
    }
}
