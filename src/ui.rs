use std::cell::RefCell;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    RedrawWindow, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW,
};
use windows::Win32::UI;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_F5;
use windows::Win32::UI::Shell::{
    SHChangeNotify, ShellExecuteW, HELPINFO, SHCNE_ASSOCCHANGED, SHCNF_IDLIST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, MessageBoxIndirectW, MessageBoxW, PostMessageW,
    SystemParametersInfoW, MB_DEFBUTTON2, MB_HELP, MB_ICONWARNING, MB_OK, MB_YESNO,
    MESSAGEBOX_STYLE, MSGBOXPARAMSW, NONCLIENTMETRICSW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE,
    SPI_GETNONCLIENTMETRICS, SPI_SETNONCLIENTMETRICS, SW_SHOWNORMAL,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WM_KEYDOWN, WM_KEYUP,
};

/// 显示一个确定按钮的消息框
pub fn show_info(text: &str, caption: &str) {
    unsafe {
        // https://kennykerr.ca/rust-getting-started/string-tutorial.html
        MessageBoxW(
            None,
            &HSTRING::from(text),
            &HSTRING::from(caption),
            MB_OK, // 等价于 MESSAGEBOX_STYLE(0)
        );
    }
}

/// 显示错误消息框
pub fn show_error(text: &str) {
    show_info(text, "错误");
}

/// 显示自定义按钮样式的消息框
pub fn show_message(text: &str, caption: &str, style: MESSAGEBOX_STYLE) {
    unsafe {
        MessageBoxW(None, &HSTRING::from(text), &HSTRING::from(caption), style);
    }
}

// ── 线程本地存储：帮助回调需要知道当前 URL ──
thread_local! {
    static CURRENT_HELP_URL: RefCell<String> = RefCell::new(String::new());
}

// ── 内部回调（不对外暴露） ──
unsafe extern "system" fn help_callback(lphelpinfo: *mut HELPINFO) {
    if lphelpinfo.is_null() {
        return;
    }
    // if let Some(info) = lphelpinfo.as_ref() {
    // 根据 dwContextId 决定打开哪个页面
    /*let url = match (*lphelpinfo).dwContextId {
        1001 => w!("https://github.com/bajins/desktop-wallpaper-rust"),
        1002 => w!("https://github.com/bajins/desktop-wallpaper-rust"),
        _ => w!("https://github.com/bajins/desktop-wallpaper-rust"),
    };*/

    CURRENT_HELP_URL.with(|url| {
        let url_str = url.borrow();
        let url_h = HSTRING::from(url_str.as_str());

        // ShellExecuteW 打开 URL
        let se = ShellExecuteW(
            None,                   // 窗口句柄
            w!("open"),             // 操作 (打开)
            PCWSTR(url_h.as_ptr()), // 要打开的URL/文件
            None,                   // 参数
            None,                   // lpDirectory
            SW_SHOWNORMAL,          // 显示状态
        );

        if se.is_invalid() {
            show_error("打开帮助失败")
        }
    });
}

// ── 对外暴露的枚举 ──
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogResult {
    Yes,
    No,
    Cancel,
    Other(i32),
}

