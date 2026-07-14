// ─────────────────────────────────────────────────────────────────────────────
// services/bonsai.rs — generador procedural de árbol bonsái para el Evergrowth
// Basado en el algoritmo de PyBonsai: ramas recursivas con ruido gaussiano,
// hojas con gravedad, y un buffer de caracteres con color por celda.
// ─────────────────────────────────────────────────────────────────────────────

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const BRANCH_CHARS: [char; 4] = ['~', ';', ':', '='];
const LEAF_CHARS: [char; 4] = ['&', '%', '#', '@'];
const CHAR_HEIGHT: f64 = 2.0;
const ANGLE_MEAN: f64 = 0.698; // ~40 grados — dispersión lateral del árbol
const ANGLE_STD: f64 = 0.140;  // ~8 grados de ruido por rama
const LEN_SCALE: f64 = 0.75;   // cada capa de ramas es 75% más corta que la anterior
const MEAN_BRANCHES: f64 = 2.0;
const BRANCH_STD: f64 = 0.5;
const NUM_LEAVES: usize = 4;
const LEAF_LEN: usize = 4;

#[derive(Clone, Copy)]
struct Cell {
    ch: char,
    color: Color,
}

pub struct BonsaiGrid {
    rows: usize,
    cols: usize,
    buf: Vec<Vec<Option<Cell>>>,
}

