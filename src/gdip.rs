//! Minimal GDI+ text drawing. The original WinForms launcher never calls
//! SetCompatibleTextRenderingDefault(false), so its controls draw text with GDI+
//! (Graphics.DrawString) rather than GDI. Using GDI+ here gives the same glyph
//! spacing, padding and anti-aliasing.

use core::ffi::c_void;
use std::cell::Cell;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{CreateCompatibleDC, DeleteDC, SelectObject, HDC, HFONT};

#[repr(C)]
struct StartupInput {
    version: u32,
    debug_callback: usize,
    suppress_background_thread: i32,
    suppress_external_codecs: i32,
}

#[repr(C)]
struct RectF {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

type P = *mut c_void;

#[link(name = "gdiplus")]
extern "system" {
    fn GdiplusStartup(token: *mut usize, input: *const StartupInput, output: P) -> i32;
    fn GdipCreateFromHDC(hdc: HDC, g: *mut P) -> i32;
    fn GdipDeleteGraphics(g: P) -> i32;
    fn GdipCreateFontFamilyFromName(name: *const u16, coll: P, fam: *mut P) -> i32;
    fn GdipCreateFont(fam: P, em: f32, style: i32, unit: i32, font: *mut P) -> i32;
    fn GdipCreateFontFromDC(hdc: HDC, font: *mut P) -> i32;
    fn GdipCreateSolidFill(argb: u32, brush: *mut P) -> i32;
    fn GdipDeleteBrush(b: P) -> i32;
    fn GdipCreateStringFormat(flags: i32, lang: u16, fmt: *mut P) -> i32;
    fn GdipDeleteStringFormat(fmt: P) -> i32;
    fn GdipSetStringFormatAlign(fmt: P, align: i32) -> i32;
    fn GdipSetStringFormatLineAlign(fmt: P, align: i32) -> i32;
    fn GdipSetStringFormatTrimming(fmt: P, trimming: i32) -> i32;
    fn GdipSetStringFormatHotkeyPrefix(fmt: P, prefix: i32) -> i32;
    fn GdipDrawString(g: P, s: *const u16, len: i32, font: P, rect: *const RectF, fmt: P, brush: P) -> i32;
    fn GdipMeasureString(
        g: P, s: *const u16, len: i32, font: P, layout: *const RectF, fmt: P,
        bbox: *mut RectF, chars: *mut i32, lines: *mut i32,
    ) -> i32;
}

const UNIT_POINT: i32 = 3;
const STYLE_REGULAR: i32 = 0;
const STYLE_BOLD: i32 = 1;
const FLAG_NO_WRAP: i32 = 0x1000;
const TRIM_NONE: i32 = 0;
const TRIM_ELLIPSIS_CHAR: i32 = 3;

pub enum HAlign {
    Left,
    Center,
}
pub enum VAlign {
    Top,
    Center,
}

thread_local! {
    static FONTS: Cell<(P, P)> = const { Cell::new((null_mut(), null_mut())) };
}

/// Starts GDI+ and creates "Microsoft Sans Serif" 8.25pt, the WinForms default font.
/// If that family isn't installed, falls back to the given GDI fonts (whatever
/// Windows substituted for them) so text is never missing.
pub fn init(gdi_regular: HFONT, gdi_bold: HFONT) {
    unsafe {
        let mut token = 0usize;
        let input = StartupInput { version: 1, debug_callback: 0, suppress_background_thread: 0, suppress_external_codecs: 0 };
        GdiplusStartup(&mut token, &input, null_mut());
        let name: Vec<u16> = "Microsoft Sans Serif".encode_utf16().chain(Some(0)).collect();
        let mut fam = null_mut();
        let (mut regular, mut bold) = (null_mut(), null_mut());
        if GdipCreateFontFamilyFromName(name.as_ptr(), null_mut(), &mut fam) == 0 {
            GdipCreateFont(fam, 8.25, STYLE_REGULAR, UNIT_POINT, &mut regular);
            GdipCreateFont(fam, 8.25, STYLE_BOLD, UNIT_POINT, &mut bold);
        }
        if regular.is_null() || bold.is_null() {
            let dc = CreateCompatibleDC(null_mut());
            let old = SelectObject(dc, gdi_regular as _);
            GdipCreateFontFromDC(dc, &mut regular);
            SelectObject(dc, gdi_bold as _);
            GdipCreateFontFromDC(dc, &mut bold);
            SelectObject(dc, old);
            DeleteDC(dc);
        }
        FONTS.with(|f| f.set((regular, bold)));
    }
}

fn argb(colorref: u32) -> u32 {
    let (r, g, b) = (colorref & 0xFF, (colorref >> 8) & 0xFF, (colorref >> 16) & 0xFF);
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

unsafe fn with_format<R>(h: HAlign, v: VAlign, ellipsis: bool, f: impl FnOnce(P) -> R) -> R {
    let mut fmt = null_mut();
    GdipCreateStringFormat(FLAG_NO_WRAP, 0, &mut fmt);
    GdipSetStringFormatAlign(fmt, match h { HAlign::Left => 0, HAlign::Center => 1 });
    GdipSetStringFormatLineAlign(fmt, match v { VAlign::Top => 0, VAlign::Center => 1 });
    GdipSetStringFormatTrimming(fmt, if ellipsis { TRIM_ELLIPSIS_CHAR } else { TRIM_NONE });
    GdipSetStringFormatHotkeyPrefix(fmt, 0);
    let r = f(fmt);
    GdipDeleteStringFormat(fmt);
    r
}

fn font(bold: bool) -> P {
    FONTS.with(|f| if bold { f.get().1 } else { f.get().0 })
}

pub unsafe fn draw(hdc: HDC, s: &str, r: &RECT, color: u32, h: HAlign, v: VAlign, bold: bool, ellipsis: bool) {
    let ws: Vec<u16> = s.encode_utf16().collect();
    let mut g = null_mut();
    if GdipCreateFromHDC(hdc, &mut g) != 0 {
        return;
    }
    let mut brush = null_mut();
    GdipCreateSolidFill(argb(color), &mut brush);
    let rect = RectF { x: r.left as f32, y: r.top as f32, w: (r.right - r.left) as f32, h: (r.bottom - r.top) as f32 };
    with_format(h, v, ellipsis, |fmt| {
        GdipDrawString(g, ws.as_ptr(), ws.len() as i32, font(bold), &rect, fmt, brush);
    });
    GdipDeleteBrush(brush);
    GdipDeleteGraphics(g);
}

/// Width and height of `s` as GDI+ lays it out (includes GDI+'s usual padding).
pub unsafe fn measure(hdc: HDC, s: &str, bold: bool) -> (f32, f32) {
    let ws: Vec<u16> = s.encode_utf16().collect();
    let mut g = null_mut();
    if GdipCreateFromHDC(hdc, &mut g) != 0 {
        return (0.0, 0.0);
    }
    let layout = RectF { x: 0.0, y: 0.0, w: 10000.0, h: 1000.0 };
    let mut bbox = RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
    with_format(HAlign::Left, VAlign::Top, false, |fmt| {
        GdipMeasureString(g, ws.as_ptr(), ws.len() as i32, font(bold), &layout, fmt, &mut bbox, null_mut(), null_mut());
    });
    GdipDeleteGraphics(g);
    (bbox.w, bbox.h)
}
