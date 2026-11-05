//! Win32 recreation of SelectionUI (the WinForms window), matching the original 1:1.
//! Layout coordinates come from the original InitializeComponent(); flat controls are
//! custom-drawn over the original background art, resolution boxes are real EDIT controls.

use crate::gdip;
use crate::logic::{self, Renderer, Settings, SHADOW_MAPS, WINDOW_MODES};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLIENT_W: i32 = 961;
const CLIENT_H: i32 = 493;
const TITLE: &str = "Anomaly Launcher v1.0.5";

const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | (g << 8) | (b << 16)
}
const DARK_GRAY: u32 = rgb(169, 169, 169);
const DISABLED_GRAY: u32 = rgb(105, 105, 105);
const CONTROL_BG: u32 = rgb(32, 32, 32);
const HOVER_BG: u32 = rgb(52, 52, 52);
const DOWN_BG: u32 = rgb(20, 20, 20);
const ORANGE_RED: u32 = rgb(255, 69, 0);
const GREEN_YELLOW: u32 = rgb(173, 255, 47);

const WM_MOUSELEAVE: u32 = 0x02A3;
const WM_APP_STATUS: u32 = WM_APP + 1;
const WM_APP_ADDONS_DONE: u32 = WM_APP + 2;
const WM_APP_VERIFY_DONE: u32 = WM_APP + 3;
const WM_APP_SHOWN: u32 = WM_APP + 4;

// Worker threads hand over status text through these; the UI
// thread is poked with at most one pending message at a time.
static STATUS: Mutex<Option<String>> = Mutex::new(None);
static STATUS_PENDING: AtomicBool = AtomicBool::new(false);

// Panel (options2.png) origin and the group boxes inside it.
const PANEL: (i32, i32) = (16, 40);
const fn p(x: i32, y: i32) -> (i32, i32) {
    (PANEL.0 + x, PANEL.1 + y)
}
const G1: (i32, i32, i32, i32) = (p(9, 52).0, p(9, 52).1, 192, 140);
const AVX_POS: (i32, i32) = p(15, 198);
const G2: (i32, i32, i32, i32) = (p(9, 228).0, p(9, 228).1, 192, 161);
const GG: (i32, i32, i32, i32) = (p(239, 87).0, p(239, 87).1, 267, 302);
const RADIO_Y: [i32; 4] = [19, 49, 79, 109];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ctl {
    Play,
    Quit,
    Verify,
    Save,
    DelShader,
    ResetLtx,
    Avx,
    ForceGfx,
    Debug,
    SoundFix,
    Prefetch,
    Radio(usize),
    ComboShadow,
    ComboWindow,
}

#[derive(PartialEq)]
enum Kind {
    Button,
    Check,
    Radio,
    Combo,
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT { left: x, top: y, right: x + w, bottom: y + h }
}

