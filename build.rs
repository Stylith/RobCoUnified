use std::fs;
use std::path::{Path, PathBuf};

fn render_svg_to_png(svg_path: &Path, png_path: &Path) {
    let bytes = fs::read(svg_path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", svg_path.display()));
    let tree = usvg::Tree::from_data(&bytes, &usvg::Options::default())
        .unwrap_or_else(|err| panic!("failed parsing {}: {err}", svg_path.display()));
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .unwrap_or_else(|| panic!("failed allocating pixmap for {}", svg_path.display()));
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    let mut out = image::RgbaImage::new(size.width(), size.height());
    for (idx, pixel) in pixmap.pixels().iter().enumerate() {
        let x = (idx as u32) % size.width();
        let y = (idx as u32) / size.width();
        let r = pixel.red();
        let g = pixel.green();
        let b = pixel.blue();
        let a = pixel.alpha();
        // Treat white template backgrounds as transparent so monochrome tinting works.
        let alpha = if a > 0 && r >= 250 && g >= 250 && b >= 250 {
            0
        } else {
            a
        };
        out.put_pixel(x, y, image::Rgba([r, g, b, alpha]));
    }
    out.save(png_path)
        .unwrap_or_else(|err| panic!("failed writing {}: {err}", png_path.display()));
}

fn build_donkey_kong_assets() {
    let asset_root = PathBuf::from("assets/donkey_kong/svg");
    if !asset_root.exists() {
        return;
    }

    let out_dir =
        PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR missing")).join("donkey_kong");
    fs::create_dir_all(&out_dir)
        .unwrap_or_else(|err| panic!("failed creating {}: {err}", out_dir.display()));

    for sheet in [
        "mario_sheet",
        "barrels_sheet",
        "level_sheet",
        "ui_sheet",
        "effects_sheet",
    ] {
        let svg_path = asset_root.join(format!("{sheet}.svg"));
        let png_path = out_dir.join(format!("{sheet}.png"));
        println!("cargo:rerun-if-changed={}", svg_path.display());
        render_svg_to_png(&svg_path, &png_path);
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    build_donkey_kong_assets();

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(err) = res.compile() {
            panic!("failed to embed Windows icon: {err}");
        }
    }
}
