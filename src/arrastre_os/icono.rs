//! El dibujito que acompaña al puntero mientras arrastras archivos fuera de
//! Solar (en Wayland el compositor lo pinta; X11 no tiene algo así).

use resvg::tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

/// Tamaño en puntos; se dibuja al doble para pantallas HiDPI.
pub const LADO: u32 = 78;
/// El dibujo está pensado en 64×64; se agranda un poco
const AUMENTO: f32 = 1.2;
pub const ESCALA: u32 = 2;

#[derive(Clone, Copy)]
pub struct Icono {
    pub cuantos: usize,
    pub carpeta: bool,
    pub color: [u8; 3],
}

/// Píxeles en ARGB8888 (lo que pide wl_shm), premultiplicados.
pub fn dibujar(ic: Icono) -> Vec<u8> {
    let lado = LADO * ESCALA;
    let Some(mut px) = Pixmap::new(lado, lado) else { return vec![0; (lado * lado * 4) as usize] };
    let t = Transform::from_scale(ESCALA as f32 * AUMENTO, ESCALA as f32 * AUMENTO);
    let [r, g, b] = ic.color;
    if ic.cuantos > 1 {
        // Otro detrás, para que se note que son varios
        figura(&mut px, t.pre_translate(6.0, -5.0), ic.carpeta, (r, g, b), 0.45);
    }
    figura(&mut px, t, ic.carpeta, (r, g, b), 1.0);
    if ic.cuantos > 1 {
        insignia(&mut px, t, ic.cuantos);
    }
    // RGBA -> BGRA
    let mut v = px.take();
    for p in v.chunks_exact_mut(4) {
        p.swap(0, 2);
    }
    v
}

fn pintura(r: u8, g: u8, b: u8, a: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(r, g, b, (a * 255.0) as u8));
    p.anti_alias = true;
    p
}

fn aclarar((r, g, b): (u8, u8, u8), f: f32) -> (u8, u8, u8) {
    let m = |c: u8| (c as f32 + (255.0 - c as f32) * f) as u8;
    (m(r), m(g), m(b))
}

fn oscurecer((r, g, b): (u8, u8, u8), f: f32) -> (u8, u8, u8) {
    let m = |c: u8| (c as f32 * (1.0 - f)) as u8;
    (m(r), m(g), m(b))
}

fn figura(px: &mut Pixmap, t: Transform, carpeta: bool, c: (u8, u8, u8), alfa: f32) {
    if carpeta {
        let mut pb = PathBuilder::new();
        pb.move_to(12.0, 16.0);
        pb.quad_to(12.0, 12.0, 16.0, 12.0);
        pb.line_to(26.0, 12.0);
        pb.line_to(30.0, 16.0);
        pb.line_to(48.0, 16.0);
        pb.quad_to(52.0, 16.0, 52.0, 20.0);
        pb.line_to(52.0, 46.0);
        pb.quad_to(52.0, 50.0, 48.0, 50.0);
        pb.line_to(16.0, 50.0);
        pb.quad_to(12.0, 50.0, 12.0, 46.0);
        pb.close();
        let (r, g, b) = oscurecer(c, 0.18);
        if let Some(p) = pb.finish() {
            px.fill_path(&p, &pintura(r, g, b, alfa), FillRule::Winding, t, None);
        }
        let (r, g, b) = aclarar(c, 0.15);
        if let Some(p) = redondeado(12.0, 22.0, 52.0, 50.0, 4.0) {
            px.fill_path(&p, &pintura(r, g, b, alfa), FillRule::Winding, t, None);
        }
    } else {
        let mut pb = PathBuilder::new();
        pb.move_to(14.0, 14.0);
        pb.quad_to(14.0, 10.0, 18.0, 10.0);
        pb.line_to(36.0, 10.0);
        pb.line_to(46.0, 20.0);
        pb.line_to(46.0, 48.0);
        pb.quad_to(46.0, 52.0, 42.0, 52.0);
        pb.line_to(18.0, 52.0);
        pb.quad_to(14.0, 52.0, 14.0, 48.0);
        pb.close();
        if let Some(p) = pb.finish() {
            px.fill_path(&p, &pintura(c.0, c.1, c.2, alfa), FillRule::Winding, t, None);
        }
        // La esquina doblada
        let mut pb = PathBuilder::new();
        pb.move_to(36.0, 10.0);
        pb.line_to(36.0, 17.0);
        pb.quad_to(36.0, 20.0, 39.0, 20.0);
        pb.line_to(46.0, 20.0);
        pb.close();
        if let Some(p) = pb.finish() {
            px.fill_path(&p, &pintura(255, 255, 255, 0.45 * alfa), FillRule::Winding, t, None);
        }
    }
}

fn redondeado(l: f32, a: f32, d: f32, b: f32, r: f32) -> Option<resvg::tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(l + r, a);
    pb.line_to(d - r, a);
    pb.quad_to(d, a, d, a + r);
    pb.line_to(d, b - r);
    pb.quad_to(d, b, d - r, b);
    pb.line_to(l + r, b);
    pb.quad_to(l, b, l, b - r);
    pb.line_to(l, a + r);
    pb.quad_to(l, a, l + r, a);
    pb.close();
    pb.finish()
}

/// Números de 3×5 «píxeles», para no cargar una fuente solo por esto.
const DIGITOS: [[u8; 5]; 11] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
    [0b000, 0b010, 0b111, 0b010, 0b000], // +
];

fn insignia(px: &mut Pixmap, t: Transform, n: usize) {
    let texto: Vec<usize> = if n > 99 {
        vec![9, 9, 10]
    } else {
        n.to_string().bytes().map(|b| (b - b'0') as usize).collect()
    };
    let celda = 1.5;
    let ancho_dig = 3.0 * celda;
    let ancho = texto.len() as f32 * ancho_dig + (texto.len() as f32 - 1.0) * celda;
    let (cx, cy) = (50.0, 48.0);
    let radio = (ancho / 2.0 + 4.0).max(9.0);
    if let Some(p) = PathBuilder::from_circle(cx, cy, radio + 1.5) {
        px.fill_path(&p, &pintura(255, 255, 255, 1.0), FillRule::Winding, t, None);
    }
    if let Some(p) = PathBuilder::from_circle(cx, cy, radio) {
        px.fill_path(&p, &pintura(32, 30, 28, 1.0), FillRule::Winding, t, None);
    }
    let blanco = pintura(255, 255, 255, 1.0);
    let x0 = cx - ancho / 2.0;
    let y0 = cy - 2.5 * celda;
    for (i, &d) in texto.iter().enumerate() {
        let xd = x0 + i as f32 * (ancho_dig + celda);
        for (fila, bits) in DIGITOS[d].iter().enumerate() {
            for col in 0..3 {
                if bits & (0b100 >> col) != 0
                    && let Some(r) = Rect::from_xywh(xd + col as f32 * celda, y0 + fila as f32 * celda, celda, celda)
                {
                    px.fill_rect(r, &blanco, t, None);
                }
            }
        }
    }
}