impl Ctl {
    fn all() -> Vec<Ctl> {
        let mut v = vec![
            Ctl::Play, Ctl::Quit, Ctl::Verify, Ctl::Save, Ctl::DelShader, Ctl::ResetLtx,
            Ctl::Avx, Ctl::ForceGfx, Ctl::Debug, Ctl::SoundFix, Ctl::Prefetch,
            Ctl::ComboShadow, Ctl::ComboWindow,
        ];
        v.extend((0..Renderer::ALL.len()).map(Ctl::Radio));
        v
    }
    fn kind(self) -> Kind {
        match self {
            Ctl::Play | Ctl::Quit | Ctl::Verify | Ctl::Save => Kind::Button,
            Ctl::Radio(_) => Kind::Radio,
            Ctl::ComboShadow | Ctl::ComboWindow => Kind::Combo,
            _ => Kind::Check,
        }
    }
    fn rect(self) -> RECT {
        match self {
            Ctl::Play => rect(573, 210, 364, 79),
            Ctl::DelShader => rect(573, 295, 180, 24),
            Ctl::ResetLtx => rect(757, 295, 180, 24),
            Ctl::Verify => rect(573, 325, 364, 40),
            Ctl::Save => rect(573, 371, 364, 40),
            Ctl::Quit => rect(573, 417, 364, 40),
            Ctl::Avx => rect(AVX_POS.0, AVX_POS.1, 186, 24),
            Ctl::ForceGfx => { let (x, y) = p(239, 57); rect(x, y, 267, 24) }
            Ctl::Radio(i) => rect(G1.0 + 6, G1.1 + RADIO_Y[i], 180, 24),
            Ctl::Debug => rect(G2.0 + 6, G2.1 + 19, 180, 24),
            Ctl::ComboShadow => rect(G2.0 + 6, G2.1 + 49, 180, 21),
            Ctl::SoundFix => rect(G2.0 + 6, G2.1 + 76, 180, 24),
            Ctl::Prefetch => rect(G2.0 + 6, G2.1 + 106, 180, 24),
            Ctl::ComboWindow => rect(GG.0 + 13, GG.1 + 74, 241, 21),
        }
    }
    fn text(self) -> &'static str {
        match self {
            Ctl::Play => "Play S.T.A.L.K.E.R. Anomaly",
            Ctl::Quit => "Quit",
            Ctl::Verify => "Verify Installed Version",
            Ctl::Save => "Save Settings",
            Ctl::DelShader => "Delete Shader Cache",
            Ctl::ResetLtx => "Restore default user.ltx",
            Ctl::Avx => "Support for AVX CPU",
            Ctl::ForceGfx => "Reset graphics settings",
            Ctl::Debug => "Debug Mode",
            Ctl::SoundFix => "Sound problem workaround",
            Ctl::Prefetch => "Prefetch sounds",
            Ctl::Radio(i) => Renderer::ALL[i].label(),
            Ctl::ComboShadow | Ctl::ComboWindow => "",
        }
    }
    /// Controls inside "Graphics settings" are hidden with it.
    fn in_gfx_group(self) -> bool {
        matches!(self, Ctl::ComboWindow)
    }
}

struct App {
    s: Settings,
    base: HBITMAP,
    font: HFONT,
    font_bold: HFONT,
    edit_brush: HBRUSH,
    edit_w: HWND,
    edit_h: HWND,
    hover: Option<Ctl>,
    pressed: Option<Ctl>,
    tracking: bool,
    warn_text: String,
    warn_color: u32,
    warn_visible: bool,
    should_start: bool,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.borrow_mut().as_mut().map(f))
}

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn hit(app: &App, x: i32, y: i32) -> Option<Ctl> {
    Ctl::all().into_iter().find(|c| {
        if c.in_gfx_group() && !app.s.force_gfx {
            return false;
        }
        let r = c.rect();
        x >= r.left && x < r.right && y >= r.top && y < r.bottom
    })
}

// ---------- images ----------

fn decode_png(bytes: &[u8]) -> (usize, usize, Vec<[u8; 4]>) {
    let mut dec = png::Decoder::new(bytes);
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut rd = dec.read_info().expect("png header");
    let mut buf = vec![0; rd.output_buffer_size()];
    let info = rd.next_frame(&mut buf).expect("png data");
    let (wd, ht) = (info.width as usize, info.height as usize);
    let ch = info.color_type.samples();
    let mut px = Vec::with_capacity(wd * ht);
    for y in 0..ht {
        let row = &buf[y * info.line_size..];
        for x in 0..wd {
            let s = &row[x * ch..x * ch + ch];
            px.push(match ch {
                4 => [s[0], s[1], s[2], s[3]],
                3 => [s[0], s[1], s[2], 255],
                2 => [s[0], s[0], s[0], s[1]],
                _ => [s[0], s[0], s[0], 255],
            });
        }
    }
    (wd, ht, px)
}