/*
match ui::ask_yes_no_with_help(
    "确定要设置任务吗？",
    "警告",
    "https://github.com/bajins/desktop-wallpaper-rust",
) {
    ui::DialogResult::Yes => {
        if let Err(e) = create_schedule() {
            ui::show_error(&e.to_string());
            return Err(e);
        }
    }
    ui::DialogResult::No => println!("用户点击了 否"),
    other => println!("其他: {:?}", other),
}
*/
/// 显示带帮助按钮的是/否对话框
pub fn ask_yes_no_with_help(text: &str, caption: &str, help_url: &str) -> DialogResult {
    // 把 URL 写入线程本地存储，供回调读取
    CURRENT_HELP_URL.with(|url| {
        *url.borrow_mut() = help_url.to_string();
    });

    let params = MSGBOXPARAMSW {
        cbSize: size_of::<MSGBOXPARAMSW>() as u32, // 识别结构体版本
        hwndOwner: HWND::default(),
        hInstance: HINSTANCE::default(), // 自定义图标/字符串资源所在的模块句柄
        lpszText: PCWSTR(HSTRING::from(text).as_ptr()), // 内容
        lpszCaption: PCWSTR(HSTRING::from(caption).as_ptr()), // 标题
        dwStyle: MB_YESNO
            // | MB_YESNOCANCEL
            | MB_HELP
            | MB_ICONWARNING
            // | MB_OKCANCEL
            // | MB_ICONQUESTION
            | MB_DEFBUTTON2, // 按钮组合 + 图标 + 模态标志，默认选中"否"
        lpszIcon: PCWSTR::null(),        // 自定义图标资源名
        dwContextHelpId: 1001,           // 帮助上下文 ID
        lpfnMsgBoxCallback: Some(help_callback), // 帮助回调函数指针
        dwLanguageId: 0, // 预定义按钮的语言（如 LANG_ENGLISH、LANG_CHINESE_SIMPLIFIED）
    };

    let result = unsafe { MessageBoxIndirectW(&params) };

    // IDOK	1	确定
    // IDCANCEL	2	取消
    // IDABORT	3	中止
    // IDRETRY	4	重试
    // IDIGNORE	5	忽略
    // IDYES	6	是
    // IDNO	7	否
    // IDTRYAGAIN	10	重试
    // IDCONTINUE	11	继续
    match result {
        UI::WindowsAndMessaging::IDYES => DialogResult::Yes,
        UI::WindowsAndMessaging::IDNO => DialogResult::No,
        UI::WindowsAndMessaging::IDCANCEL => DialogResult::Cancel,
        _ => DialogResult::Other(result.0),
    }
}

/// 高层封装：显示确认框 → 用户点"是"则执行闭包，出错时自动弹错误框
pub fn confirm_and_run(
    text: &str,
    caption: &str,
    help_url: &str,
    on_yes: impl FnOnce() -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    match ask_yes_no_with_help(text, caption, help_url) {
        DialogResult::Yes => {
            if let Err(e) = on_yes() {
                show_error(&e.to_string());
                return Err(e);
            }
        }
        DialogResult::No => {
            println!("用户点击了 否");
        }
        other => {
            println!("其他: {:?}", other);
        }
    }
    Ok(())
}

/// 等价于在桌面右键菜单点击"刷新"——通知 Shell 重新枚举桌面图标（桌面图标重新枚举、图标缓存重读、图标/关联变更）
/// SHChangeNotify 的 SHCNE_ASSOCCHANGED 事件会让 Shell 重读注册表并刷新桌面图标
/// https://helgeklein.com/blog/free-tool-refresh-the-desktop-programmatically
/// https://github.com/helgeklein
pub fn refresh_desktop_shcn() {
    unsafe {
        // SHChangeNotify 签名:
        //   weventid: SHCNE_ID, uflags: SHCNF_FLAGS,
        //   dwitem1/dwitem2: Option<*const c_void>
        //  0x8000000 即 SHCNE_ASSOCCHANGED、0x1000 即 SHCNF_IDLIST
        // 如果只想刷新某个具体目录（而非整个桌面），可改用 SHCNE_UPDATEDIR 并把路径以 PIDL 形式传入 dwitem1
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
}

/// 查找承载桌面图标 ListView 的窗口（定位桌面图标 ListView 的父窗口 SHELLDLL_DefView）:
///    Progman -> SHELLDLL_DefView (常规)
///    WorkerW -> SHELLDLL_DefView (壁纸轮换/某些主题下)
fn find_desktop_defview() -> Option<HWND> {
    unsafe {
        // 1) Progman -> SHELLDLL_DefView
        if let Ok(progman) = FindWindowW(w!("Progman"), None) {
            if let Ok(dv) = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None) {
                return Some(dv);
            }
        }

        // 2) 退化路径: 遍历所有 WorkerW，查找其下的 SHELLDLL_DefView
        let mut worker: Option<HWND> = None;
        loop {
            worker = FindWindowExW(None, worker, w!("WorkerW"), None).ok();
            match worker {
                Some(wnd) => {
                    if let Ok(dv) = FindWindowExW(Some(wnd), None, w!("SHELLDLL_DefView"), None) {
                        return Some(dv);
                    }
                }
                None => break, // 枚举结束
            }
        }
        None
    }
}

