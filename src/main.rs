// #!必须放在头部
// https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute
// #![windows_subsystem = "windows"]

use std::{env, fs, io, mem};
use std::error::Error;
use std::ffi::c_void;
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::null_mut;
use std::rc::Rc;
use reqwest;
use reqwest::{Client, header, RequestBuilder, StatusCode};
use url::Url;
// use winapi::um::winuser::{SystemParametersInfoW, SPI_SETDESKWALLPAPER, SPIF_UPDATEINIFILE, SPIF_SENDCHANGE};
use windows::core::{BSTR, GUID, HSTRING, Interface, Type, VARIANT};
use windows::System::UserProfile::{IUserProfilePersonalizationSettingsStatics, LockScreen, UserProfilePersonalizationSettings};
use windows::Storage::{IStorageFile, StorageFile};
use windows::Win32::UI::WindowsAndMessaging::{SPI_GETDESKWALLPAPER, SPI_SETDESKWALLPAPER, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoA, SystemParametersInfoW};
use windows::Win32::UI::Shell::{SHGetDesktopFolder, IDesktopWallpaper, DESKTOP_WALLPAPER_POSITION, DWPOS_FILL};
use windows::Win32::Foundation::{BOOL, BOOLEAN, E_INVALIDARG, S_OK, ERROR_ACCESS_DENIED, ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS, FILETIME, GetLastError, TRUE, VARIANT_BOOL, VARIANT_FALSE, VARIANT_TRUE, GENERIC_READ, GENERIC_WRITE};
use windows::Win32::System::TaskScheduler::{IAction, IActionCollection, IBootTrigger, IDailyTrigger, IEventTrigger, IExecAction, IIdleTrigger, ILogonTrigger, IMonthlyDOWTrigger, IMonthlyTrigger, INetworkSettings, IPrincipal, IRegistrationInfo, IRegistrationTrigger, IRepetitionPattern, ITaskDefinition, ITaskFolder, ITaskService, ITaskSettings, ITimeTrigger, ITrigger, ITriggerCollection, IWeeklyTrigger, TaskScheduler, TASK_ACTION_EXEC, TASK_LOGON_TYPE, TASK_RUNLEVEL_TYPE, TASK_TRIGGER_BOOT, TASK_TRIGGER_DAILY, TASK_TRIGGER_EVENT, TASK_TRIGGER_IDLE, TASK_TRIGGER_LOGON, TASK_TRIGGER_MONTHLY, TASK_TRIGGER_MONTHLYDOW, TASK_TRIGGER_REGISTRATION, TASK_TRIGGER_TIME, TASK_TRIGGER_WEEKLY, TASK_LOGON_INTERACTIVE_TOKEN, TASK_TRIGGER_TYPE2, TASK_TRIGGER_SESSION_STATE_CHANGE, TASK_CREATE_OR_UPDATE, ISessionStateChangeTrigger, TASK_SESSION_STATE_CHANGE_TYPE, TASK_SESSION_UNLOCK, TASK_TRIGGER_CUSTOM_TRIGGER_01, ITaskTrigger, TASK_RUNLEVEL_HIGHEST, TASK_INSTANCES_IGNORE_NEW, ITaskSettings2, ITaskScheduler};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, CoCreateInstance, CLSCTX_ALL, COINIT_MULTITHREADED, COINIT_APARTMENTTHREADED, CLSCTX_INPROC_SERVER, IStream};
use windows::Win32::System::Variant::{VariantClear, VariantInit};
use windows::Win32::System::SystemInformation::*;
use windows::Win32::System::EventNotificationService::IsNetworkAlive;
use windows::Win32::System::WinRT::*;
use windows::core::PCWSTR;
use windows::Win32::NetworkManagement::*;
use windows::Win32::NetworkManagement::WNet::*;
use windows::Win32::NetworkManagement::IpHelper::{GetNetworkParams, FIXED_INFO_W2KSP1, IP_ADAPTER_INFO, GetAdaptersInfo};
use windows::Win32::Networking::*;
use windows::Win32::Networking::WinInet::{InternetGetConnectedState, INTERNET_CONNECTION_LAN, INTERNET_CONNECTION_MODEM, INTERNET_CONNECTION_PROXY, INTERNET_RAS_INSTALLED, INTERNET_CONNECTION, InternetCheckConnectionW, FLAG_ICC_FORCE_CONNECTION};
use windows::Win32::Networking::WinSock::{AF_UNSPEC, IPPROTO_IP, SOCK_STREAM, SOCKET_ERROR, WSASocketW};
use windows::Win32::Networking::NetworkListManager::{INetworkListManager, NetworkListManager, NLM_CONNECTIVITY, NLM_CONNECTIVITY_DISCONNECTED};
use windows::Win32::Storage::Packaging::Opc::*;
use windows::core::imp::PROPVARIANT;
use windows::Win32::Graphics::Imaging::{CLSID_WICImagingFactory, GUID_ContainerFormatJpeg, GUID_WICPixelFormat24bppBGR, IWICBitmapDecoder, IWICBitmapEncoder, IWICBitmapFrameDecode, IWICBitmapFrameEncode, IWICImagingFactory, WICBitmapCreateCacheOption, WICBitmapDitherTypeNone, WICBitmapNoCache, WICDecodeMetadataCacheOnDemand};
use windows::Win32::Storage::FileSystem::{CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_WRITE, FILE_SHARE_READ};
use windows::Win32::System::Com::StructuredStorage::IPropertyBag2;
use winreg::enums::*;
use winreg::RegKey;
use wallpaper;
use clap::{arg, command};
use rand::Rng;
use scraper::{Html, Selector};
use windows::Win32::Security::SECURITY_ATTRIBUTES;