/// Pre-composes form background (bg.png) and the panel (options2.png) into one bitmap.
unsafe fn build_base() -> HBITMAP {
    let (bw, bh, bg) = decode_png(include_bytes!("../assets/bg.png"));
    let (ow, oh, op) = decode_png(include_bytes!("../assets/options2.png"));
    let mut out = vec![0u32; (CLIENT_W * CLIENT_H) as usize];
    for y in 0..CLIENT_H as usize {
        for x in 0..CLIENT_W as usize {
            let mut c = if x < bw && y < bh { bg[y * bw + x] } else { [0, 0, 0, 255] };
            let (px, py) = (x as i64 - PANEL.0 as i64, y as i64 - PANEL.1 as i64);
            if px >= 0 && py >= 0 && (px as usize) < ow && (py as usize) < oh {
                let o = op[py as usize * ow + px as usize];
                let a = o[3] as u32;
                for i in 0..3 {
                    c[i] = ((o[i] as u32 * a + c[i] as u32 * (255 - a) + 127) / 255) as u8;
                }
            }
            out[y * CLIENT_W as usize + x] = (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
        }
    }
    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = CLIENT_W;
    bmi.bmiHeader.biHeight = -CLIENT_H;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB as _;
    let mut bits: *mut core::ffi::c_void = null_mut();
    let hbm = CreateDIBSection(null_mut(), &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    std::ptr::copy_nonoverlapping(out.as_ptr(), bits as *mut u32, out.len());
    hbm
}

// ---------- drawing ----------

unsafe fn fill(hdc: HDC, r: &RECT, color: u32) {
    let b = CreateSolidBrush(color);
    FillRect(hdc, r, b);
    DeleteObject(b as _);
}

unsafe fn frame(hdc: HDC, r: &RECT, color: u32) {
    let b = CreateSolidBrush(color);
    FrameRect(hdc, r, b);
    DeleteObject(b as _);
}

thread_local! {
    static BOLD_FONT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Draws text with GDI+, like WinForms' compatible text rendering in the original.
/// `fmt` takes DrawText-style flags for alignment; boldness follows the selected font.
unsafe fn text(hdc: HDC, s: &str, r: &RECT, color: u32, fmt: u32) {
    let bold = GetCurrentObject(hdc, OBJ_FONT as _) as usize == BOLD_FONT.with(|b| b.get());
    let h = if fmt & DT_CENTER != 0 { gdip::HAlign::Center } else { gdip::HAlign::Left };
    let v = if fmt & DT_VCENTER != 0 { gdip::VAlign::Center } else { gdip::VAlign::Top };
    gdip::draw(hdc, s, r, color, h, v, bold, fmt & DT_END_ELLIPSIS != 0);
}

unsafe fn line(hdc: HDC, x0: i32, y0: i32, x1: i32, y1: i32) {
    MoveToEx(hdc, x0, y0, null_mut());
    LineTo(hdc, x1, y1);
}

unsafe fn with_pen(hdc: HDC, color: u32, width: i32, f: impl FnOnce()) {
    let pen = CreatePen(PS_SOLID, width, color);
    let old = SelectObject(hdc, pen as _);
    f();
    SelectObject(hdc, old);
    DeleteObject(pen as _);
}

// Pixel colours measured from the original launcher.
const EDGE_GRAY: u32 = rgb(85, 85, 85); // check/radio box edge
const ETCH_DARK: u32 = rgb(160, 160, 160); // group box etched line
const ETCH_LIGHT: u32 = rgb(255, 255, 255);
const BUTTON_BORDER: u32 = rgb(10, 10, 10);
const EDIT_BORDER: u32 = rgb(100, 100, 100);
const COMBO_BORDER: u32 = rgb(255, 255, 255);
const COMBO_BUTTON: u32 = rgb(240, 240, 240);

/// 12x12 radio button (W = white, G = edge, D = dot when checked, . = transparent).
const RADIO: [&str; 12] = [
    "....GGGG....",
    "..GGWWWWGG..",
    ".GWWWWWWWWG.",
    ".GWWWWWWWWG.",
    "GWWWWddWWWWG",
    "GWWWddddWWWG",
    "GWWWddddWWWG",
    "GWWWWddWWWWG",
    ".GWWWWWWWWG.",
    ".GWWWWWWWWG.",
    "..GGWWWWGG..",
    "....GGGG....",
];

/// 11x11 flat check box; `d` pixels form the check mark.
const CHECK: [&str; 11] = [
    "GGGGGGGGGGG",
    "GWWWWWWWWWG",
    "GWWWWWWWWWG",
    "GWWWWWWWdWG",
    "GWWWWWWddWG",
    "GWdWWWdddWG",
    "GWddWdddWWG",
    "GWdddddWWWG",
    "GWWdddWWWWG",
    "GWWWdWWWWWG",
    "GGGGGGGGGGG",
];

unsafe fn sprite(hdc: HDC, x: i32, y: i32, rows: &[&str], marked: bool, mark: u32, edge: u32) {
    for (dy, row) in rows.iter().enumerate() {
        for (dx, ch) in row.bytes().enumerate() {
            let c = match ch {
                b'W' => rgb(255, 255, 255),
                b'G' => edge,
                b'd' if marked => mark,
                b'd' => rgb(255, 255, 255),
                _ => continue,
            };
            SetPixelV(hdc, x + dx as i32, y + dy as i32, c);
        }
    }
}

/// GDI text, used where the original shows native Win32 controls (combo boxes).
unsafe fn text_gdi(hdc: HDC, s: &str, r: &RECT, color: u32, fmt: u32) {
    let mut r = *r;
    let ws: Vec<u16> = s.encode_utf16().collect();
    SetTextColor(hdc, color);
    DrawTextW(hdc, ws.as_ptr(), ws.len() as i32, &mut r, fmt as _);
}

/// Classic (non-themed) group box: etched frame with the title cut into the top line.
unsafe fn group_box(hdc: HDC, g: (i32, i32, i32, i32), title: &str) {
    let (x, y, wd, ht) = g;
    let (tw, _) = gdip::measure(hdc, title, false);
    let top = y + 5;
    let gap0 = x + 9;
    let gap1 = x + 8 + tw.ceil() as i32;
    crate::trace::log(&format!("group '{}' title width {:.2}", title, tw));
    let r = x + wd - 1;
    let b = y + ht - 1;
    for (off, color) in [(0, ETCH_DARK), (1, ETCH_LIGHT)] {
        with_pen(hdc, color, 1, || {
            // top (split around the title), left, right, bottom
            line(hdc, x + off, top + off, gap0, top + off);
            line(hdc, gap1, top + off, r - 1 + off, top + off);
            line(hdc, x + off, top + off, x + off, b - 1 + off);
            line(hdc, r - 1 + off, top + off, r - 1 + off, b - 1 + off);
            line(hdc, x + off, b - 1 + off, r + off, b - 1 + off);
        });
    }
    text(hdc, title, &rect(x + 8, y, wd - 16, 16), DARK_GRAY, DT_LEFT | DT_TOP | DT_SINGLELINE);
}

unsafe fn draw_button(hdc: HDC, app: &App, c: Ctl, focused: bool) {
    let r = c.rect();
    let bg = if app.pressed == Some(c) && app.hover == Some(c) {
        DOWN_BG
    } else if app.hover == Some(c) {
        HOVER_BG
    } else {
        CONTROL_BG
    };
    fill(hdc, &r, bg);
    frame(hdc, &r, BUTTON_BORDER);
    if focused {
        // The default (focused) button gets a second border line, as in WinForms.
        let inner = RECT { left: r.left + 1, top: r.top + 1, right: r.right - 1, bottom: r.bottom - 1 };
        frame(hdc, &inner, BUTTON_BORDER);
    }
    text(hdc, c.text(), &r, DARK_GRAY, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
}

/// Text rectangle for check boxes and radio buttons (GDI+ adds its own padding).
fn check_text_rect(r: &RECT) -> RECT {
    RECT { left: r.left + 15, ..*r }
}

unsafe fn draw_check(hdc: HDC, c: Ctl, checked: bool, enabled: bool) {
    let r = c.rect();
    let fg = if enabled { DARK_GRAY } else { DISABLED_GRAY };
    let cy = (r.top + r.bottom) / 2;
    sprite(hdc, r.left, cy - 6, &CHECK, checked, fg, if enabled { EDGE_GRAY } else { DISABLED_GRAY });
    text(hdc, c.text(), &check_text_rect(&r), fg, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
}

unsafe fn draw_radio(hdc: HDC, c: Ctl, checked: bool) {
    let r = c.rect();
    let cy = (r.top + r.bottom) / 2;
    sprite(hdc, r.left, cy - 6, &RADIO, checked, DARK_GRAY, EDGE_GRAY);
    text(hdc, c.text(), &check_text_rect(&r), DARK_GRAY, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
}

/// Flat drop-down list: white frame, light-grey arrow button with a black arrow.
unsafe fn draw_combo(hdc: HDC, app: &App, c: Ctl, value: &str) {
    let r = c.rect();
    fill(hdc, &r, if app.hover == Some(c) { HOVER_BG } else { CONTROL_BG });
    frame(hdc, &r, COMBO_BORDER);
    let btn = RECT { left: r.right - 18, top: r.top, right: r.right, bottom: r.bottom };
    fill(hdc, &btn, COMBO_BUTTON);
    frame(hdc, &btn, COMBO_BORDER);
    let ax = r.right - 11;
    let ay = r.top + 9;
    for i in 0..3 {
        let row = RECT { left: ax + i, top: ay + i, right: ax + 5 - i, bottom: ay + i + 1 };
        fill(hdc, &row, rgb(0, 0, 0));
    }
    let tr = RECT { left: r.left + 4, top: r.top + 3, right: r.right - 19, bottom: r.bottom };
    text_gdi(hdc, value, &tr, DARK_GRAY, DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX);
}

/// Edit boxes are child windows; their 1px grey border is painted by the parent.
unsafe fn draw_edit_frame(hdc: HDC, x: i32, y: i32) {
    frame(hdc, &rect(x, y, 115, 20), EDIT_BORDER);
}

unsafe fn paint(hwnd: HWND, app: &App) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mem = CreateCompatibleDC(hdc);
    let buf = CreateCompatibleBitmap(hdc, CLIENT_W, CLIENT_H);
    let old_buf = SelectObject(mem, buf as _);

    let src = CreateCompatibleDC(hdc);
    let old_src = SelectObject(src, app.base as _);
    BitBlt(mem, 0, 0, CLIENT_W, CLIENT_H, src, 0, 0, SRCCOPY);
    SelectObject(src, old_src);
    DeleteDC(src);

    SetBkMode(mem, TRANSPARENT as _);
    let old_font = SelectObject(mem, app.font as _);

    group_box(mem, G1, " Renderer ");
    group_box(mem, G2, " Misc Settings ");
    if app.s.force_gfx {
        group_box(mem, GG, " Graphics settings ");
        text(mem, "Screen Resolution", &rect(GG.0 + 10, GG.1 + 20, 251, 23), DARK_GRAY, DT_LEFT | DT_TOP | DT_SINGLELINE);
        draw_edit_frame(mem, GG.0 + 13, GG.1 + 41);
        draw_edit_frame(mem, GG.0 + 139, GG.1 + 41);
    }
    let s = &app.s;
    let play_focused = GetFocus() == hwnd;
    for c in Ctl::all() {
        match c {
            Ctl::Play | Ctl::Quit | Ctl::Verify | Ctl::Save => draw_button(mem, app, c, c == Ctl::Play && play_focused),
            Ctl::Radio(i) => draw_radio(mem, c, s.renderer == Some(Renderer::ALL[i])),
            Ctl::Avx => draw_check(mem, c, s.avx, s.avx_enabled),
            Ctl::ForceGfx => draw_check(mem, c, s.force_gfx, true),
            Ctl::Debug => draw_check(mem, c, s.debug, true),
            Ctl::SoundFix => draw_check(mem, c, s.sound_fix, true),
            Ctl::Prefetch => draw_check(mem, c, s.prefetch_sounds, true),
            Ctl::DelShader => draw_check(mem, c, s.delete_shader_cache, true),
            Ctl::ResetLtx => draw_check(mem, c, s.reset_user_ltx, true),
            Ctl::ComboShadow => draw_combo(mem, app, c, SHADOW_MAPS[s.shadow_map.min(4)]),
            Ctl::ComboWindow => {
                if s.force_gfx {
                    draw_combo(mem, app, c, WINDOW_MODES[s.window_mode.min(2)])
                }
            }
        }
    }
    if app.warn_visible {
        SelectObject(mem, app.font_bold as _);
        text(mem, &app.warn_text, &rect(25, 460, 912, 23), app.warn_color,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
    }
    SelectObject(mem, old_font);

    BitBlt(hdc, 0, 0, CLIENT_W, CLIENT_H, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old_buf);
    DeleteObject(buf as _);
    DeleteDC(mem);
    EndPaint(hwnd, &ps);
}

// ---------- behaviour ----------

unsafe fn invalidate(hwnd: HWND) {
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn get_text(h: HWND) -> String {
    let n = GetWindowTextLengthW(h);
    let mut buf = vec![0u16; n as usize + 1];
    let got = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
    String::from_utf16_lossy(&buf[..got.max(0) as usize])
}

/// Pulls the edit boxes' text into the settings snapshot.
unsafe fn sync_edits() -> Settings {
    let (ew, eh) = with_app(|a| (a.edit_w, a.edit_h)).unwrap();
    let (tw, th) = (get_text(ew), get_text(eh));
    with_app(|a| {
        a.s.res_width = tw;
        a.s.res_height = th;
        a.s.clone()
    })
    .unwrap()
}

unsafe fn msg_box(hwnd: HWND, body: &str, caption: &str, flags: u32) {
    MessageBoxW(hwnd, w(body).as_ptr(), w(caption).as_ptr(), flags);
}

unsafe fn set_gfx_visible(visible: bool) {
    let (ew, eh) = with_app(|a| (a.edit_w, a.edit_h)).unwrap();
    let cmd = if visible { SW_SHOW } else { SW_HIDE };
    ShowWindow(ew, cmd);
    ShowWindow(eh, cmd);
}

unsafe fn popup(hwnd: HWND, c: Ctl, items: &[&str], current: usize) -> Option<usize> {
    let menu = CreatePopupMenu();
    for (i, it) in items.iter().enumerate() {
        let flags = MF_STRING | if i == current { MF_CHECKED } else { 0 };
        AppendMenuW(menu, flags, i + 1, w(it).as_ptr());
    }
    let r = c.rect();
    let mut pt = POINT { x: r.left, y: r.bottom };
    ClientToScreen(hwnd, &mut pt);
    let id = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN | TPM_NONOTIFY, pt.x, pt.y, 0, hwnd, null());
    DestroyMenu(menu);
    if id > 0 { Some(id as usize - 1) } else { None }
}

fn post_box<T>(hwnd: HWND, msg: u32, wparam: usize, v: T) {
    let ptr = Box::into_raw(Box::new(v)) as isize;
    unsafe {
        PostMessageW(hwnd, msg, wparam, ptr);
    }
}

unsafe fn take_box<T>(lparam: LPARAM) -> T {
    *Box::from_raw(lparam as *mut T)
}

fn push_status(hwnd: HWND, s: String) {
    *STATUS.lock().unwrap() = Some(s);
    if !STATUS_PENDING.swap(true, Ordering::AcqRel) {
        unsafe { PostMessageW(hwnd, WM_APP_STATUS, 0, 0) };
    }
}

/// SelectionUI_Shown: look for addons in the background. Unlike the original,
/// the window stays usable, and the scan stops at the first non-stock file.
unsafe fn start_addon_scan(hwnd: HWND) {
    let h = hwnd as usize;
    std::thread::spawn(move || {
        let t = std::time::Instant::now();
        let found = logic::find_addon().is_some();
        crate::trace::log(&format!("startup addon scan took {:.2?}", t.elapsed()));
        unsafe { PostMessageW(h as HWND, WM_APP_ADDONS_DONE, found as usize, 0) };
    });
}

unsafe fn start_verify(hwnd: HWND) {
    with_app(|a| {
        a.s.delete_shader_cache = true;
        a.warn_color = ORANGE_RED;
        a.warn_text = "Verifying install".into();
        a.warn_visible = true;
    });
    invalidate(hwnd);
    EnableWindow(hwnd, 0);
    let h = hwnd as usize;
    std::thread::spawn(move || {
        let hwnd = h as HWND;
        let r = logic::verify_install(&|s| push_status(h as HWND, s));
        post_box(hwnd, WM_APP_VERIFY_DONE, 0, r);
    });
}

unsafe fn activate(hwnd: HWND, c: Ctl) {
    match c {
        Ctl::Quit => {
            DestroyWindow(hwnd);
        }
        Ctl::Play => {
            let s = sync_edits();
            match logic::on_play(&s) {
                Ok(()) => {
                    with_app(|a| a.should_start = true);
                    DestroyWindow(hwnd);
                }
                Err(e) => msg_box(hwnd, &e, "Anomaly Launcher", MB_OK | MB_ICONERROR),
            }
        }
        Ctl::Save => {
            let s = sync_edits();
            logic::save_settings(&s);
            msg_box(hwnd, "Settings saved.", "Anomaly Launcher", MB_OK | MB_ICONINFORMATION);
        }
        Ctl::Verify => start_verify(hwnd),
        Ctl::ComboShadow => {
            let cur = with_app(|a| a.s.shadow_map).unwrap();
            if let Some(i) = popup(hwnd, c, &SHADOW_MAPS, cur) {
                with_app(|a| a.s.shadow_map = i);
            }
        }
        Ctl::ComboWindow => {
            let cur = with_app(|a| a.s.window_mode).unwrap();
            if let Some(i) = popup(hwnd, c, &WINDOW_MODES, cur) {
                with_app(|a| a.s.window_mode = i);
            }
        }
        Ctl::ForceGfx => {
            let v = with_app(|a| {
                a.s.force_gfx = !a.s.force_gfx;
                a.s.force_gfx
            })
            .unwrap();
            set_gfx_visible(v);
        }
        _ => {
            with_app(|a| match c {
                Ctl::Radio(i) => a.s.renderer = Some(Renderer::ALL[i]),
                Ctl::Avx => {
                    if a.s.avx_enabled {
                        a.s.avx = !a.s.avx
                    }
                }
                Ctl::Debug => a.s.debug = !a.s.debug,
                Ctl::SoundFix => a.s.sound_fix = !a.s.sound_fix,
                Ctl::Prefetch => a.s.prefetch_sounds = !a.s.prefetch_sounds,
                Ctl::DelShader => a.s.delete_shader_cache = !a.s.delete_shader_cache,
                Ctl::ResetLtx => a.s.reset_user_ltx = !a.s.reset_user_ltx,
                _ => {}
            });
        }
    }
    invalidate(hwnd);
}

fn xy(lparam: LPARAM) -> (i32, i32) {
    ((lparam & 0xFFFF) as i16 as i32, ((lparam >> 16) & 0xFFFF) as i16 as i32)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => 1,
        WM_SETFOCUS | WM_KILLFOCUS => {
            // Play's double border follows keyboard focus.
            invalidate(hwnd);
            0
        }
        WM_PAINT => {
            let painted = APP.with(|a| {
                if let Some(app) = a.borrow().as_ref() {
                    paint(hwnd, app);
                    true
                } else {
                    false
                }
            });
            if painted { 0 } else { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC;
            SetTextColor(hdc, DARK_GRAY);
            SetBkColor(hdc, CONTROL_BG);
            with_app(|a| a.edit_brush as LRESULT).unwrap_or(0)
        }
        WM_MOUSEMOVE => {
            let (x, y) = xy(lparam);
            let (changed, need_track) = with_app(|a| {
                let h = hit(a, x, y).filter(|c| matches!(c.kind(), Kind::Button | Kind::Combo));
                let changed = h != a.hover;
                a.hover = h;
                let need = !a.tracking;
                a.tracking = true;
                (changed, need)
            })
            .unwrap_or((false, false));
            if need_track {
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                TrackMouseEvent(&mut tme);
            }
            if changed {
                invalidate(hwnd);
            }
            0
        }
        WM_MOUSELEAVE => {
            with_app(|a| {
                a.hover = None;
                a.tracking = false;
            });
            invalidate(hwnd);
            0
        }
        WM_LBUTTONDOWN => {
            SetFocus(hwnd);
            let (x, y) = xy(lparam);
            let c = with_app(|a| {
                let c = hit(a, x, y);
                a.pressed = c;
                c
            })
            .flatten();
            if let Some(c) = c {
                if c.kind() == Kind::Combo {
                    with_app(|a| a.pressed = None);
                    activate(hwnd, c);
                } else {
                    SetCapture(hwnd);
                    invalidate(hwnd);
                }
            }
            0
        }
        WM_LBUTTONUP => {
            ReleaseCapture();
            let (x, y) = xy(lparam);
            let c = with_app(|a| {
                let p = a.pressed.take();
                let h = hit(a, x, y);
                if p.is_some() && p == h { p } else { None }
            })
            .flatten();
            invalidate(hwnd);
            if let Some(c) = c {
                activate(hwnd, c);
            }
            0
        }
        WM_KEYDOWN => {
            if wparam as u16 == VK_RETURN {
                // Play has focus on start-up in the original, so Enter launches the game.
                activate(hwnd, Ctl::Play);
            }
            0
        }
        WM_APP_SHOWN => {
            start_addon_scan(hwnd);
            0
        }
        WM_APP_STATUS => {
            STATUS_PENDING.store(false, Ordering::Release);
            if let Some(s) = STATUS.lock().unwrap().take() {
                with_app(|a| a.warn_text = s);
            }
            invalidate(hwnd);
            0
        }
        WM_APP_ADDONS_DONE => {
            if wparam != 0 {
                with_app(|a| {
                    a.warn_text = "* installed addons can cause problems or crashes *".into();
                    a.warn_visible = true;
                });
            }
            invalidate(hwnd);
            0
        }
        WM_APP_VERIFY_DONE => {
            let r: logic::VerifyResult = take_box(lparam);
            let list = |v: &[String]| v.iter().map(|s| format!("{}\r\n", s)).collect::<String>();
            if !r.corrupt.is_empty() {
                msg_box(hwnd, &list(&r.corrupt), "Corrupt Files", MB_OK | MB_ICONERROR);
            }
            if !r.missing.is_empty() {
                msg_box(hwnd, &list(&r.missing), "Missing Files", MB_OK | MB_ICONERROR);
            }
            with_app(|a| {
                if r.message.is_empty() {
                    a.warn_color = GREEN_YELLOW;
                    a.warn_text = "Verification successful".into();
                } else {
                    a.warn_text = r.message;
                }
                a.warn_visible = true;
            });
            EnableWindow(hwnd, 1);
            invalidate(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn make_font(height: i32, weight: i32) -> HFONT {
    CreateFontW(
        height, 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET as _, OUT_DEFAULT_PRECIS as _,
        CLIP_DEFAULT_PRECIS as _, DEFAULT_QUALITY as _, (DEFAULT_PITCH | FF_DONTCARE) as _,
        w("Microsoft Sans Serif").as_ptr(),
    )
}

unsafe fn make_edit(parent: HWND, inst: HINSTANCE, x: i32, y: i32, value: &str, font: HFONT) -> HWND {
    let h = CreateWindowExW(
        0, w("EDIT").as_ptr(), w(value).as_ptr(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        x + 1, y + 1, 113, 18, parent, null_mut(), inst, null(),
    );
    SendMessageW(h, WM_SETFONT, font as usize, 1);
    // EM_SETMARGINS: 1px left/right, matching the original text position.
    SendMessageW(h, 0x00D3, 0x1 | 0x2, (1 | (1 << 16)) as isize);
    h
}

/// Shows the launcher window. Returns (should_start, delete_shader_cache).
pub fn run(has_avx: bool) -> (bool, bool) {
    unsafe {
        let inst = GetModuleHandleW(null());
        let class = w("AnomalyLauncherRs");
        let icon = LoadIconW(inst, 1 as _);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: inst,
            hIcon: icon,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class.as_ptr(),
            hIconSm: icon,
        };
        RegisterClassExW(&wc);

        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let settings = logic::load_settings(has_avx, sw, sh);

        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN;
        let mut r = rect(0, 0, CLIENT_W, CLIENT_H);
        AdjustWindowRect(&mut r, style, 0);
        let (ww, wh) = (r.right - r.left, r.bottom - r.top);
        let hwnd = CreateWindowExW(
            0, class.as_ptr(), w(TITLE).as_ptr(), style,
            (sw - ww) / 2, (sh - wh) / 2, ww, wh,
            null_mut(), null_mut(), inst, null(),
        );

        let font = make_font(-11, FW_NORMAL as i32);
        let font_bold = make_font(-11, FW_BOLD as i32);
        BOLD_FONT.with(|b| b.set(font_bold as usize));
        gdip::init(font, font_bold);
        let edit_w = make_edit(hwnd, inst, GG.0 + 13, GG.1 + 41, &settings.res_width, font);
        let edit_h = make_edit(hwnd, inst, GG.0 + 139, GG.1 + 41, &settings.res_height, font);
        let force_gfx = settings.force_gfx;

        APP.with(|a| {
            *a.borrow_mut() = Some(App {
                s: settings,
                base: build_base(),
                font,
                font_bold,
                edit_brush: CreateSolidBrush(CONTROL_BG),
                edit_w,
                edit_h,
                hover: None,
                pressed: None,
                tracking: false,
                warn_text: "OK".into(),
                warn_color: ORANGE_RED,
                warn_visible: false,
                should_start: false,
            })
        });
        set_gfx_visible(force_gfx);

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        SetFocus(hwnd);
        PostMessageW(hwnd, WM_APP_SHOWN, 0, 0);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        APP.with(|a| {
            a.borrow_mut()
                .take()
                .map(|app| (app.should_start, app.s.delete_shader_cache))
                .unwrap_or((false, false))
        })
    }
}
