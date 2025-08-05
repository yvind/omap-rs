use omap::editor::OmapEditor;

fn main() {
    let editor = OmapEditor::from_path("../test.omap").unwrap();

    println!("{:#?}", editor);
}
