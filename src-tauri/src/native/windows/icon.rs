use super::*;
use base64::Engine;
use image::ImageEncoder;
use windows::Win32::UI::Shell::ExtractIconExW;
/// Extracts an executable resource; never launches the application or a shell handler.
pub fn extract(executable: &str) -> Option<String> {
    unsafe {
        let path = wide(executable);
        let mut icon = HICON::default();
        if ExtractIconExW(PCWSTR(path.as_ptr()), 0, None, Some(&mut icon), 1) == 0
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
                biWidth: 32,
                biHeight: -32,
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
        std::ptr::write_bytes(pixels.cast::<u8>(), 0, 32 * 32 * 4);
        let result = (|| {
            DrawIconEx(dc, 0, 0, icon, 32, 32, 0, None, DI_NORMAL).ok()?;
            let bgra = std::slice::from_raw_parts(pixels.cast::<u8>(), 32 * 32 * 4);
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
                .write_image(&rgba, 32, 32, image::ExtendedColorType::Rgba8)
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