// 下载必应每日一图
async fn get_bing_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=1&mkt=en-US";
    // 发起网络请求
    let res = reqwest::get(api_url).await?;
    let body = res.text().await?;
    println!("{:?}", body);
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

    Ok((image_url, rf.1.to_string()))
}

// 获取Windows Spotlight壁纸
async fn get_spotlight_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://arc.msn.com/v3/Delivery/Placement?pid=209567&fmt=json&cdm=1&pl=zh-CN&lc=zh-CN&ctry=CN";
    // 发起网络请求
    let res = reqwest::get(api_url).await?;
    let body = res.text().await?;
    println!("{:?}", body);
    let v: Value = serde_json::from_str(&body)?;
    let item: Value = serde_json::from_str(v["batchrsp"]["items"][0]["item"].as_str().unwrap())?;
    let image_url = item["ad"]["image_fullscreen_001_landscape"]["u"].as_str().unwrap();

    println!("{:?}", item["ad"]["hs1_title_text"]["tx"]);

    Ok((image_url.to_string(), String::from("")))
}

// 获取Edge Chromium壁纸
async fn get_edge_chromium_image_url() -> Result<(String, String), Box<dyn Error>> {
    //
    let api_url = "https://ntp.msn.com/edge/ntp?locale=zh-cn";
    // 发起网络请求
    let client = Client::new();
    // 创建一个请求构建器
    let mut builder: RequestBuilder = client.get(api_url);
    // 设置请求头
    builder = builder.header(header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36 Edg/123.0.0.0");
    // 发送请求并获取响应
    let response = builder.send().await?;
    // 检查响应状态码
    if response.status() != StatusCode::OK {
        return Err(Box::new(io::Error::new(io::ErrorKind::Other, "请求获取版本失败")));
    }
    let body = response.text().await?;
    // println!("{:?}", body);
    // 解析HTML
    let document = Html::parse_document(&*body);
    let head_selector = Selector::parse("head").unwrap();
    let head = document.select(&head_selector).next().unwrap();
    let dcs = head.value().attr("data-client-settings").unwrap();
    if dcs.is_empty() {
        return Err(Box::new(io::Error::new(io::ErrorKind::Other, "解析HTML并获取版本信息失败")));
    }
    // 解析JSON
    let body_json: Value = serde_json::from_str(&dcs)?;
    println!("{:?}", body_json);
    let version = body_json["bundleInfo"]["v"].as_str().unwrap();

    // 壁纸API的URL
    let api_url = "https://assets.msn.cn/resolver/api/resolve/v3/config/\
    ?expType=AppConfig&expInstance=default&apptype=edgeChromium&v=".to_owned() + version;
    // 发起网络请求
    let res = reqwest::get(api_url).await?;
    let body = res.text().await?;
    let v: Value = serde_json::from_str(&body)?;
    let datas = v["configs"]["BackgroundImageWC/default"]["properties"]["cmsImage"]["data"]
        .as_array().unwrap();
    println!("{:?}", datas);
    // 随机获取一张图片
    let mut rng = rand::thread_rng();
    let num = rng.gen_range(0..datas.len());
    let data_map = datas[num]["image"].as_object().unwrap();
    // 获取分辨率最大的图片
    let image = data_map.iter()
        .max_by_key(|(key, _value)| key.to_string()[1..].parse::<i64>().unwrap())
        .map(|(key, _value)| _value);
    // 获取图片的URL
    let mut image_url = v["configs"]["StickyPeek/default"]["properties"]
        ["stickyPeekLightCoachmarkMainImageURL"].as_str().unwrap();
    // 截取URL的路径
    match image_url.rfind("/") {
        Some(index) => image_url = &image_url[0..index + 1],
        None => println!("Substring not found")
    }
    // 拼接图片的URL
    let image_url = format!("{}{}", image_url, image.unwrap().as_str().unwrap());

    println!("{:?}", data_map);
    println!("{:?}", image_url);

    Ok((image_url.to_string(), String::from("")))
}

// 获取Pixabay壁纸
async fn get_pixabay_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://pixabay.com/api/?key=30271602-41319186b7198e7712c568e90&lang=zh&editors_choice=true";
    // 发起网络请求
    let res = reqwest::get(api_url).await?;
    let body = res.text().await?;
    println!("{:?}", body);
    let v: Value = serde_json::from_str(&body)?;
    let img_infos = v["hits"].as_array().unwrap();
    // 随机获取一张图片
    let mut rng = rand::thread_rng();
    let num = rng.gen_range(0..img_infos.len());
    let image_url = img_infos[num]["largeImageURL"].as_str().unwrap();

    println!("{:?}", image_url);

    Ok((image_url.to_string(), String::from("")))
}

