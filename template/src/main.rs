//! Thin binary for the AppHost-generated app.
//!
//! All testable wiring lives in the `app_skeleton` library. This
//! file is only the process entry point:
//!
//! * Windows: single-instance check, then the hand-declared Win32
//!   window + tray shell in the `windows_gui` module below.
//! * Other platforms: run the library wiring self-check and exit.
//!   This keeps `cargo build` / `cargo test` meaningful on a Linux
//!   development host without pretending to be the desktop app.

#[cfg(not(windows))]
fn main() {
    match app_skeleton::run_self_check() {
        Ok(report) => {
            println!("{} wiring self-check passed", app_skeleton::APP_NAME);
            println!("  data dir:    {}", report.data_dir);
            println!("  launch #:    {}", report.launch_count);
            println!("  demo job:    {:.0}%", report.job_percent);
            println!("  demo child:  {}", report.child_stdout);
            println!("  updates:     {}", report.update_outcome);
        }
        Err(e) => {
            eprintln!("wiring self-check failed: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(windows)]
fn main() {
    windows_gui::run();
}

#[cfg(windows)]
mod windows_gui {
    //! Hand-declared Win32 GUI: dark window, status line, tray icon.
    //!
    //! LOAD-BEARING (learned from ytTrayDL): the tray icon's callback
    //! message is handled *inside this window procedure*. An earlier
    //! design polled a global event channel from the UI loop and
    //! tray clicks were silently never delivered in production. Do
    //! not move tray handling to a polled receiver.
    //!
    //! Single-instance: `Local\<APP_ID>-SingleInstance` mutex. A
    //! second launch finds the first window, posts the registered
    //! wake message, and exits.

    use app_skeleton::wiring::jobs;
    use app_skeleton::{AppContext, APP_ID, APP_NAME, VERSION};
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

    type Hwnd = *mut c_void;
    type Hmenu = *mut c_void;
    type Hicon = *mut c_void;
    type Hinstance = *mut c_void;
    type Hbrush = *mut c_void;
    type Hdc = *mut c_void;
    type Lresult = isize;
    type Wparam = usize;
    type Lparam = isize;

    const WM_DESTROY: u32 = 0x0002;
    const WM_PAINT: u32 = 0x000F;
    const WM_CLOSE: u32 = 0x0010;
    const WM_COMMAND: u32 = 0x0111;
    const WM_SYSCOMMAND: u32 = 0x0112;
    const WM_TIMER: u32 = 0x0113;
    const WM_APP: u32 = 0x8000;
    const WM_TRAY: u32 = WM_APP + 1;
    const WM_LBUTTONUP: u32 = 0x0202;
    const WM_RBUTTONUP: u32 = 0x0205;
    const WM_CONTEXTMENU: u32 = 0x007B;

    const SC_MINIMIZE: usize = 0xF020;
    const SIZE_RESTORED_WPARAM: usize = 0;

    const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
    const SW_HIDE: i32 = 0;
    const SW_SHOW: i32 = 5;
    const SW_RESTORE: i32 = 9;
    const TIMER_ID: usize = 1;

    const ID_OPEN: usize = 1001;
    const ID_JOB: usize = 1002;
    const ID_CHILD: usize = 1003;
    const ID_UPDATE: usize = 1004;
    const ID_EXIT: usize = 1005;

    const NIM_ADD: u32 = 0;
    const NIM_DELETE: u32 = 2;
    const NIF_MESSAGE: u32 = 0x0000_0001;
    const NIF_ICON: u32 = 0x0000_0002;
    const NIF_TIP: u32 = 0x0000_0004;

    const MF_STRING: u32 = 0x0000;
    const MF_SEPARATOR: u32 = 0x0800;
    const TPM_RETURNCMD: u32 = 0x0100;
    const TPM_RIGHTBUTTON: u32 = 0x0002;

    const ERROR_ALREADY_EXISTS: u32 = 183;
    const COLOR_BG: u32 = 0x0010_1010; // near-black (COLORREF 0x00BBGGRR)
    const COLOR_ACCENT: u32 = 0x0000_00E8; // family red, RGB(232, 0, 0)
    const COLOR_TEXT: u32 = 0x00D8_D8D8; // light gray

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct Msg {
        hwnd: Hwnd,
        message: u32,
        w_param: Wparam,
        l_param: Lparam,
        time: u32,
        pt: Point,
        l_private: u32,
    }

    #[repr(C)]
    struct PaintStruct {
        hdc: Hdc,
        erase: i32,
        rc_paint: Rect,
        restore: i32,
        inc_update: i32,
        reserved: [u8; 32],
    }

    type WndProc = unsafe extern "system" fn(Hwnd, u32, Wparam, Lparam) -> Lresult;

    #[repr(C)]
    struct WndClassW {
        style: u32,
        wnd_proc: Option<WndProc>,
        cls_extra: i32,
        wnd_extra: i32,
        instance: Hinstance,
        icon: Hicon,
        cursor: *mut c_void,
        background: Hbrush,
        menu_name: *const u16,
        class_name: *const u16,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    struct NotifyIconDataW {
        cb_size: u32,
        hwnd: Hwnd,
        id: u32,
        flags: u32,
        callback_message: u32,
        icon: Hicon,
        tip: [u16; 128],
        state: u32,
        state_mask: u32,
        info: [u16; 256],
        timeout_or_version: u32,
        info_title: [u16; 64],
        info_flags: u32,
        guid_item: Guid,
        balloon_icon: Hicon,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
        fn CreateMutexW(
            attributes: *mut c_void,
            initial_owner: i32,
            name: *const u16,
        ) -> *mut c_void;
        fn GetLastError() -> u32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn RegisterClassW(class: *const WndClassW) -> u16;
        fn CreateWindowExW(
            ex_style: u32,
            class_name: *const u16,
            window_name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Hwnd,
            menu: Hmenu,
            instance: Hinstance,
            param: *mut c_void,
        ) -> Hwnd;
        fn DefWindowProcW(hwnd: Hwnd, msg: u32, w_param: Wparam, l_param: Lparam) -> Lresult;
        fn ShowWindow(hwnd: Hwnd, cmd: i32) -> i32;
        fn SetForegroundWindow(hwnd: Hwnd) -> i32;
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> Hwnd;
        fn PostMessageW(hwnd: Hwnd, msg: u32, w_param: Wparam, l_param: Lparam) -> i32;
        fn RegisterWindowMessageW(name: *const u16) -> u32;
        fn GetMessageW(msg: *mut Msg, hwnd: Hwnd, min: u32, max: u32) -> i32;
        fn TranslateMessage(msg: *const Msg) -> i32;
        fn DispatchMessageW(msg: *const Msg) -> Lresult;
        fn PostQuitMessage(exit_code: i32);
        fn DestroyWindow(hwnd: Hwnd) -> i32;
        fn MessageBoxW(hwnd: Hwnd, text: *const u16, caption: *const u16, kind: u32) -> i32;
        fn LoadCursorW(instance: Hinstance, name: *const u16) -> *mut c_void;
        fn LoadIconW(instance: Hinstance, name: *const u16) -> Hicon;
        fn SetTimer(hwnd: Hwnd, id: usize, elapse: u32, callback: Option<WndProc>) -> usize;
        fn KillTimer(hwnd: Hwnd, id: usize) -> i32;
        fn InvalidateRect(hwnd: Hwnd, rect: *const Rect, erase: i32) -> i32;
        fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
        fn BeginPaint(hwnd: Hwnd, paint: *mut PaintStruct) -> Hdc;
        fn EndPaint(hwnd: Hwnd, paint: *const PaintStruct) -> i32;
        fn FillRect(hdc: Hdc, rect: *const Rect, brush: Hbrush) -> i32;
        fn GetClientRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
        fn SetBkMode(hdc: Hdc, mode: i32) -> i32;
        fn SetTextColor(hdc: Hdc, color: u32) -> u32;
        fn DrawTextW(hdc: Hdc, text: *const u16, count: i32, rect: *mut Rect, format: u32) -> i32;
        fn CreatePopupMenu() -> Hmenu;
        fn AppendMenuW(menu: Hmenu, flags: u32, id: usize, text: *const u16) -> i32;
        fn DestroyMenu(menu: Hmenu) -> i32;
        fn TrackPopupMenu(
            menu: Hmenu,
            flags: u32,
            x: i32,
            y: i32,
            reserved: i32,
            hwnd: Hwnd,
            rect: *const Rect,
        ) -> i32;
        fn GetCursorPos(point: *mut Point) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateSolidBrush(color: u32) -> Hbrush;
        fn DeleteObject(object: *mut c_void) -> i32;
    }

    #[link(name = "shell32")]
    extern "system" {
        fn Shell_NotifyIconW(message: u32, data: *const NotifyIconDataW) -> i32;
    }

    struct GuiState {
        context: AppContext,
        status: String,
        demo_job: Option<app_skeleton::DemoJob>,
        job_terminal_logged: bool,
    }

    static STATE: AtomicPtr<GuiState> = AtomicPtr::new(std::ptr::null_mut());
    static WAKE_MESSAGE: AtomicU32 = AtomicU32::new(0);

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn class_name() -> Vec<u16> {
        wide(&format!("{APP_ID}-MainWindow"))
    }

    fn state<'a>() -> Option<&'a mut GuiState> {
        let ptr = STATE.load(Ordering::SeqCst);
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { &mut *ptr })
        }
    }

    fn set_status(hwnd: Hwnd, text: String) {
        if let Some(state) = state() {
            state.status = text;
        }
        unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
    }

    fn show_main_window(hwnd: Hwnd) {
        unsafe {
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    }

    pub fn run() {
        unsafe {
            let wake_name = wide(&format!("{APP_ID}-Wake"));
            WAKE_MESSAGE.store(RegisterWindowMessageW(wake_name.as_ptr()), Ordering::SeqCst);

            // Single-instance: named mutex. Keep the handle open for
            // the life of the process (never closed on purpose).
            let mutex_name = wide(&format!(r"Local\{APP_ID}-SingleInstance"));
            let _mutex = CreateMutexW(std::ptr::null_mut(), 0, mutex_name.as_ptr());
            if GetLastError() == ERROR_ALREADY_EXISTS {
                signal_first_instance();
                return;
            }

            let context = match AppContext::start(true) {
                Ok(context) => context,
                Err(e) => {
                    show_error(&format!("Startup failed: {e}"));
                    std::process::exit(1);
                }
            };
            let start_x = context.settings().window_x;
            let start_y = context.settings().window_y;
            let initial_status = format!(
                "Ready — launch #{} ({} v{})",
                context.settings().launch_count,
                APP_NAME,
                VERSION
            );
            let gui = Box::new(GuiState {
                context,
                status: initial_status,
                demo_job: None,
                job_terminal_logged: false,
            });
            STATE.store(Box::into_raw(gui), Ordering::SeqCst);

            let instance = GetModuleHandleW(std::ptr::null());
            let class = class_name();
            let wnd_class = WndClassW {
                style: 0,
                wnd_proc: Some(wnd_proc),
                cls_extra: 0,
                wnd_extra: 0,
                instance,
                icon: LoadIconW(std::ptr::null_mut(), 32512 as *const u16),
                cursor: LoadCursorW(std::ptr::null_mut(), 32512 as *const u16),
                background: CreateSolidBrush(COLOR_BG),
                menu_name: std::ptr::null(),
                class_name: class.as_ptr(),
            };
            if RegisterClassW(&wnd_class) == 0 && GetLastError() != 1410 {
                show_error("RegisterClassW failed");
                cleanup_state();
                std::process::exit(1);
            }

            let title = wide(&format!("{APP_NAME} — AppHost skeleton"));
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                start_x,
                start_y,
                640,
                400,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null_mut(),
            );
            if hwnd.is_null() {
                show_error("CreateWindowExW failed");
                cleanup_state();
                std::process::exit(1);
            }

            ShowWindow(hwnd, SW_SHOW);
            let mut msg = Msg {
                hwnd: std::ptr::null_mut(),
                message: 0,
                w_param: 0,
                l_param: 0,
                time: 0,
                pt: Point::default(),
                l_private: 0,
            };
            loop {
                let result = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
                if result <= 0 {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            cleanup_state();
        }
    }

    unsafe fn signal_first_instance() {
        let class = class_name();
        let mut hwnd = FindWindowW(class.as_ptr(), std::ptr::null());
        if hwnd.is_null() {
            // The first instance may still be creating its window.
            std::thread::sleep(std::time::Duration::from_millis(300));
            hwnd = FindWindowW(class.as_ptr(), std::ptr::null());
        }
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
            let wake = WAKE_MESSAGE.load(Ordering::SeqCst);
            if wake != 0 {
                PostMessageW(hwnd, wake, 0, 0);
            }
        }
    }

    unsafe fn cleanup_state() {
        let ptr = STATE.swap(std::ptr::null_mut(), Ordering::SeqCst);
        if !ptr.is_null() {
            drop(Box::from_raw(ptr));
        }
    }

    fn show_error(text: &str) {
        let text_w = wide(text);
        let caption_w = wide(APP_NAME);
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text_w.as_ptr(),
                caption_w.as_ptr(),
                0x0000_0010, // MB_ICONERROR
            );
        }
    }

    unsafe fn add_tray_icon(hwnd: Hwnd) {
        let mut data = NotifyIconDataW {
            cb_size: 0,
            hwnd,
            id: 1,
            flags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            callback_message: WM_TRAY,
            icon: LoadIconW(std::ptr::null_mut(), 32512 as *const u16),
            tip: [0; 128],
            state: 0,
            state_mask: 0,
            info: [0; 256],
            timeout_or_version: 0,
            info_title: [0; 64],
            info_flags: 0,
            guid_item: Guid::default(),
            balloon_icon: std::ptr::null_mut(),
        };
        data.cb_size = std::mem::size_of::<NotifyIconDataW>() as u32;
        let tip = wide(APP_NAME);
        for (slot, ch) in data.tip.iter_mut().zip(tip.iter()) {
            *slot = *ch;
        }
        Shell_NotifyIconW(NIM_ADD, &data);
    }

    unsafe fn remove_tray_icon(hwnd: Hwnd) {
        let mut data = NotifyIconDataW {
            cb_size: 0,
            hwnd,
            id: 1,
            flags: 0,
            callback_message: 0,
            icon: std::ptr::null_mut(),
            tip: [0; 128],
            state: 0,
            state_mask: 0,
            info: [0; 256],
            timeout_or_version: 0,
            info_title: [0; 64],
            info_flags: 0,
            guid_item: Guid::default(),
            balloon_icon: std::ptr::null_mut(),
        };
        data.cb_size = std::mem::size_of::<NotifyIconDataW>() as u32;
        Shell_NotifyIconW(NIM_DELETE, &data);
    }

    unsafe fn show_tray_menu(hwnd: Hwnd) {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let items: [(usize, &str); 6] = [
            (ID_OPEN, "Open"),
            (0, ""),
            (ID_JOB, "Run demo job"),
            (ID_CHILD, "Run demo child process"),
            (ID_UPDATE, "Check for updates"),
            (ID_EXIT, "Exit"),
        ];
        for (id, label) in items {
            if id == 0 {
                AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
            } else {
                let text = wide(label);
                AppendMenuW(menu, MF_STRING, id, text.as_ptr());
            }
        }
        let mut point = Point::default();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        DestroyMenu(menu);
        if command > 0 {
            handle_action(hwnd, command as usize);
        }
    }

    fn handle_action(hwnd: Hwnd, id: usize) {
        match id {
            ID_OPEN => show_main_window(hwnd),
            ID_JOB => {
                let Some(state) = state() else { return };
                if state.demo_job.is_some() {
                    set_status(hwnd, "Demo job is already running".to_string());
                    return;
                }
                match state.context.start_demo_job() {
                    Ok(job) => {
                        state.demo_job = Some(job);
                        state.job_terminal_logged = false;
                        set_status(hwnd, "Demo job started".to_string());
                    }
                    Err(e) => set_status(hwnd, format!("Demo job failed to start: {e}")),
                }
            }
            ID_CHILD => {
                let Some(state) = state() else { return };
                match state.context.run_demo_child() {
                    Ok(result) => set_status(
                        hwnd,
                        format!(
                            "Demo child: state={} exit={:?} stdout={}",
                            result.state.name(),
                            result.exit_code,
                            result.stdout.trim()
                        ),
                    ),
                    Err(e) => set_status(hwnd, format!("Demo child failed: {e}")),
                }
            }
            ID_UPDATE => {
                let Some(state) = state() else { return };
                let outcome = state.context.check_for_updates();
                set_status(hwnd, outcome.summary());
            }
            ID_EXIT => unsafe {
                DestroyWindow(hwnd);
            },
            _ => {}
        }
    }

    fn poll_demo_job(hwnd: Hwnd) {
        let Some(gui) = state() else { return };
        let Some(job) = gui.demo_job.as_ref() else {
            return;
        };
        let status = job.status();
        let terminal = status.state.is_terminal();
        let text = jobs::format_status(&status);
        if terminal && !gui.job_terminal_logged {
            gui.job_terminal_logged = true;
            let _ = gui.context.logger().info(
                "jobs",
                &format!(
                    "demo job {} percent={} elapsed_ms={}",
                    status.state.as_str(),
                    status.percent,
                    status.elapsed_ms
                ),
            );
        }
        set_status(hwnd, text);
        if terminal {
            if let Some(mut finished) = state().and_then(|s| s.demo_job.take()) {
                finished.shutdown();
            }
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: Hwnd,
        msg: u32,
        w_param: Wparam,
        l_param: Lparam,
    ) -> Lresult {
        let wake = WAKE_MESSAGE.load(Ordering::SeqCst);
        if wake != 0 && msg == wake {
            show_main_window(hwnd);
            return 0;
        }

        match msg {
            0x0001 => {
                // WM_CREATE
                add_tray_icon(hwnd);
                SetTimer(hwnd, TIMER_ID, 100, None);
                0
            }
            WM_TRAY => {
                // Tray callback — handled HERE, in the wndproc (see
                // the module comment; never poll a channel for this).
                let mouse = l_param as u32;
                if mouse == WM_RBUTTONUP || mouse == WM_CONTEXTMENU {
                    show_tray_menu(hwnd);
                } else if mouse == WM_LBUTTONUP {
                    show_main_window(hwnd);
                }
                0
            }
            WM_COMMAND => {
                handle_action(hwnd, w_param & 0xFFFF);
                0
            }
            WM_SYSCOMMAND if (w_param & 0xFFF0) == SC_MINIMIZE => {
                ShowWindow(hwnd, SW_HIDE);
                0
            }
            WM_TIMER if w_param == TIMER_ID => {
                poll_demo_job(hwnd);
                0
            }
            WM_PAINT => {
                paint(hwnd);
                0
            }
            WM_CLOSE => {
                if let Some(state) = state() {
                    let mut rect = Rect::default();
                    if GetWindowRect(hwnd, &mut rect) != 0 {
                        let _ = state.context.save_window_position(rect.left, rect.top);
                    }
                }
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                remove_tray_icon(hwnd);
                KillTimer(hwnd, TIMER_ID);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, w_param, l_param),
        }
    }

    unsafe fn paint(hwnd: Hwnd) {
        let mut ps = PaintStruct {
            hdc: std::ptr::null_mut(),
            erase: 0,
            rc_paint: Rect::default(),
            restore: 0,
            inc_update: 0,
            reserved: [0; 32],
        };
        let hdc = BeginPaint(hwnd, &mut ps);
        if hdc.is_null() {
            return;
        }
        let mut client = Rect::default();
        GetClientRect(hwnd, &mut client);
        let brush = CreateSolidBrush(COLOR_BG);
        FillRect(hdc, &client, brush);
        DeleteObject(brush);

        SetBkMode(hdc, 1); // TRANSPARENT
        let title = wide(&format!("{APP_NAME} — AppHost skeleton"));
        let mut title_rect = Rect {
            left: 24,
            top: 20,
            right: client.right - 24,
            bottom: 56,
        };
        SetTextColor(hdc, COLOR_ACCENT);
        DrawTextW(hdc, title.as_ptr(), -1, &mut title_rect, 0);

        let status_text = state().map(|s| s.status.clone()).unwrap_or_default();
        let status = wide(&status_text);
        let mut status_rect = Rect {
            left: 24,
            top: 64,
            right: client.right - 24,
            bottom: client.bottom - 20,
        };
        SetTextColor(hdc, COLOR_TEXT);
        DrawTextW(hdc, status.as_ptr(), -1, &mut status_rect, 0);
        EndPaint(hwnd, &ps);
    }

    // Referenced so the compiler keeps the restored-size constant
    // documented beside the minimize handling above.
    #[allow(dead_code)]
    const _SIZE_RESTORED: usize = SIZE_RESTORED_WPARAM;
}
