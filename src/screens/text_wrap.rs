// ─────────────────────────────────────────────────────────────────────────────
// text_wrap.rs — envoltura de texto con seguimiento del cursor
// ─────────────────────────────────────────────────────────────────────────────
//
// Los campos de descripción se dibujan como una lista de líneas, así que una línea
// más ancha que su caja se recortaba y el texto desaparecía a la derecha mientras
// se seguía escribiendo. Aquí se parte el texto en filas visuales y se traduce el
// cursor a (fila, columna) sobre esas filas, que es lo que permite hacer scroll y
// mantener el cursor a la vista.

/// Filas visuales de un texto y la posición del cursor sobre ellas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedText {
    /// Rangos de bytes `[inicio, fin)` sobre el texto original, uno por fila visual.
    /// Cubren cada byte del texto exactamente una vez, sin los saltos de línea.
    pub rows: Vec<(usize, usize)>,
    /// Fila visual donde está el cursor.
    pub cursor_row: usize,
    /// Columna del cursor dentro de su fila, contada en caracteres.
    pub cursor_col: usize,
}

/// Parte `text` en filas de `width` columnas y ubica `cursor` (offset en bytes).
///
/// Corta de preferencia después de un espacio; una palabra más larga que el ancho se
/// parte para que nunca se salga de la caja. El espacio del corte se queda al final de
/// su fila, así cada byte pertenece a una sola fila y el cursor se ubica sin ambigüedad.
pub fn wrap_with_cursor(text: &str, cursor: usize, width: usize) -> WrappedText {
    let width = width.max(1);
    let cursor = clamp_to_char_boundary(text, cursor);
    let mut rows: Vec<(usize, usize)> = Vec::new();

    let mut line_start = 0usize;
    loop {
        let line_end = text[line_start..]
            .find('\n')
            .map(|offset| line_start + offset)
            .unwrap_or(text.len());
        wrap_single_line(text, line_start, line_end, width, &mut rows);
        if line_end == text.len() {
            break;
        }
        line_start = line_end + 1; // saltar el '\n'
    }
    if rows.is_empty() {
        rows.push((0, 0));
    }

    // El cursor cae en la fila que lo contiene; al final del texto o de una línea
    // lógica se queda en la última fila de esa línea, no al inicio de la siguiente.
    let mut cursor_row = rows.len() - 1;
    for (index, &(_, end)) in rows.iter().enumerate() {
        if cursor < end || (cursor == end && is_row_end_of_line(text, end)) {
            cursor_row = index;
            break;
        }
        if cursor == end {
            // Frontera dentro de una línea envuelta: el cursor se ve al inicio de la
            // siguiente fila, que es donde el siguiente carácter se va a escribir.
            cursor_row = (index + 1).min(rows.len() - 1);
            break;
        }
    }
    let (row_start, _) = rows[cursor_row];
    let mut cursor_col = text[row_start..cursor.max(row_start)].chars().count();

    // Un cursor apoyado justo en el borde derecho no cabe en la caja. Como en cualquier
    // editor, se muestra al inicio de la fila siguiente; si no hay ninguna se abre una
    // fila vacía para él, que es lo que hace visible el texto recién escrito.
    if cursor_col >= width {
        if cursor_row + 1 < rows.len() {
            cursor_row += 1;
        } else {
            rows.push((cursor, cursor));
            cursor_row = rows.len() - 1;
        }
        cursor_col = 0;
    }

    WrappedText {
        rows,
        cursor_row,
        cursor_col,
    }
}

/// Una fila termina una línea lógica si le sigue un `\n` o el fin del texto.
fn is_row_end_of_line(text: &str, end: usize) -> bool {
    end == text.len() || text.as_bytes().get(end) == Some(&b'\n')
}

fn wrap_single_line(
    text: &str,
    start: usize,
    end: usize,
    width: usize,
    rows: &mut Vec<(usize, usize)>,
) {
    if start == end {
        rows.push((start, start));
        return;
    }
    let mut row_start = start;
    while row_start < end {
        let line = &text[row_start..end];
        let mut chars = line.char_indices();
        let mut taken = 0usize;
        let mut row_end: Option<usize> = None;
        let mut last_space_end: Option<usize> = None;
        for (offset, character) in chars.by_ref() {
            if taken == width {
                row_end = Some(row_start + offset);
                break;
            }
            taken += 1;
            if character == ' ' {
                last_space_end = Some(row_start + offset + character.len_utf8());
            }
        }
        let mut row_end = match row_end {
            // Todo lo que queda cabe en esta fila.
            None => end,
            Some(hard_break) => {
                // Preferir el último espacio de la fila, si deja algo dentro de ella.
                match last_space_end {
                    Some(space_end) if space_end > row_start && space_end <= hard_break => space_end,
                    _ => hard_break,
                }
            }
        };
        // Los espacios del corte se quedan al final de esta fila. Si empezaran la
        // siguiente se verían como una sangría que el usuario nunca escribió, y el
        // recorte de la derecha los oculta sin costo.
        while row_end < end && text.as_bytes()[row_end] == b' ' {
            row_end += 1;
        }
        rows.push((row_start, row_end));
        row_start = row_end;
    }
}

