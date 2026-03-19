use std::collections::HashSet;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

fn render_svg_to_png(svg_path: &Path, png_path: &Path, target_size: Option<u32>) {
    let bytes = fs::read(svg_path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", svg_path.display()));
    let tree = usvg::Tree::from_data(&bytes, &usvg::Options::default())
        .unwrap_or_else(|err| panic!("failed parsing {}: {err}", svg_path.display()));
    let natural = tree.size().to_int_size();
    let largest_edge = natural.width().max(natural.height()) as f32;
    let (width, height, transform) = if let Some(target_size) = target_size {
        let scale = target_size as f32 / largest_edge;
        let width = (natural.width() as f32 * scale).round().max(1.0) as u32;
        let height = (natural.height() as f32 * scale).round().max(1.0) as u32;
        (
            width,
            height,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
        )
    } else {
        (
            natural.width(),
            natural.height(),
            resvg::tiny_skia::Transform::identity(),
        )
    };
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .unwrap_or_else(|| panic!("failed allocating pixmap for {}", svg_path.display()));
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let mut out = image::RgbaImage::new(width, height);
    for (idx, pixel) in pixmap.pixels().iter().enumerate() {
        let x = (idx as u32) % width;
        let y = (idx as u32) / width;
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
        render_svg_to_png(&svg_path, &png_path, None);
    }
}

#[derive(Debug)]
struct BuiltinIconSource {
    name: String,
    output_stem: String,
    path: PathBuf,
}

fn sanitize_icon_stem(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_was_underscore = false;
    for ch in name.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '_'
        };
        if mapped == '_' {
            if !last_was_underscore {
                out.push(mapped);
            }
            last_was_underscore = true;
        } else {
            out.push(mapped);
            last_was_underscore = false;
        }
    }
    let trimmed = out.trim_matches('_');
    let mut sanitized = if trimmed.is_empty() {
        "icon".to_string()
    } else {
        trimmed.to_string()
    };
    if sanitized
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit())
    {
        sanitized.insert_str(0, "icon_");
    }
    sanitized
}

fn collect_builtin_icon_sources(icon_root: &Path) -> Vec<BuiltinIconSource> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(icon_root)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", icon_root.display()))
    {
        let entry =
            entry.unwrap_or_else(|err| panic!("failed reading {} entry: {err}", icon_root.display()));
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("svg") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_else(|| panic!("non-utf8 svg file name: {}", path.display()))
            .to_string();
        entries.push(BuiltinIconSource {
            output_stem: sanitize_icon_stem(&name),
            name,
            path,
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));

    let mut seen = HashSet::new();
    for entry in &entries {
        if !seen.insert(entry.output_stem.clone()) {
            panic!(
                "duplicate sanitized icon name `{}` in {}",
                entry.output_stem,
                icon_root.display()
            );
        }
    }

    entries
}

fn build_builtin_icon_assets() {
    const ICON_SIZES: [u16; 5] = [16, 24, 32, 48, 64];

    let icon_root = PathBuf::from("src/Icons");
    if !icon_root.exists() {
        return;
    }

    let icons = collect_builtin_icon_sources(&icon_root);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR missing"));
    let png_out_dir = out_dir.join("builtin_icons");
    fs::create_dir_all(&png_out_dir)
        .unwrap_or_else(|err| panic!("failed creating {}: {err}", png_out_dir.display()));

    let mut generated = String::new();
    generated.push_str("// @generated by build.rs\n");
    generated.push_str("pub const BUILTIN_ICON_SIZES: [u16; 5] = [16, 24, 32, 48, 64];\n");
    generated.push_str("pub const BUILTIN_ICON_NAMES: &[&str] = &[\n");
    for icon in &icons {
        writeln!(&mut generated, "    {:?},", icon.name).expect("write icon name");
    }
    generated.push_str("];\n\n");
    generated.push_str("pub fn builtin_icon(name: &str, size: u16) -> Option<&'static [u8]> {\n");
    generated.push_str("    match (name, size) {\n");

    for icon in &icons {
        println!("cargo:rerun-if-changed={}", icon.path.display());
        for size in ICON_SIZES {
            let png_name = format!("{}_{}.png", icon.output_stem, size);
            let png_path = png_out_dir.join(&png_name);
            render_svg_to_png(&icon.path, &png_path, Some(size.into()));
            writeln!(
                &mut generated,
                "        ({:?}, {}) => Some(include_bytes!(concat!(env!(\"OUT_DIR\"), \"/builtin_icons/{}\"))),",
                icon.name,
                size,
                png_name
            )
            .expect("write icon arm");
        }
    }

    generated.push_str("        _ => None,\n");
    generated.push_str("    }\n");
    generated.push_str("}\n");

    let generated_rs = out_dir.join("builtin_icons.rs");
    fs::write(&generated_rs, generated)
        .unwrap_or_else(|err| panic!("failed writing {}: {err}", generated_rs.display()));
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    build_donkey_kong_assets();
    build_builtin_icon_assets();

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(err) = res.compile() {
            panic!("failed to embed Windows icon: {err}");
        }
    }
}