// 获取金山词霸壁纸
async fn get_iciba_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://open.iciba.com/dsapi";
    // 发起网络请求
    let res = reqwest::get(api_url).await?;
    let body = res.text().await?;
    println!("{:?}", body);
    let v: Value = serde_json::from_str(&body)?;
    let image_url = v["picture2"].as_str().unwrap();

    println!("{:?}", image_url);

    Ok((image_url.to_string(), String::from("")))
}

// 获取Alphacoders壁纸
async fn get_alphacoders_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://alphacoders.com/nature-4k-wallpapers";
    // 发起网络请求
    let client = Client::new();
    // 创建一个请求构建器
    let mut builder: RequestBuilder = client.get(api_url);
    // 定义请求头
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"),
    );
    headers.insert(
        header::ACCEPT_ENCODING,
        header::HeaderValue::from_static("gzip, deflate, br, zstd"),
    );
    headers.insert(
        header::ACCEPT_LANGUAGE,
        header::HeaderValue::from_static("zh-CN,zh;q=0.9"),
    );
    headers.insert(
        header::HeaderName::from_static("sec-ch-ua"),
        header::HeaderValue::from_static(r#""Microsoft Edge";v="123", "Not:A-Brand";v="8", "Chromium";v="123""#),
    );
    headers.insert(
        header::HeaderName::from_static("sec-ch-ua-mobile"),
        header::HeaderValue::from_static(r#"?0"#),
    );
    headers.insert(
        header::HeaderName::from_static("sec-ch-ua-platform"),
        header::HeaderValue::from_static(r#""Windows""#),
    );
    headers.insert(
        header::HeaderName::from_static("sec-fetch-dest"),
        header::HeaderValue::from_static(r#"document"#),
    );
    headers.insert(
        header::HeaderName::from_static("sec-fetch-mode"),
        header::HeaderValue::from_static(r#"navigate"#),
    );
    headers.insert(
        header::HeaderName::from_static("sec-fetch-site"),
        header::HeaderValue::from_static(r#"none"#),
    );
    headers.insert(
        header::HeaderName::from_static("sec-fetch-user"),
        header::HeaderValue::from_static(r#"?1"#),
    );
    headers.insert(
        header::HeaderName::from_static("upgrade-insecure-requests"),
        header::HeaderValue::from_static(r#"1"#),
    );
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36 Edg/123.0.0.0"),
    );
    // 设置请求头
    builder = builder.headers(headers);
    // 发送请求并获取响应
    let response = builder.send().await?;
    // 检查响应状态码
    /*if response.status() != StatusCode::OK {
        return Err(Box::new(io::Error::new(io::ErrorKind::Other, "请求页面失败")));
    }*/
    let body = response.text().await?;
    println!("{:?}", body);
    // 解析HTML
    let document = Html::parse_document(&*body);
    let a_selector = Selector::parse("div.center a").unwrap();
    let img_selector = Selector::parse("picture img").unwrap();
    let img: Vec<_> = document.select(&img_selector).collect();
    println!("{:?}", img);
    let mut img_ids: Vec<String> = Vec::new();
    // 遍历符合父元素选择器条件的元素
    for a in document.select(&a_selector) {
        let image_url = a.value().attr("href").unwrap();
        println!("{}", image_url);
        // 解析URL
        let parsed = Url::parse(&image_url).unwrap();
        // 获取查询参数
        /*for (key, value) in parsed.query_pairs() {
            println!("{}: {}", key, value);
        }*/
        let id = parsed.query_pairs().find(|(key, _)| key == "i").unwrap();
        println!("{:?}", id.1);
        img_ids.push(id.1.to_string());
    }
    if img_ids.is_empty() {
        return Err(Box::new(io::Error::new(io::ErrorKind::Other, "解析HTML并获取版本信息失败")));
    }
    // 随机获取一张图片
    let mut rng = rand::thread_rng();
    let num = rng.gen_range(0..img_ids.len());
    let image_id = img_ids[num].as_str();

    let image_url = format!("https://initiate.alphacoders.com/download/images6/{}/png", image_id);
    println!("{:?}", image_url);

    Ok((image_url.to_string(), String::from("")))
}

// 获取NASA壁纸
async fn get_nasa_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    // https://apod.nasa.gov/apod
    // let formatted_date = info.date.format("%Y-%m-%d").to_string();
    // &date={formatted_date}
    let api_url = "https://api.nasa.gov/planetary/apod?api_key=DEMO_KEY";
    // 创建一个允许所有证书的信任锚
    /*let mut trust_anchor = RootCertStore::empty();
    trust_anchor.add_server_trust_anchors(&[TrustAnchor::from_pem(include_bytes!("any_certificate.pem"))?]);
    // 创建一个TLS配置，忽略证书验证
    let tls_builder = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(trust_anchor);*/
    // 创建一个允许所有证书的reqwest客户端
    let client = Client::builder()
        .danger_accept_invalid_certs(true) // 接受无效主机名
        // .use_rustls_tls()
        // .rustls_client_config(tls_builder.build())
        /*.add_root_certificate({
            let file = File::open("any_certificate.pem")?;
            let reader = BufReader::new(file);
            Certificate::from_pem(reader)
        }) // 添加自定义 CA 证书*/
        .build()?;
    // 发起网络请求
    let res = client.get(api_url).send().await?;
    // 检查响应状态码
    match res.status() {
        StatusCode::OK => {
            // 处理成功的响应
            println!("Response OK");
        }
        status => {
            // 处理错误或非200响应
            println!("Unexpected status code: {}", status);
        }
    }
    let body = res.text().await?;
    println!("{:?}", body);
    let v: Value = serde_json::from_str(&body)?;
    let media_type = v["media_type"].as_str().unwrap();
    if media_type != "image" {
        return Err(Box::new(io::Error::new(io::ErrorKind::Other, format!("媒体类型不是图片: {media_type}"))));
    }
    let image_url = v["hdurl"].as_str().unwrap();

    println!("{:?}", image_url);

    Ok((image_url.to_string(), String::from("")))
}

// 下载壁纸图片
async fn download_image() -> Result<String, Box<dyn std::error::Error>> {
    // 获取图片的URL
    let image_url;
    let file_name;
    let mut rng = rand::thread_rng();
    let num = rng.gen_range(0..7);
    if num == 1 {
        (image_url, file_name) = get_spotlight_image_url().await?;
    } else if num == 2 {
        (image_url, file_name) = get_edge_chromium_image_url().await?;
        /*} else if num == 3 {
            (image_url, file_name) = get_pixabay_image_url().await?;*/
    } else if num == 4 {
        (image_url, file_name) = get_iciba_image_url().await?;
        /*} else if num == 5 {
            (image_url, file_name) = get_alphacoders_image_url().await?;*/
    } else if num == 6 {
        (image_url, file_name) = get_nasa_image_url().await?;
    } else {
        (image_url, file_name) = get_bing_image_url().await?;
    }
    // 下载图片
    let response = reqwest::get(&image_url).await?;

    // 获取当前目录
    let current_dir = env::current_dir().expect("获取当前目录失败");
    // 获取文件的扩展名
    let ext = Path::new(&file_name).extension().and_then(|ext| ext.to_str()).unwrap_or("jpg");
    // 构建文件的绝对路径
    let file_path = current_dir.join("bing_wallpaper.".to_owned() + ext);
    // 创建文件
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

// Windows Imaging Component (WIC) 将图像转码为 Windows 11 中 TranscodedWallpaper 文件（JPEG 格式）
// https://learn.microsoft.com/zh-cn/windows/win32/wic/-wic-about-windows-imaging-codec
fn wic_codec() -> windows::core::Result<()> {
    /*unsafe {
        // 初始化 COM
        CoInitializeEx(None, COINIT_MULTITHREADED)?;

        // 创建 WIC 工厂
        let factory: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

        // 创建输入图像解码器
        let input_file_path = "input_image.png";
        let decoder: IWICBitmapDecoder = factory.CreateDecoderFromFilename(&input_file_path, None, GENERIC_READ, WICDecodeMetadataCacheOnDemand)?;

        // 获取第一帧
        let frame_index = 0;
        let frame_decode: IWICBitmapFrameDecode = decoder.GetFrame(frame_index)?;

        // 获取输入帧像素格式
        let input_pixel_format = frame_decode.GetPixelFormat()?;

        // 定义目标像素格式（JPEG 通常使用 24 位 BGR）
        let desired_pixel_format = GUID_WICPixelFormat24bppBGR;

        // 如果需要，转换像素格式
        let source_to_write: Box<dyn windows::Win32::Graphics::Imaging::IWICBitmapSource> = if input_pixel_format == desired_pixel_format {
            frame_decode
        } else {
            let converter = factory.CreateFormatConverter()?;
            converter.Initialize(frame_decode, desired_pixel_format, WICBitmapDitherTypeNone, None, 0.0f32 as f64, WICBitmapPalettesMedianCut)?;
            converter
        };

        // 创建 OPC Factory
        let opc_factory: IOpcFactory = CoCreateInstance(
            &OpcFactory,
            None,
            CLSCTX_ALL,
        )?;
        // 创建输出 JPEG 文件的编码器
        let output_file_path = "output_image.jpg";
        // 调用 CreateStreamOnFile
        let mut stream = None;
        let stream: IStream = opc_factory.CreateStreamOnFile(
            &output_file_path,
            OPC_STREAM_IO_WRITE, // 写入模式
            SECURITY_ATTRIBUTES::default(),                // 安全属性默认
            FILE_ATTRIBUTE_NORMAL,
        );
        let encoder: IWICBitmapEncoder = factory.CreateEncoder(&GUID_ContainerFormatJpeg, GUID::default() as *const GUID)?;
        encoder.Initialize(stream, WICBitmapNoCache)?;

        // 创建新帧
        let frame_encode: IWICBitmapFrameEncode = encoder.CreateNewFrame((), ())?;

        // 设置属性包，设置图像质量为 85（0.85）
        // https://learn.microsoft.com/zh-cn/windows/win32/wic/-wic-creating-encoder
        let property_bag: IPropertyBag2 = frame_encode.GetPropertyBag2().unwrap();
        let mut var_quality = PROPVARIANT { vt: VT_R4, fltVal: 0.85f32, ..Default::default() };
        property_bag.Write(L"ImageQuality", &var_quality)?;

        // 设置帧大小
        let (width, height) = source_to_write.GetSize().unwrap();
        frame_encode.SetSize(width, height)?;

        // 写入源图像到帧
        frame_encode.WriteSource(source_to_write, None)?;

        // 提交帧和编码器
        frame_encode.Commit()?;
        encoder.Commit()?;

        // 清理
        CoUninitialize();
    }*/
    Ok(())
}

// 设置壁纸的函数
fn set_wallpaper(image_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 通过系统调用设置壁纸
    println!("{:?}", get_wallpaper()?);
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
            if error == ERROR_ACCESS_DENIED {
                // 错误码表明权限不足
                Ok(false)
            } else {
                // 其他错误
                Err(windows::core::Error::from_win32())
            }
        } else {
            Ok(true)
        }

        // 方式2 https://learn.microsoft.com/zh-cn/windows/win32/api/shobjidl_core/nn-shobjidl_core-idesktopwallpaper
        /*let result = CoCreateInstance(
            &IDesktopWallpaper::IID,
            None,
            CLSCTX_ALL,
        );
        if result.is_err() {
            return Err(From::from(result));
        }
        // 获取桌面壁纸管理器的 COM 接口
        let desktop_wallpaper: IDesktopWallpaper = result.unwrap();

        // 获取桌面文件夹路径
        let result = SHGetDesktopFolder()?;

        // 设置锁屏壁纸
        // 构建完整路径
        let result = desktop_wallpaper.SetWallpaper(desktop_wallpaper.GetMonitorDevicePathAt(0).unwrap(), PCWSTR::from_raw(image_path.as_ptr() as _));
        if result.is_err() {
            return Err(From::from(result));
        }
        // 设置壁纸位置
        desktop_wallpaper.SetPosition(DWPOS_FILL)?;*/

        // 方式3 IActiveDesktop

        // 方法4 https://learn.microsoft.com/zh-cn/uwp/api/windows.system.userprofile.userprofilepersonalizationsettings.trysetwallpaperimageasync
    }.expect("设置壁纸失败");

    Ok(())
}

// 设置锁屏壁纸的函数
fn set_lock_screen_wallpaper(image_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // 初始化COM
        let com_res = CoInitializeEx(None, COINIT_MULTITHREADED);
        if com_res.is_err() {
            // 初始化COM库失败
            Err(Box::<dyn Error>::from(com_res.message()))?;
        }
        // 初始化Windows运行时
        RoInitialize(RO_INIT_MULTITHREADED)?;

        // https://learn.microsoft.com/zh-cn/uwp/api/windows.system.userprofile.userprofilepersonalizationsettings.trysetlockscreenimageasync
        /*let personalization_settings: IUserProfilePersonalizationSettingsStatics = RoGetActivationFactory(
            &HSTRING::from("Windows.System.UserProfile.UserProfilePersonalizationSettings"),
        )?;
        // Convert wallpaper path to HSTRING
        let hstring_wallpaper_path = HSTRING::from(image_path);
        // Set the lock screen image
        personalization_settings.TrySetLockScreenImageAsync(&hstring_wallpaper_path)?;*/

        /*if !UserProfilePersonalizationSettings::IsSupported() {
            return Err("Lock screen image setting is not supported on this device.".into());
        }
        let personalization_settings = UserProfilePersonalizationSettings::Current()?;
        StorageFile::GetFileFromPathAsync(&HSTRING::from(image_path))?.then(|file| {
           let result = personalization_settings.TrySetLockScreenImageAsync(&file)?;
            if !result.get().unwrap() {
                return Err("Failed to set lock screen image.".into());
            }
        })?;*/

        // https://learn.microsoft.com/zh-cn/uwp/api/windows.system.userprofile.lockscreen
        // SetImageFileAsync、SetImageStreamAsync
        let file: StorageFile = StorageFile::GetFileFromPathAsync(&HSTRING::from(image_path))?.get()?;
        // 将 StorageFile 转换为 IStorageFile
        let file: IStorageFile = file.cast()?;
        let result = LockScreen::SetImageFileAsync(&file)?;
        match result.get() {
            Ok(result) => {
                println!("{:?}", result);
            }
            Err(error) => {
                println!("{:?}", error);
            }
        }
        // Windows聚焦(Windows Spotlight)

        // 关闭Windows运行时
        RoUninitialize();
        // 释放COM资源
        CoUninitialize();
    }

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

// 创建Windows任务计划:
// https://docs.microsoft.com/zh-cn/windows/win32/taskschd/task-scheduler-start-page
// https://learn.microsoft.com/zh-cn/windows/win32/api/_taskschd
// 解锁、启动、登录等事件触发任务计划 taskschd.msc
fn create_schedule() -> Result<(), Box<dyn std::error::Error>> {
    // 获取当前执行程序的路径
    let exe_path = env::current_exe()?;
    // let args: Vec<String> = env::args().collect();

    unsafe {
        // 初始化COM库
        let com_res = CoInitializeEx(None, COINIT_MULTITHREADED);
        if com_res.is_err() {
            // 初始化COM库失败
            Err(Box::<dyn Error>::from(com_res.message()))?;
        }
        // 创建任务计划服务
        let task_service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_ALL)?;
        // 连接到任务计划服务
        task_service.Connect(
            &VARIANT::default(),
            &VARIANT::default(),
            &VARIANT::default(),
            &VARIANT::default(),
        )?;
        // 获取任务计划根文件夹
        let task_folder: ITaskFolder = task_service.GetFolder(&BSTR::from("\\"))?;
        let task_definition: ITaskDefinition = task_service.NewTask(0)?;
        let triggers: ITriggerCollection = task_definition.Triggers()?;
        let registration_info: IRegistrationInfo = task_definition.RegistrationInfo()?;
        let actions: IActionCollection = task_definition.Actions()?;
        let principal: IPrincipal = task_definition.Principal()?;
        let settings: ITaskSettings = task_definition.Settings()?;

        // 创建事件触发器
        // https://docs.microsoft.com/en-us/previous-versions//aa446887(v=vs.85)
        /*let trigger0 = triggers.Create(TASK_TRIGGER_EVENT);
        let i_event_trigger: IEventTrigger = trigger0.unwrap().cast::<IEventTrigger>()?;
        i_event_trigger.SetId(&BSTR::from("bing_wallpaper_event_trigger"))?;
        // i_event_trigger.SetDelay(&BSTR::from("2007-01-01T08:00:00"))?;
        // i_event_trigger.SetStartBoundary(&Local::now().to_rfc3339())?;
        // 定义事件查询。触发器将启动任务，当收到事件时。
        i_event_trigger.SetSubscription(&BSTR::from(r"<QueryList>
    <Query Id='0'>
        <Select Path='System'>
            *[System[Provider[@Name='Microsoft-Windows-Power-Troubleshooter'] and EventID=1]]
        </Select>
    </Query>
    <Query Id='1'>
        <Select Path='System'>
            *[System/Level=2]
        </Select>
    </Query>
</QueryList>"))?;*/

        // 创建定时触发器
        /*let trigger1 = triggers.Create(TASK_TRIGGER_TIME)?;
        let i_time_trigger: ITimeTrigger = trigger1.cast::<ITimeTrigger>()?;
        i_time_trigger.SetId(&BSTR::from("bing_wallpaper_time_trigger"))?;
        i_time_trigger.SetEnabled(VARIANT_BOOL::from(true))?;*/

        // 创建每日触发器
        /*let mut trigger2 = triggers.Create(TASK_TRIGGER_DAILY)?;
        let i_daily_trigger: IDailyTrigger = trigger2.cast::<IDailyTrigger>()?;
        i_daily_trigger.SetId(&BSTR::from("bing_wallpaper_daily_trigger"))?;
        i_daily_trigger.SetEnabled(VARIANT_BOOL::from(true))?;
        i_daily_trigger.SetDaysInterval(1)?;*/

        // 创建每周触发器
        /*let mut trigger3 = triggers.Create(TASK_TRIGGER_WEEKLY)?;
        let i_weekly_trigger: IWeeklyTrigger = trigger3.cast::<IWeeklyTrigger>()?;
        i_weekly_trigger.SetId(&BSTR::from("bing_wallpaper_weekly_trigger"))?;
        i_weekly_trigger.SetEnabled(VARIANT_BOOL::from(true))?;*/

        // 创建每月的第几天触发器
        /*let trigger4 = triggers.Create(TASK_TRIGGER_MONTHLY)?;
        let i_monthly_trigger: IMonthlyTrigger = trigger4.cast::<IMonthlyTrigger>()?;
        i_monthly_trigger.SetDaysOfMonth(1i32)?;*/

        // 创建每月的第几个星期几触发器
        /*let trigger5 = triggers.Create(TASK_TRIGGER_MONTHLYDOW)?;
        let i_monthly_dow_trigger: IMonthlyDOWTrigger = trigger5.cast::<IMonthlyDOWTrigger>()?;
        i_monthly_dow_trigger.SetDaysOfWeek(1i32)?;*/

        // 创建闲置触发，在发生空闲情况时启动任务的触发器
        /*let mut trigger6 = triggers.Create(TASK_TRIGGER_IDLE)?;
        let i_idle_trigger: IIdleTrigger = trigger6.cast::<IIdleTrigger>()?;
        i_idle_trigger.SetId(&BSTR::from("bing_wallpaper_idle_trigger"))?;
        i_idle_trigger.SetEnabled(VARIANT_BOOL::from(true))?;*/

        // 创建/修改任务时触发器
        /*let trigger7 = triggers.Create(TASK_TRIGGER_REGISTRATION)?;
        let i_registration_trigger: IRegistrationTrigger = trigger7.cast::<IRegistrationTrigger>()?;
        i_registration_trigger.SetId(&BSTR::from("bing_wallpaper_registration_trigger"))?;
        i_registration_trigger.SetEnabled(VARIANT_BOOL::from(true))?;*/

        // 创建启动触发器
        let trigger8 = triggers.Create(TASK_TRIGGER_BOOT)?;
        let i_boot_trigger: IBootTrigger = trigger8.cast::<IBootTrigger>()?;
        i_boot_trigger.SetId(&BSTR::from("bing_wallpaper_boot_trigger"))?;
        // 设置延迟时间
        // ISO 8601 duration format (e.g., "PT30M" for 30 minutes) P[nY][nM][nD][T[nH][nM][nS]]
        i_boot_trigger.SetDelay(&BSTR::from("PT2M"))?;
        i_boot_trigger.SetEnabled(VARIANT_BOOL::from(true))?;
        // trigger8.SetStartBoundary(&BSTR::from("2007-01-01T08:00:00"))?;

        // 创建登录触发器
        let trigger9 = triggers.Create(TASK_TRIGGER_LOGON)?;
        let i_logon_trigger: ILogonTrigger = trigger9.cast::<ILogonTrigger>()?;
        i_logon_trigger.SetId(&BSTR::from("bing_wallpaper_logon_trigger"))?;
        // 设置延迟时间
        // ISO 8601 duration format (e.g., "PT30M" for 30 minutes) P[nY][nM][nD][T[nH][nM][nS]]
        i_logon_trigger.SetDelay(&BSTR::from("PT2M"))?;
        i_logon_trigger.SetEnabled(VARIANT_BOOL::from(true))?;

        // 用于触发控制台连接或断开连接，远程连接或断开连接或工作站锁定或解锁通知的任务。
        let trigger11 = triggers.Create(TASK_TRIGGER_SESSION_STATE_CHANGE);
        let i_ssc_trigger: ISessionStateChangeTrigger = trigger11.unwrap()
            .cast::<ISessionStateChangeTrigger>()?;
        i_ssc_trigger.SetId(&BSTR::from("bing_wallpaper_ssc_trigger"))?;
        // 设置延迟时间
        // ISO 8601 duration format (e.g., "PT30M" for 30 minutes) P[nY][nM][nD][T[nH][nM][nS]]
        i_logon_trigger.SetDelay(&BSTR::from("PT30S"))?;
        i_ssc_trigger.SetEnabled(VARIANT_BOOL::from(true))?;
        // 获取或设置将触发任务启动的终端服务器会话更改的类型：7锁定；8解锁
        i_ssc_trigger.SetStateChange(TASK_SESSION_UNLOCK)?;
        // trigger11 = Some(i_ssc_trigger.cast::<ISessionStateChangeTrigger>()?);

        // 设置任务的注册信息
        registration_info.SetAuthor(&BSTR::from("bajins"))?;
        registration_info.SetDescription(&BSTR::from("设置Bing桌面壁纸"))?;

        // 创建任务的操作
        let i_action: IAction = actions.Create(TASK_ACTION_EXEC)?;
        let i_exec_action: IExecAction = i_action.cast()?;
        i_exec_action.SetPath(&BSTR::from(exe_path.to_str().unwrap()))?;
        i_exec_action.SetId(&BSTR::from("set_bing_wallpaper"))?;
        i_exec_action.SetWorkingDirectory(&BSTR::from(""))?;
        i_exec_action.SetArguments(&BSTR::from(""))?;

        // 设置任务的主体
        // principal.SetUserId(&BSTR::from())?;
        principal.SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN)?;
        principal.SetRunLevel(TASK_RUNLEVEL_HIGHEST)?;

        // 设置任务的设置
        settings.SetEnabled(VARIANT_TRUE)?;
        settings.SetHidden(VARIANT_TRUE)?;
        // settings.SetWakeToRun(VARIANT_TRUE)?;
        settings.SetRunOnlyIfIdle(VARIANT_FALSE)?;
        // settings.SetAllowDemandStart(VARIANT_TRUE)?;
        // settings.SetStartWhenAvailable(VARIANT_TRUE)?;
        // settings.SetDisallowStartIfOnBatteries(VARIANT_FALSE)?;
        // settings.SetStopIfGoingOnBatteries(VARIANT_FALSE)?;
        // settings.IdleSettings().unwrap().StopOnIdleEnd(VARIANT_FALSE)?;
        // settings.SetMultipleInstances(TASK_INSTANCES_IGNORE_NEW)?;
        // settings.SetRestartCount(5i32)?;
        // settings.SetRestartInterval(&BSTR::from("100"))?;
        // settings.SetPriority(0i32)?;
        // settings.SetExecutionTimeLimit(&BSTR::from("0"))?;

        /*let settings2: ITaskSettings2 = Some(settings.cast::<ITaskSettings2>()?);
        settings2.SetDisallowStartOnRemoteAppSession(VARIANT_FALSE)?;
        settings2.SetUseUnifiedSchedulingEngine(VARIANT_TRUE)?;*/

        // 设置任务的注册信息
        task_folder.RegisterTaskDefinition(
            &BSTR::from("SetBingWallpaper"),
            &task_definition,
            TASK_CREATE_OR_UPDATE.0,
            &VARIANT::default(),
            &VARIANT::default(),
            TASK_LOGON_INTERACTIVE_TOKEN,
            &VARIANT::default(),
        )?;
        // 释放COM库
        CoUninitialize();
    }

    Ok(())
}

// 检查网络连接状态
fn is_connected() -> Result<bool, Box<dyn std::error::Error>> {
    // 方式1：使用 Windows API 函数 InternetCheckConnectionW
    let url = "https://www.google.com";
    // Windows API 函数通常使用宽字符串（UTF-16）。将 URL 转换为宽字符串并添加一个 null 终止符
    let url_wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    let is_alive = unsafe {
        let result = InternetCheckConnectionW(
            PCWSTR(url_wide.as_ptr()),
            FLAG_ICC_FORCE_CONNECTION,
            0,
        );
        match result {
            Ok(_) => true,
            Err(_) => false,
        }
    };

    // 方式2：使用 Windows API 函数 InternetGetConnectedState
    let is_alive = unsafe {
        let mut flags = INTERNET_CONNECTION::default();
        // 调用 InternetGetConnectedState 函数获取网络连接状态
        InternetGetConnectedState(&mut flags, 0).is_err() ||
            // INTERNET_CONNECTION_MODEM：调制解调器连接。
            // INTERNET_CONNECTION_LAN：局域网连接。
            // INTERNET_CONNECTION_PROXY：代理连接。
            // INTERNET_RAS_INSTALLED：如果设置了此标志，则表示安装了远程访问服务 (RAS)，并且可能存在活动连接。
            (flags & (INTERNET_CONNECTION_MODEM | INTERNET_CONNECTION_LAN | INTERNET_CONNECTION_PROXY)) != INTERNET_CONNECTION::default()
            || (flags & INTERNET_RAS_INSTALLED) != INTERNET_CONNECTION::default()
    };

    // 方式3：使用 Windows API 函数 IsNetworkAlive
    let is_alive = unsafe {
        let result = IsNetworkAlive(&mut 0);
        match result {
            Ok(_) => true,
            Err(_) => false,
        }
    };


    // 方式4：使用 Windows API 函数 GetConnectivity
    let is_alive = unsafe {
        let network_list_manager: INetworkListManager = unsafe {
            CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL)?
        };
        let connectivity = network_list_manager.GetConnectivity()?;
        connectivity != NLM_CONNECTIVITY_DISCONNECTED
    };

    // 方式5：使用 Windows API 函数 GetNetworkParams
    let is_alive = unsafe {
        /*let mut fixed_info: *mut FIXED_INFO_W2KSP1 = null_mut();
        let mut size = 0;

        // 第一次调用 GetNetworkParams 函数来获取所需的缓冲区大小。
        let result = GetNetworkParams(*null_mut(), &mut size);
        if result != ERROR_BUFFER_OVERFLOW {
            return Err(windows::core::Error::from_win32().into());
        }
        // 分配足够的内存来存储网络参数。
        fixed_info = std::alloc::alloc(std::alloc::Layout::from_size_align_unchecked(size as usize, 1)) as *mut FIXED_INFO_W2KSP1;

        // 第二次调用 GetNetworkParams 函数来获取实际的网络参数。
        let result = GetNetworkParams(Option::from(fixed_info), &mut size);
        if result != ERROR_SUCCESS {
            return Err(windows::core::Error::from_win32().into());
        }
        // 检查 CurrentIpAddress 字段是否有效来判断网络是否连接。
        let is_connected = (*(*fixed_info).CurrentDnsServer).IpAddress.String[0] != 0;
        // 释放分配的内存。
        std::alloc::dealloc(fixed_info as *mut u8, std::alloc::Layout::from_size_align_unchecked(size as usize, 1));*/

        let mut adapter_info: Option<*mut IP_ADAPTER_INFO> = None;
        let mut out_buf_len: u32 = 0;

        // 第一次调用 GetAdaptersInfo 函数来获取所需的缓冲区大小
        GetAdaptersInfo(adapter_info, &mut out_buf_len);
        // Allocate memory for the buffer
        adapter_info = Some(
            std::mem::transmute(
                std::alloc::alloc(
                    std::alloc::Layout::from_size_align_unchecked(
                        out_buf_len as usize,
                        std::mem::align_of::<IP_ADAPTER_INFO>(),
                    )
                )
            )
        );
        // Second call to GetAdaptersInfo to get the actual data
        GetAdaptersInfo(adapter_info, &mut out_buf_len);
        // Access the adapter info
        let mut adapter_info = adapter_info.unwrap();
        // Iterate over the linked list of IP_ADAPTER_INFO structures
        /*while !adapter_info.is_null() {
            let adapter = &*adapter_info;
            let adapter_name = std::ffi::CStr::from_ptr(adapter.AdapterName.as_ptr() as *const i8);
            let description = std::ffi::CStr::from_ptr(adapter.Description.as_ptr() as *const i8);

            println!("Adapter Name: {:?}", adapter_name);
            println!("Description: {:?}", description);

            adapter_info = adapter.Next;
        }*/
        // Free the allocated memory
        std::alloc::dealloc(
            std::mem::transmute(adapter_info),
            std::alloc::Layout::from_size_align_unchecked(
                out_buf_len as usize,
                std::mem::align_of::<IP_ADAPTER_INFO>(),
            ),
        );

        (*(*adapter_info).CurrentIpAddress).IpAddress.String[0] != 0
    };

    Ok(is_alive)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image_path = download_image().await?;
    set_wallpaper(&image_path)?;
    // add_to_startup("", "")?;

    /*match fs::remove_file(image_path) {
        Err(e) => println!("壁纸文件删除错误: {}", e),
        Ok(_) => {}
    }*/

    let matches = command!()
        .arg(
            arg!(
                -t --taskschd "设置Windows任务计划，可在taskschd.msc中查看"
            )
                .value_name("taskschd")
                .required(false)
        )
        .get_matches();

    /*let cli = Command::new("SetBingWallpaper")
        .version("1.0")
        .author("https://bajins.com")
        .about("https://github.com/bajins/notes-vuepress").arg(arg!(-b - -built).action(clap::ArgAction::SetTrue));
    let cli = DerivedArgs::augment_args(cli);
    let matches = cli.get_matches();
    matches.get_flag("built")*/

    if matches.get_flag("taskschd") {
        create_schedule()?;
    }

    Ok(())
}

// 测试
#[tokio::test]
async fn test_get_url() {
    println!("{:?}", get_pixabay_image_url().await);
}