use dioxus::prelude::*;
use dioxus::desktop::{LogicalPosition, LogicalSize, tao::platform::windows::WindowExtWindows};

use ::windows::core::*;
use ::windows::Win32::Foundation::*;
use ::windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Shell::RemoveWindowSubclass;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT,
};
use ::windows::Win32::Graphics::Gdi::*;
use ::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use ::windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};


pub fn use_snap_layout<F: FnMut(MaxButtonEvent) + 'static>(
    relative_to: WindowCorner,
    offset: LogicalPosition<i32>,
    size: LogicalSize<i32>,
    hand_cursor: bool,
    mut event_handler: F
)  {

    let (hwnd, overlay)= use_hook(move || {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        // forward events from our stream to the event handler
        // calling the handler here:
        // - removes the Send bound to share with the window event functions
        // - allows it to run in dioxus's runtime context
        spawn(async move {
            loop {
                let Some(evt) = rx.recv().await else { break };
                (event_handler)(evt)
            }
        });

        let hwnd = HWND(dioxus::desktop::window().hwnd() as _);

        SNAP_LAYOUT_STATE.lock().unwrap().replace(SnapLayoutState {
            relative_to,
            offset,
            size,
            use_hand_cursor: hand_cursor,
            is_mouse_over: false,
            event_sink: tx,
        });
        (hwnd, unsafe {
             overlay_nchittest_window(
                hwnd
            )
        })
    });

    use_drop(move || {
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(subclass_proc), (WM_USER + 1) as _);
            let _ = DestroyWindow(overlay);
        }
        SNAP_LAYOUT_STATE.lock().unwrap().take();
    });
}


#[derive(Debug, Clone, Copy)]
pub enum WindowCorner {
    TopRight,
}

pub struct SnapLayoutState {
    relative_to: WindowCorner,
    offset: LogicalPosition<i32>,
    size: LogicalSize<i32>,
    use_hand_cursor: bool,
    is_mouse_over: bool,
    event_sink: tokio::sync::mpsc::Sender<MaxButtonEvent>
}

#[derive(Debug, Clone, Copy)]
pub enum MaxButtonEvent {
    LeftButtonDown,
    LeftButtonUp,
    MouseEnter,
    MouseLeave,
}

static SNAP_LAYOUT_STATE: Mutex<Option<SnapLayoutState>> = Mutex::new(None);

fn edit_state(f: impl FnOnce(&mut SnapLayoutState)) {
    let mut state_lock = SNAP_LAYOUT_STATE.lock().unwrap();
    state_lock.as_mut().map(f);
}

unsafe fn update_overlay_position(parent: HWND, child: HWND) {
    edit_state(|state| {
        let mut rect = RECT::default();
        GetClientRect(parent, &mut rect).unwrap();
        let width = rect.right - rect.left;

        let dpi = ::windows::Win32::UI::HiDpi::GetDpiForWindow(child);
        let scale_factor = dpi as f64 / 96.;

        let offset = state.offset.to_physical(scale_factor);
        
        let sz = state.size.to_physical(scale_factor);

        let (x, y) = match state.relative_to {
            WindowCorner::TopRight => (width - offset.x - sz.width, offset.y)
        };

        SetWindowPos(
            child,
            Some(HWND_TOP),
            x,
            y,
            sz.width,
            sz.height,
            // SWP_ASYNCWINDOWPOS, // | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOMOVE,
            SWP_ASYNCWINDOWPOS,
        )
        .unwrap();
    });
}

unsafe fn overlay_nchittest_window(parent: HWND) -> HWND {

    let class_name = w!("DRAG_WINDOW3");

    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: WNDCLASS_STYLES::default(),
        lpfnWndProc: Some(overlay_window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: unsafe { HINSTANCE(GetModuleHandleW(PCWSTR::null()).unwrap_or_default().0) },
        hIcon: HICON::default(),
        hCursor: HCURSOR::default(),
        hbrBackground: HBRUSH::default(),
        lpszMenuName: PCWSTR::null(),
        lpszClassName: class_name,
        hIconSm: HICON::default(),
    };

    RegisterClassExW(&class);

    let inst = GetModuleHandleW(PCWSTR::null()).unwrap_or_default().0;
    let inst = HINSTANCE(inst);

    let overlay_hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        class_name,
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_OVERLAPPED,
        0,
        0,
        0,
        0,
        Some(parent),
        Some(HMENU::default()),
        Some(inst),
        None,
    )
    .unwrap();

    update_overlay_position(parent, overlay_hwnd);

    SetWindowSubclass(
        parent,
        Some(subclass_proc),
        (WM_USER + 1) as _,
        overlay_hwnd.0 as _,
    )
    .unwrap();

    overlay_hwnd

}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    data_ptr: usize,
) -> LRESULT {
    // println!("subclass_proc msg {:?}", msg);
    let overlay_hwnd = HWND(data_ptr as _);
    match msg {
        WM_SIZE => {
            update_overlay_position(hwnd, overlay_hwnd);
        }

        _ => {}
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

use std::sync::Mutex;


unsafe extern "system" fn overlay_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // println!("overlay_window_proc msg {:?}", msg);
    match msg {
        WM_NCHITTEST => {
            // window covers exactly the button region, so no hit test is necessary
            return LRESULT(HTMAXBUTTON as _);
        }
        WM_NCLBUTTONDOWN => {
            edit_state(|state| { let _ = state.event_sink.try_send(MaxButtonEvent::LeftButtonDown); });
            return LRESULT(0);
        }
        WM_NCLBUTTONUP => {
            edit_state(|state| { let _ = state.event_sink.try_send(MaxButtonEvent::LeftButtonUp); });
            return LRESULT(0);
        }
        WM_NCMOUSEMOVE => {
            edit_state(|state| {
                if !state.is_mouse_over {
                    state.is_mouse_over = true;
                    let _ = state.event_sink.try_send(MaxButtonEvent::MouseEnter);
                }
            });
            let mut track = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE | TME_NONCLIENT,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            let _ = TrackMouseEvent(&mut track as *mut _);
            return LRESULT(0);
        }
        WM_NCMOUSELEAVE => {
            edit_state(|state| {
                state.is_mouse_over = false;
                let _ = state.event_sink.try_send(MaxButtonEvent::MouseLeave);
            });
        }
        WM_SETCURSOR => {
            let state_lock = SNAP_LAYOUT_STATE.lock().unwrap();
            if state_lock.as_ref().map(|state| state.use_hand_cursor).unwrap_or(false) {
                let _ = SetCursor(Some(LoadCursorW(None, IDC_HAND).unwrap()));
                return LRESULT(1);
            }
        }
        _ => {}
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

