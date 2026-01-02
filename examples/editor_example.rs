use std::rc::Rc;

use omap::editor::OmapEditor;

fn main() {
    let editor = OmapEditor::from_path("./my_map.omap").unwrap();

    // get references to all the black colors
    let mut black_colors = Vec::new();
    for color_ref in editor.colors.iter().flatten() {
        if color_ref.get_cmyk().k > 0.99 {
            black_colors.push(color_ref);
        }
    }

    // get references to all symbols using one of the black colors
    let mut black_symbols = Vec::new();
    for symbol_rc in editor.symbols.iter_rc() {
        let symbol_ref = symbol_rc.borrow();
        if black_colors.iter().any(|bc| symbol_ref.contains_color(bc)) {
            black_symbols.push(symbol_rc);
        }
    }

    // get all objects for these symbols
    let mut black_objects = Vec::new();
    for symbol in black_symbols {
        black_objects.extend(
            editor
                .parts
                .get_map_part_by_index(0)
                .unwrap()
                .get(&Rc::downgrade(symbol)),
        );
    }

    println!("{:#?}", black_objects);
}
