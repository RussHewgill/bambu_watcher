pub fn _load_icon<P: AsRef<std::path::Path>>(
    path: P,
) -> (image::ImageBuffer<image::Rgba<u8>, Vec<u8>>, u32, u32) {
    let image = image::open(path)
        .expect("Failed to open icon path")
        .into_rgba8();
    let (width, height) = image.dimensions();

    (image, width, height)
}

pub fn load_icon<P: AsRef<std::path::Path>>(path: P) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}