// Distribución gaussiana usando la transformada de Box-Muller, acotada a ±3σ
fn gauss(mean: f64, std: f64, rng: &mut StdRng) -> f64 {
    let u1 = rng.gen_range(1e-10_f64..1.0);
    let u2 = rng.gen_range(0.0_f64..1.0);
    let z = (-2.0_f64 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
    (mean + std * z).clamp(mean - 3.0 * std, mean + 3.0 * std)
}

impl BonsaiGrid {
    fn new(rows: usize, cols: usize) -> Self {
        let buf = (0..rows)
            .map(|_| (0..cols).map(|_| None).collect())
            .collect();
        Self { rows, cols, buf }
    }

    fn set(&mut self, row: isize, col: isize, cell: Cell) {
        if row >= 0 && (row as usize) < self.rows && col >= 0 && (col as usize) < self.cols {
            self.buf[row as usize][col as usize] = Some(cell);
        }
    }

    // Convierte coordenadas de plano (x→, y↑) a coordenadas de pantalla (fila↓, col→)
    fn to_screen(&self, x: f64, y: f64) -> (isize, isize) {
        let row = self.rows as isize - 1 - (y / CHAR_HEIGHT).round() as isize;
        (row, x.round() as isize)
    }

    fn line_char(theta: f64) -> char {
        let a = theta.abs();
        let upper = std::f64::consts::FRAC_PI_2 * 2.0 / 3.0;
        let lower = std::f64::consts::FRAC_PI_2 / 3.0;
        if a > upper {
            '|'
        } else if a < lower {
            '_'
        } else if theta > 0.0 {
            '/'
        } else {
            '\\'
        }
    }

    fn branch_color(rng: &mut StdRng) -> Color {
        Color::Rgb(
            rng.gen_range(95_u8..=155),
            rng.gen_range(50_u8..=85),
            0,
        )
    }

    // El color de la hoja depende de la salud: a menor salud, mayor probabilidad de tono muerto
    fn leaf_color(health: i32, rng: &mut StdRng) -> Color {
        let die_prob = ((100 - health.clamp(0, 100)) as f64 / 100.0).powi(2) * 2.0;
        if rng.gen_range(0.0_f64..1.0) < die_prob.min(0.97) {
            Color::Rgb(
                rng.gen_range(120_u8..=190),
                rng.gen_range(45_u8..=95),
                0,
            )
        } else {
            Color::Rgb(
                rng.gen_range(0_u8..=25),
                rng.gen_range(130_u8..=220),
                rng.gen_range(0_u8..=20),
            )
        }
    }

    fn draw_segment(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        color: Color,
        width: usize,
        theta: f64,
        rng: &mut StdRng,
    ) {
        let (r0, c0) = self.to_screen(x0, y0);
        let (r1, c1) = self.to_screen(x1, y1);
        let base_ch = Self::line_char(theta);
        let dr = (r1 - r0).abs();
        let dc = (c1 - c0).abs();
        let half = (width as isize).saturating_sub(1) / 2;

        if dr >= dc {
            if r0 == r1 {
                return;
            }
            let (sr, er, sc, ec) = if r0 <= r1 {
                (r0, r1, c0, c1)
            } else {
                (r1, r0, c1, c0)
            };
            let span_f = (er - sr) as f64;
            for row in sr..=er {
                let t = (row - sr) as f64 / span_f;
                let mid = (sc as f64 + t * (ec - sc) as f64).round() as isize;
                for dw in -half..=half {
                    let ch = if rng.gen_range(0.0_f64..1.0) < 0.3 {
                        BRANCH_CHARS[rng.gen_range(0..BRANCH_CHARS.len())]
                    } else {
                        base_ch
                    };
                    self.set(row, mid + dw, Cell { ch, color });
                }
            }
        } else {
            if c0 == c1 {
                return;
            }
            let (sc, ec, sr, er) = if c0 <= c1 {
                (c0, c1, r0, r1)
            } else {
                (c1, c0, r1, r0)
            };
            let span_f = (ec - sc) as f64;
            for col in sc..=ec {
                let t = (col - sc) as f64 / span_f;
                let mid = (sr as f64 + t * (er - sr) as f64).round() as isize;
                for dw in -half..=half {
                    let ch = if rng.gen_range(0.0_f64..1.0) < 0.3 {
                        BRANCH_CHARS[rng.gen_range(0..BRANCH_CHARS.len())]
                    } else {
                        base_ch
                    };
                    self.set(mid + dw, col, Cell { ch, color });
                }
            }
        }
    }

    fn draw_leaves(&mut self, x: f64, y: f64, health: i32, rng: &mut StdRng) {
        for _ in 0..NUM_LEAVES {
            let vx = rng.gen_range(-1.0_f64..=1.0);
            let vy = rng.gen_range(-1.0_f64..=1.0);
            let len = (vx * vx + vy * vy).sqrt().max(1e-6);
            let (dvx, mut dvy) = (vx / len, vy / len);
            let (mut px, mut py) = (x, y);
            for i in 0..LEAF_LEN {
                px += dvx;
                py += dvy;
                let ch = LEAF_CHARS[rng.gen_range(0..LEAF_CHARS.len())];
                let color = Self::leaf_color(health, rng);
                let (row, col) = self.to_screen(px, py);
                self.set(row, col, Cell { ch, color });
                dvy -= i as f64 / LEAF_LEN as f64;
            }
        }
    }

    fn draw_branch(
        &mut self,
        x: f64,
        y: f64,
        layer: u32,
        num_layers: u32,
        length: f64,
        width: usize,
        theta: f64,
        health: i32,
        rng: &mut StdRng,
    ) {
        if layer >= num_layers {
            self.draw_leaves(x, y, health, rng);
            return;
        }
        let end_x = x + length * theta.sin();
        let end_y = y + length * theta.cos();
        let color = Self::branch_color(rng);
        self.draw_segment(x, y, end_x, end_y, color, width, theta, rng);

        let n = gauss(MEAN_BRANCHES, BRANCH_STD, rng)
            .round()
            .max(0.0)
            .min(3.0) as usize;
        let step = length / n.max(1) as f64;
        let mut sign = 1.0_f64;
        for i in 0..n {
            let dist = (i + 1) as f64 * step;
            let bx = x + dist * theta.sin();
            let by = y + dist * theta.cos();
            let new_theta = theta + sign * gauss(ANGLE_MEAN, ANGLE_STD, rng);
            self.draw_branch(
                bx,
                by,
                layer + 1,
                num_layers,
                length * LEN_SCALE,
                width.saturating_sub(1).max(1),
                new_theta,
                health,
                rng,
            );
            sign = -sign;
        }
    }

    /// Genera la cuadrícula del bonsái para el área disponible, etapa y salud del árbol
    pub fn generate(rows: usize, cols: usize, seed: u64, stage: i32, health: i32) -> Self {
        let mut grid = Self::new(rows, cols);
        if rows < 3 || cols < 4 {
            return grid;
        }
        let mut rng = StdRng::seed_from_u64(seed);

        let root_x = cols as f64 / 2.0;
        let root_y = 1.5; // el tronco arranca justo encima de la última fila

        let (num_layers, base_len) = match stage {
            1 => (2_u32, 3.0_f64),
            2 => (3, 5.0),
            3 => (4, 7.5),
            4 => (5, 9.5),
            5 => (6, 11.5),
            6 => (7, 13.5),
            _ => (8, 16.0),
        };

        // Escala la longitud para que el árbol quepa en el área sin exceder bordes
        let max_by_height = rows as f64 * CHAR_HEIGHT * 0.72;
        let max_by_width = cols as f64 * 0.38;
        let len = base_len.min(max_by_height).min(max_by_width);
        let width = (len / 5.0).round().max(1.0) as usize;
        let initial_theta = gauss(0.0, 0.07, &mut rng);

        grid.draw_branch(
            root_x,
            root_y,
            1,
            num_layers,
            len,
            width,
            initial_theta,
            health,
            &mut rng,
        );
        grid
    }

    /// Convierte la cuadrícula en líneas de ratatui, agrupando celdas del mismo color en un Span
    pub fn into_lines(self) -> Vec<Line<'static>> {
        self.buf
            .into_iter()
            .map(|row| {
                let mut spans: Vec<Span<'static>> = Vec::new();
                let mut i = 0;
                while i < row.len() {
                    if row[i].is_none() {
                        let start = i;
                        while i < row.len() && row[i].is_none() {
                            i += 1;
                        }
                        spans.push(Span::raw(" ".repeat(i - start)));
                    } else {
                        let color = row[i].as_ref().unwrap().color;
                        let mut s = String::new();
                        while i < row.len() {
                            match row[i] {
                                Some(c) if c.color == color => {
                                    s.push(c.ch);
                                    i += 1;
                                }
                                _ => break,
                            }
                        }
                        spans.push(Span::styled(s, Style::default().fg(color)));
                    }
                }
                Line::from(spans)
            })
            .collect()
    }
}