/// Cuántas filas visuales ocupa una línea lógica a `width` columnas.
pub fn visual_row_count(line: &str, width: usize) -> usize {
    let width = width.max(1);
    let mut rows = Vec::new();
    wrap_single_line(line, 0, line.len(), width, &mut rows);
    rows.len().max(1)
}

/// Parte una línea ya estilizada en filas de `width` celdas conservando los estilos.
///
/// El editor de descripción produce spans con resaltados de cursor y selección, así que
/// no se puede envolver su texto crudo: hay que cortar los spans. Se prefiere cortar
/// después de un espacio y se parte cualquier palabra más larga que la caja.
pub fn split_styled_line(line: &ratatui::text::Line<'_>, width: usize) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::text::{Line, Span};

    let width = width.max(1);
    // Se aplana a (carácter, estilo) para poder cortar en cualquier punto sin perder
    // el estilo, y luego se vuelven a unir los caracteres contiguos del mismo estilo.
    let cells: Vec<(char, ratatui::style::Style)> = line
        .spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |character| (character, span.style)))
        .collect();
    if cells.is_empty() {
        return vec![Line::from(String::new())];
    }

    let mut rows: Vec<Line<'static>> = Vec::new();
    let mut start = 0usize;
    while start < cells.len() {
        let hard_end = (start + width).min(cells.len());
        let mut end = hard_end;
        if hard_end < cells.len() {
            // Último espacio dentro de la fila: se corta justo después de él.
            if let Some(offset) = cells[start..hard_end]
                .iter()
                .rposition(|(character, _)| *character == ' ')
            {
                end = start + offset + 1;
            }
        }
        // Los espacios del corte se quedan en esta fila para no sangrar la siguiente.
        while end < cells.len() && cells[end].0 == ' ' {
            end += 1;
        }
        let mut spans: Vec<Span<'static>> = Vec::new();
        for &(character, style) in &cells[start..end] {
            match spans.last_mut() {
                Some(last) if last.style == style => last.content.to_mut().push(character),
                _ => spans.push(Span::styled(character.to_string(), style)),
            }
        }
        rows.push(Line::from(spans));
        start = end;
    }
    rows
}

