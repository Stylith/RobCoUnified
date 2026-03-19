include!(concat!(env!("OUT_DIR"), "/builtin_icons.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_icon_lookup_returns_generated_png_bytes() {
        for size in BUILTIN_ICON_SIZES {
            assert!(builtin_icon("pixel--cog-solid", size).is_some());
        }
        assert!(BUILTIN_ICON_NAMES.contains(&"pixel--folder-solid"));
        assert!(builtin_icon("pixel--does-not-exist", 32).is_none());
        assert!(builtin_icon("pixel--cog-solid", 12).is_none());
    }
}
