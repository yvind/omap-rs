use omap::editor::OmapEditor;

fn main() {
    let editor = OmapEditor::from_path("./my_map.omap").unwrap();

    // get references to all the black colors
    let mut black_colors = Vec::new();
    println!("Black colors:");
    for color_ref in editor.colors.iter().flatten() {
        if color_ref.get_cmyk().k > 0.99 {
            println!("color {:2}: {:?}", color_ref.get_id(), color_ref.get_name());
            black_colors.push(color_ref);
        }
    }
    println!(
        "Found {} black colors (of {} total colors)",
        black_colors.len(),
        editor.colors.num_colors()
    );

    // get weak references to all symbols using at least one of the black colors
    let mut black_symbols = Vec::new();
    println!("\nBlack symbols:");
    for symbol_rc in editor.symbols.iter_rc() {
        let symbol_ref = symbol_rc.borrow();
        if black_colors.iter().any(|bc| symbol_ref.contains_color(bc)) {
            println!(
                "symbol {:3}: {} {:?}",
                symbol_ref.get_id(),
                symbol_ref.get_code(),
                symbol_ref.get_name(),
            );
            black_symbols.push(std::rc::Rc::downgrade(symbol_rc));
        }
    }
    println!(
        "Found {} black symbols (of {} total symbols)",
        black_symbols.len(),
        editor.symbols.num_symbols()
    );

    // get all objects for these symbols
    let mut black_objects = Vec::new();
    for symbol in black_symbols {
        black_objects.extend(
            editor
                .parts
                .get_map_part_by_index(0)
                .and_then(|mp| mp.get(&symbol)),
        );
    }

    println!("\nBlack objects:");
    println!("{:#?}", black_objects);
}
