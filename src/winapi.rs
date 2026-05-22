/**
 * 使用winapi创建窗口
 * Rust使用winapi与C++类似,由于Rust没有null定义,在必要使用null的时候可以用系统包std::ptr::null_mut函数进行null赋值
 */

#[cfg(windows)]
#[allow(non_snake_case)]
extern crate winapi;

#[cfg(windows)]
#[allow(non_snake_case)]
extern crate native_windows_gui as nwg;

use winapi::shared::minwindef::TRUE;
use winapi::shared::windef::POINT;
use winapi::um::winuser::{
    DispatchMessageW, GetMessageW, TranslateMessage, UpdateWindow, MSG, WS_OVERLAPPEDWINDOW,
};

use std::ptr::null_mut;

use winapi::um::wingdi::WHITE_BRUSH;
use winapi::um::{
    libloaderapi::GetModuleHandleW,
    wingdi::GetStockObject,
    winuser::{
        CreateWindowExW, DefWindowProcW, LoadCursorW, LoadIconW, RegisterClassW, ShowWindow,
        CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW, IDI_APPLICATION, SW_SHOW, WM_PAINT,
        WNDCLASSW, WS_EX_TOPMOST,
    },
};
use winapi::{
    shared::{
        minwindef::{LPARAM, LRESULT, UINT, WPARAM},
        windef::{HBRUSH, HWND},
    },
    um::winuser::{MessageBoxW, MB_ICONERROR},
};

fn main() {
    unsafe {
        create_window();
    }
}

#[allow(non_snake_case)]
unsafe fn create_window() {
    let instance = GetModuleHandleW(null_mut());
    let window_name_wide = text_wide("About3");
    let wnd_class = &WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(crid_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: LoadIconW(instance, IDI_APPLICATION),
        hCursor: LoadCursorW(instance, IDC_ARROW),
        hbrBackground: GetStockObject(WHITE_BRUSH as i32) as HBRUSH,
        lpszMenuName: window_name_wide,
        lpszClassName: window_name_wide,
    };
    if RegisterClassW(wnd_class) == (0 as u16) {
        MessageBoxW(
            null_mut(),
            text_wide("This Program requires Windows NT"),
            window_name_wide,
            MB_ICONERROR,
        );
        return;
    }
    let hw = CreateWindowExW(
        WS_EX_TOPMOST,
        window_name_wide,
        window_name_wide,
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        null_mut(),
        null_mut(),
        instance,
        null_mut(),
    );
    ShowWindow(hw, SW_SHOW);
    UpdateWindow(hw);

    let mut msg = MSG {
        hwnd: 0 as HWND,
        message: 0 as u32,
        wParam: 0 as usize,
        lParam: 0 as isize,
        time: 0 as u32,
        pt: POINT { x: 0, y: 0 },
    };
    while GetMessageW(&mut msg as *mut MSG, null_mut(), 0, 0) == TRUE {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

#[allow(non_snake_case)]
unsafe extern "system" fn crid_proc(
    hwnd: HWND,
    u_msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if u_msg == WM_PAINT {
        return 0;
    }
    return DefWindowProcW(hwnd, u_msg, w_param, l_param);
}

use std::{ffi::OsStr, iter::once, os::windows::prelude::OsStrExt};

/**
 * 宽字符转换: Windows使用的字符编码是宽字符，所以需要将源字符改成宽字符模式
 *
 * @param text 需要转换字符
 * @return 宽字符指针
 */
pub fn text_wide(text: &str) -> *const u16 {
    let win_text: Vec<u16> = OsStr::new(text).encode_wide().chain(once(0)).collect();
    return win_text.as_ptr();
}