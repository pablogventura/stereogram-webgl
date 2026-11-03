use std::fs;
use std::path::{Path, PathBuf};

use image::RgbaImage;

use crate::mesh::Mesh;

pub struct Catalog {
    root: PathBuf,
}

impl Catalog {
    pub fn discover() -> Result<Self, String> {
        let candidates = [
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/static/resources"),
            PathBuf::from("src/static/resources"),
            PathBuf::from("../src/static/resources"),
        ];
        let root = candidates
            .into_iter()
            .find(|path| path.join("heightmaps").is_dir() && path.join("tiles").is_dir())
            .ok_or_else(|| "No encuentro las texturas en src/static/resources.".to_string())?;
        Ok(Self { root })
    }

    pub fn load_image(&self, folder: &str, file_name: &str) -> Result<RgbaImage, String> {
        load_image_path(&self.root.join(folder).join(file_name))
    }

    pub fn load_obj(&self, file_name: &str) -> Result<Mesh, String> {
        let path = self.root.join("models").join(file_name);
        let text =
            fs::read_to_string(&path).map_err(|_| format!("No pude leer {}.", path.display()))?;
        Mesh::from_obj(&text)
    }
}

pub fn load_image_path(path: &Path) -> Result<RgbaImage, String> {
    let image = image::open(path).map_err(|_| format!("No pude abrir {}.", path.display()))?;
    Ok(image.to_rgba8())
}

pub fn noise_tile(resolution: u32, square: bool) -> (RgbaImage, u32) {
    let useful_width = resolution.max(1);
    let useful_height = if square {
        useful_width
    } else {
        useful_width.saturating_mul(5)
    };
    let width = useful_width + 2;
    let height = useful_height + 2;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let index = ((x + y * width) * 4) as usize;
            pixels[index] = (js_random() * 255.9) as u8;
            pixels[index + 1] = (js_random() * 255.9) as u8;
            pixels[index + 2] = (js_random() * 255.9) as u8;
            pixels[index + 3] = 255;
        }
    }

    for x in 0..width {
        copy_wrapped(
            &mut pixels,
            width,
            height,
            useful_width,
            useful_height,
            x,
            0,
        );
        copy_wrapped(
            &mut pixels,
            width,
            height,
            useful_width,
            useful_height,
            x,
            height - 1,
        );
    }
    for y in 0..height {
        copy_wrapped(
            &mut pixels,
            width,
            height,
            useful_width,
            useful_height,
            0,
            y,
        );
        copy_wrapped(
            &mut pixels,
            width,
            height,
            useful_width,
            useful_height,
            width - 1,
            y,
        );
    }

    (
        RgbaImage::from_raw(width, height, pixels).expect("el tamaño del ruido coincide"),
        1,
    )
}

fn copy_wrapped(
    pixels: &mut [u8],
    width: u32,
    _height: u32,
    useful_width: u32,
    useful_height: u32,
    x: u32,
    y: u32,
) {
    let original_x = ((x + useful_width - 1) % useful_width) + 1;
    let original_y = ((y + useful_height - 1) % useful_height) + 1;
    let target = ((x + width * y) * 4) as usize;
    let source = ((original_x + width * original_y) * 4) as usize;
    pixels.copy_within(source..source + 4, target);
}

fn js_random() -> f32 {
    fastrand_like()
}

fn fastrand_like() -> f32 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = Cell::new(0x1234_5678_9abc_def0);
    }
    STATE.with(|state| {
        let mut value = state.get();
        value = value.wrapping_mul(6364136223846793005).wrapping_add(1);
        state.set(value);
        ((value >> 33) as f32) / (u32::MAX as f32)
    })
}
