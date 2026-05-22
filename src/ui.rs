use std::cell::RefCell;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI;
use windows::Win32::UI::Shell::{ShellExecuteW, HELPINFO};
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxIndirectW, MessageBoxW, MB_DEFBUTTON2, MB_HELP, MB_ICONWARNING, MB_OK, MB_YESNO,
    MESSAGEBOX_STYLE, MSGBOXPARAMSW, SW_SHOWNORMAL,
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
