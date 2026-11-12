use super::*;
use base64::Engine;
use image::ImageEncoder;
use windows::Win32::UI::Shell::SHDefExtractIconW;

// Enough source pixels for the 22 CSS-pixel application icon on high-DPI displays.
const ICON_SIZE: u32 = 128;
const PIXEL_BYTES: usize = (ICON_SIZE * ICON_SIZE * 4) as usize;

/// Extracts an executable resource; never launches the application or a shell handler.
pub fn extract(executable: &str) -> Option<String> {
    unsafe {
        let path = wide(executable);
        let mut icon = HICON::default();
        if SHDefExtractIconW(
            PCWSTR(path.as_ptr()),
            0,
            0,
            Some(&mut icon),
            None,
            ICON_SIZE,
        )
        .is_err()
            || icon.is_invalid()
        {
            return None;
        }
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            let _ = DestroyIcon(icon);
            return None;
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: ICON_SIZE as i32,
                biHeight: -(ICON_SIZE as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut pixels, None, 0) {
            Ok(value) => value,
            Err(_) => {
                let _ = DeleteDC(dc);
                let _ = DestroyIcon(icon);
                return None;
            }
        };
        let previous = SelectObject(dc, HGDIOBJ(bitmap.0));
        std::ptr::write_bytes(pixels.cast::<u8>(), 0, PIXEL_BYTES);
        let result = (|| {
            DrawIconEx(
                dc,
                0,
                0,
                icon,
                ICON_SIZE as i32,
                ICON_SIZE as i32,
                0,
                None,
                DI_NORMAL,
            )
            .ok()?;
            GdiFlush().ok().ok()?;
            let bgra = std::slice::from_raw_parts(pixels.cast::<u8>(), PIXEL_BYTES);
            let mut rgba = Vec::with_capacity(bgra.len());
            for px in bgra.chunks_exact(4) {
                let alpha = px[3];
                let unpremultiply = |c: u8| {
                    if alpha > 0 {
                        ((u32::from(c) * 255) / u32::from(alpha)).min(255) as u8
                    } else {
                        0
                    }
                };
                rgba.extend_from_slice(&[
                    unpremultiply(px[2]),
                    unpremultiply(px[1]),
                    unpremultiply(px[0]),
                    alpha,
                ]);
            }
            let mut bytes = vec![];
            image::codecs::png::PngEncoder::new(&mut bytes)
                .write_image(&rgba, ICON_SIZE, ICON_SIZE, image::ExtendedColorType::Rgba8)
                .ok()?;
            Some(format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            ))
        })();
        SelectObject(dc, previous);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);
        let _ = DestroyIcon(icon);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_high_resolution_resource_with_transparency() {
        // Different colors distinguish the real large resource from an upscaled small icon.
        let mut resources = Vec::new();
        for (size, color) in [(16u32, [240, 60, 20, 255]), (128, [20, 160, 240, 255])] {
            let image = image::RgbaImage::from_fn(size, size, |x, y| {
                if x >= 2 && y >= 2 && x < size - 2 && y < size - 2 {
                    image::Rgba(color)
                } else {
                    image::Rgba([0, 0, 0, 0])
                }
            });
            let mut png = Vec::new();
            image::codecs::png::PngEncoder::new(&mut png)
                .write_image(image.as_raw(), size, size, image::ExtendedColorType::Rgba8)
                .unwrap();
            resources.push((size, png));
        }
        let mut ico = vec![0, 0, 1, 0, 2, 0];
        let mut offset = 6 + 16 * resources.len() as u32;
        for (size, png) in &resources {
            ico.extend_from_slice(&[*size as u8, *size as u8, 0, 0]);
            ico.extend_from_slice(&1u16.to_le_bytes());
            ico.extend_from_slice(&32u16.to_le_bytes());
            ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
            ico.extend_from_slice(&offset.to_le_bytes());
            offset += png.len() as u32;
        }
        for (_, png) in resources {
            ico.extend_from_slice(&png);
        }
        let path =
            std::env::temp_dir().join(format!("audioflow-icon-{}.ico", uuid::Uuid::new_v4()));
        std::fs::write(&path, ico).unwrap();
        let result = extract(path.to_str().unwrap());
        std::fs::remove_file(path).unwrap();
        let data = result.expect("Windows should extract the icon resource");
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data.strip_prefix("data:image/png;base64,").unwrap())
            .unwrap();
        let image = image::load_from_memory(&bytes).unwrap().into_rgba8();
        assert_eq!(image.dimensions(), (128, 128));
        assert_eq!(image.get_pixel(64, 64).0, [20, 160, 240, 255]);
        assert_eq!(image.get_pixel(0, 0).0[3], 0);
    }
}