/// 模拟在桌面上按下并释放 F5
/// 定位 Progman → SHELLDLL_DefView，再 PostMessage 一对 WM_KEYDOWN/WM_KEYUP，虚拟键 VK_F5
///
/// 必须用 PostMessage 而非 SendMessage——后者会同步等待窗口过程处理，对桌面窗口经常无效。
///
/// 开启壁纸轮换时 SHELLDLL_DefView 不在 Progman 下而在某个 WorkerW 下，需要遍历 WorkerW 才能找到。
///
/// 不要把消息发到 Progman 自身或 SysListView32（SHELLDLL_DefView 的子窗口）——F5 必须投递给 SHELLDLL_DefView，由它转发给 ListView，否则可能无反应
pub fn refresh_desktop_f5() {
    unsafe {
        let Some(hwnd) = find_desktop_defview() else {
            return;
        };
        // const VK_F5: u32 = 0x74;
        // VK_F5 是 VIRTUAL_KEY 新类型，转成 WPARAM
        let wp = WPARAM(VK_F5.0 as usize);
        // lParam: bit31=转换状态(0=按下), bit30=前一状态
        //   KEYDOWN: 0x00000001 (repeat count=1)
        //   KEYUP  : 0xC0000001 (release + previous down)
        let _ = PostMessageW(Some(hwnd), WM_KEYDOWN, wp, LPARAM(0x0000_0001));
        let _ = PostMessageW(Some(hwnd), WM_KEYUP, wp, LPARAM(0xC000_0001));
    }
}

/// 直接对桌面窗口树做失效+立即重绘
/// 与 F5 的差别——只重绘,不触发 Shell 重读图标/注册表
pub fn redraw_desktop() {
    unsafe {
        let Some(hwnd) = find_desktop_defview() else {
            return;
        };
        let flags = RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_ERASE | RDW_UPDATENOW;
        let _ = RedrawWindow(Some(hwnd), None, None, flags);
    }
}

/// 通过 "读出-原样写回" 非客户区度量来强制桌面整体重绘
///
/// 先用 SPI_GETNONCLIENTMETRICS 读出当前的非客户区度量，原样不改地再用 SPI_SETNONCLIENTMETRICS 写回去——系统在 set 路径上无法判断值是否变化，会强制重算所有窗口的非客户区并触发一次桌面级重绘，从而附带刷新桌面图标区域
///
/// 副作用: 所有已打开窗口的非客户区（所有窗口标题栏/菜单/边框）都会重绘一次，会有可见闪烁（会写一次用户配置文件）
pub fn refresh_desktop_via_ncm() -> windows::core::Result<()> {
    unsafe {
        // 1) 读出当前 NONCLIENTMETRICS，cbSize 必须先填好
        let mut ncm: NONCLIENTMETRICSW = std::mem::zeroed();
        ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;

        SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
            Some(&mut ncm as *mut _ as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )?;

        // 2) 原样写回；fWinIni 带 SPIF_UPDATEINIFILE 写用户配置 + SPIF_SENDCHANGE 广播 WM_SETTINGCHANGE
        let flags = SPIF_UPDATEINIFILE | SPIF_SENDCHANGE;
        SystemParametersInfoW(
            SPI_SETNONCLIENTMETRICS,
            std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
            Some(&mut ncm as *mut _ as *mut _),
            flags,
        )
    }
}