/// Evita cortar un carácter multibyte por la mitad al indexar con el cursor.
fn clamp_to_char_boundary(text: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(text.len());
    while cursor > 0 && !text.is_char_boundary(cursor) {
        cursor -= 1;
    }
    cursor
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_as_text<'a>(text: &'a str, wrapped: &WrappedText) -> Vec<&'a str> {
        wrapped
            .rows
            .iter()
            .map(|&(start, end)| &text[start..end])
            .collect()
    }

    #[test]
    fn short_text_stays_on_one_row() {
        let text = "hola";
        let wrapped = wrap_with_cursor(text, 4, 20);
        assert_eq!(rows_as_text(text, &wrapped), vec!["hola"]);
        assert_eq!((wrapped.cursor_row, wrapped.cursor_col), (0, 4));
    }

    #[test]
    fn empty_text_still_has_a_row_for_the_cursor() {
        let wrapped = wrap_with_cursor("", 0, 10);
        assert_eq!(wrapped.rows, vec![(0, 0)]);
        assert_eq!((wrapped.cursor_row, wrapped.cursor_col), (0, 0));
    }

    #[test]
    fn wraps_on_word_boundaries_and_never_exceeds_the_width() {
        let text = "the notification swarm approaches the realm";
        let wrapped = wrap_with_cursor(text, 0, 12);
        // Solo los espacios del corte pueden pasar del borde, y el recorte los oculta.
        for row in rows_as_text(text, &wrapped) {
            assert!(
                row.trim_end().chars().count() <= 12,
                "row {row:?} is wider than the box"
            );
        }
        // Ninguna palabra queda partida y ninguna fila arranca con un espacio sobrante.
        let rows = rows_as_text(text, &wrapped);
        assert!(
            !rows.iter().any(|row| row.starts_with(' ')),
            "a wrapped row must not begin with a leftover space: {rows:?}"
        );
        assert_eq!(
            rows.iter().map(|row| row.trim_end()).collect::<Vec<_>>(),
            vec!["the", "notification", "swarm", "approaches", "the realm"]
        );
    }

    #[test]
    fn a_word_longer_than_the_box_is_split_instead_of_clipped() {
        let text = "supercalifragilistic";
        let wrapped = wrap_with_cursor(text, 0, 6);
        assert_eq!(
            rows_as_text(text, &wrapped),
            vec!["superc", "alifra", "gilist", "ic"]
        );
    }

    #[test]
    fn every_byte_belongs_to_exactly_one_row() {
        let text = "primera linea larga que se envuelve\nsegunda\n\ncuarta";
        let wrapped = wrap_with_cursor(text, 0, 9);
        let rebuilt: String = rows_as_text(text, &wrapped).concat();
        assert_eq!(rebuilt, text.replace('\n', ""));
    }

    #[test]
    fn explicit_newlines_start_new_rows_and_blank_lines_survive() {
        let text = "uno\n\ndos";
        let wrapped = wrap_with_cursor(text, 0, 10);
        assert_eq!(rows_as_text(text, &wrapped), vec!["uno", "", "dos"]);
    }

    #[test]
    fn the_cursor_follows_the_text_past_the_right_edge() {
        // Es el bug reportado: al escribir más allá del ancho, el cursor tiene que
        // bajar a la siguiente fila en vez de irse fuera de la caja.
        let text = "abcdefghij";
        let width = 5;
        let wrapped = wrap_with_cursor(text, text.len(), width);
        // El texto llena dos filas exactas, así que el cursor abre una tercera: es lo
        // que antes se salía de la caja y hacía "desaparecer" lo que se escribía.
        assert_eq!(rows_as_text(text, &wrapped), vec!["abcde", "fghij", ""]);
        assert_eq!((wrapped.cursor_row, wrapped.cursor_col), (2, 0));
        // Y en cada posición intermedia la columna nunca se sale del ancho.
        for cursor in 0..=text.len() {
            let step = wrap_with_cursor(text, cursor, width);
            assert!(
                step.cursor_col <= width,
                "cursor {cursor} landed at column {} beyond width {width}",
                step.cursor_col
            );
            assert!(step.cursor_row < step.rows.len());
        }
    }

    #[test]
    fn the_cursor_at_the_end_of_a_logical_line_stays_on_that_line() {
        let text = "uno\ndos";
        let wrapped = wrap_with_cursor(text, 3, 10);
        assert_eq!((wrapped.cursor_row, wrapped.cursor_col), (0, 3));
    }

    #[test]
    fn multibyte_text_is_measured_in_characters_not_bytes() {
        let text = "áéíóú añadir";
        let wrapped = wrap_with_cursor(text, text.len(), 6);
        for row in rows_as_text(text, &wrapped) {
            assert!(row.chars().count() <= 6, "row {row:?} too wide");
        }
        // Un cursor a media secuencia UTF-8 no debe provocar un panic de índice.
        for cursor in 0..=text.len() + 3 {
            let _ = wrap_with_cursor(text, cursor, 6);
        }
    }

    #[test]
    fn visual_row_count_matches_the_wrapped_rows() {
        assert_eq!(visual_row_count("", 10), 1);
        assert_eq!(visual_row_count("corto", 10), 1);
        assert_eq!(visual_row_count("abcdefghij", 5), 2);
        assert_eq!(visual_row_count("uno dos tres cuatro", 8), 3);
    }

    #[test]
    fn splitting_a_styled_line_keeps_every_character_and_its_style() {
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};

        let cursor_style = Style::default().bg(Color::Green);
        let line = Line::from(vec![
            Span::raw("hola mundo "),
            Span::styled("X", cursor_style),
            Span::raw(" resto del texto"),
        ]);
        let rows = split_styled_line(&line, 8);

        // Nada se pierde ni se reordena al cortar.
        let rebuilt: String = rows
            .iter()
            .flat_map(|row| row.spans.iter().map(|span| span.content.to_string()))
            .collect();
        assert_eq!(rebuilt, "hola mundo X resto del texto");

        // Ninguna fila excede el ancho, salvo los espacios de corte que se recortan.
        for row in &rows {
            let visible: String = row.spans.iter().map(|span| span.content.to_string()).collect();
            assert!(
                visible.trim_end().chars().count() <= 8,
                "row {visible:?} is wider than the box"
            );
        }

        // El resaltado del cursor sobrevive el corte con su estilo intacto.
        let highlighted: Vec<_> = rows
            .iter()
            .flat_map(|row| row.spans.iter())
            .filter(|span| span.style == cursor_style)
            .map(|span| span.content.to_string())
            .collect();
        assert_eq!(highlighted, vec!["X".to_string()]);
    }

    #[test]
    fn splitting_an_empty_styled_line_still_yields_one_row() {
        let rows = split_styled_line(&ratatui::text::Line::from(""), 10);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn a_styled_word_longer_than_the_box_is_split_across_rows() {
        use ratatui::text::{Line, Span};
        let line = Line::from(vec![Span::raw("supercalifragilistic")]);
        let rows = split_styled_line(&line, 6);
        let texts: Vec<String> = rows
            .iter()
            .map(|row| row.spans.iter().map(|span| span.content.to_string()).collect())
            .collect();
        assert_eq!(texts, vec!["superc", "alifra", "gilist", "ic"]);
    }

    #[test]
    fn width_zero_is_treated_as_one_column() {
        let wrapped = wrap_with_cursor("ab", 2, 0);
        // Una columna: "a", "b", y la fila que abre el cursor al final.
        assert_eq!(wrapped.rows.len(), 3);
        assert_eq!((wrapped.cursor_row, wrapped.cursor_col), (2, 0));
    }
}
