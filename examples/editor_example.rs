use omap::editor::OmapEditor;

fn main() {
    let editor = OmapEditor::from_path("./my_map.omap").unwrap();

    println!("{:#?}", editor);
}
