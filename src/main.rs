use std::{env, io, mem};
use std::ffi::c_void;
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use reqwest;
use url::Url;
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, GetLastError};
// use winapi::um::winuser::{SystemParametersInfoW, SPI_SETDESKWALLPAPER, SPIF_UPDATEINIFILE, SPIF_SENDCHANGE};
use windows::Win32::UI::WindowsAndMessaging::{SPI_GETDESKWALLPAPER, SPI_SETDESKWALLPAPER, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoA, SystemParametersInfoW};
use windows::Win32::Foundation::TRUE;
use winreg::enums::*;
use winreg::RegKey;
use wallpaper;

// 下载必应每日一图的函数
async fn download_bing_wallpaper() -> Result<String, Box<dyn std::error::Error>> {
    // Bing 壁纸API的URL
    let api_url = "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=1&mkt=en-US";

    // 发起网络请求
    let res = reqwest::get(api_url).await?;
    let body = res.text().await?;
    let v: Value = serde_json::from_str(&body)?;
    let image_url = format!("https://www.bing.com{}", v["images"][0]["url"].as_str().unwrap());
    // 解析URL
    let parsed = Url::parse(&image_url).unwrap();
    // 获取查询参数
    /*for (key, value) in parsed.query_pairs() {
        println!("{}: {}", key, value);
    }*/
    let id = parsed.query_pairs().find(|(key, _)| key == "id").unwrap();
    println!("{:?}", id.1);
    let rf = parsed.query_pairs().find(|(key, _)| key == "rf").unwrap();
    println!("{:?}", rf.1);

    // 下载图片
    let response = reqwest::get(&image_url).await?;

    // 获取当前目录
    let current_dir = env::current_dir().expect("获取当前目录失败");
    // 获取文件的扩展名
    let ext = Path::new(rf.1.as_ref()).extension().and_then(|ext| ext.to_str()).unwrap_or("jpg");
    // 构建文件的绝对路径
    let file_path = current_dir.join("bing_wallpaper.".to_owned() + ext);
    let mut file = File::create(&file_path)?;
    let content = response.bytes().await?;
    file.write_all(&content)?;

    Ok(file_path.to_str().unwrap().to_string())
}

// 获取当前壁纸的函数
fn get_wallpaper() -> Result<String, Box<dyn std::error::Error>> {
    unsafe {
        let buffer: [u16; 260] = mem::zeroed();
        let result = SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buffer.len() as u32,
            Option::from(buffer.as_ptr() as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0u32),
        );

        if !result.is_err() {
            let path = String::from_utf16(&buffer)?
                // removes trailing zeroes from buffer
                .trim_end_matches('\x00')
                .into();
            Ok(path)
        } else {
            Err(io::Error::last_os_error().into())
        }
    }
}

// 设置壁纸的函数
fn set_wallpaper(image_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 通过系统调用设置壁纸
    println!("{:?}", get_wallpaper().unwrap());
    /*println!("{:?}", wallpaper::get());
    // 从文件路径设置当前桌面的壁纸。
    wallpaper::set_from_path(&image_path).unwrap();
    // 设置壁纸的样式，有填充、适应、拉伸、居中、裁剪等模式可选。
    wallpaper::set_mode(wallpaper::Mode::Crop).unwrap();
    // 从URL设置当前桌面的壁纸。
    // wallpaper::set_from_url(&image_path).unwrap();
    // 返回当前桌面的壁纸。
    println!("{:?}", wallpaper::get());*/
    unsafe {
        // 使用 ANSI 字符串版本
        // 使用CString来确保字符串结束于空字符
        /*let path = windows::core::PCSTR(image_path.as_ptr());
        let result = SystemParametersInfoA(
            SPI_SETDESKWALLPAPER,
            0,
            Option::from(path.as_ptr() as *mut c_void), // 图片路径
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE, // 变化是否应该被保存到用户的配置文件中
        );*/
        // 推荐使用的Unicode版本
        // 将Rust字符串转换为宽字符串，以匹配 SystemParametersInfoW 所需的格式
        let path: Vec<u16> = std::ffi::OsStr::new(image_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        // let path: Vec<u16> = image_path.encode_utf16().chain(std::iter::once(0)).collect();
        let result = SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            Option::from(path.as_ptr() as *mut c_void),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        );
        if result.is_err() { // 设置失败
            // 设置失败，检查错误码
            let error = GetLastError();
            if error == ERROR_ACCESS_DENIED.ok() {
                // 错误码表明权限不足
                Ok(false)
            } else {
                // 其他错误
                Err(windows::core::Error::from_win32())
            }
        } else {
            Ok(true)
        }
    }.expect("设置壁纸失败");

    Ok(())
}

// 注册开机启动的函数
fn add_to_startup(app_name: &str, app_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
    let (key, _disp) = hklm.create_subkey(&path)?;

    key.set_value(app_name, &app_path)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image_path = download_bing_wallpaper().await?;
    set_wallpaper(&image_path)?;
    // add_to_startup("", "")?;
    Ok(())
}