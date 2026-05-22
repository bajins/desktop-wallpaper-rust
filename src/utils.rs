use std::error::Error;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::{io, mem};
use windows::core::{Interface, HSTRING, PCWSTR};
use windows::Storage::{IStorageFile, StorageFile};
use windows::System::UserProfile::LockScreen;
use windows::Win32::Foundation::{GetLastError, ERROR_ACCESS_DENIED};
use windows::Win32::NetworkManagement::IpHelper::{GetAdaptersInfo, IP_ADAPTER_INFO};
use windows::Win32::Networking::NetworkListManager::{
    INetworkListManager, NetworkListManager, NLM_CONNECTIVITY_DISCONNECTED,
};
use windows::Win32::Networking::WinInet::{
    InternetCheckConnectionW, InternetGetConnectedState, FLAG_ICC_FORCE_CONNECTION,
    INTERNET_CONNECTION, INTERNET_CONNECTION_LAN, INTERNET_CONNECTION_MODEM,
    INTERNET_CONNECTION_PROXY, INTERNET_RAS_INSTALLED,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::System::EventNotificationService::IsNetworkAlive;
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_GETDESKWALLPAPER,
    SPI_SETDESKWALLPAPER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

/// 检查网络连接状态
pub fn is_connected() -> anyhow::Result<bool, Box<dyn std::error::Error>> {
    // 方式1：使用 Windows API 函数 InternetCheckConnectionW
    let url = "https://www.google.com";
    // Windows API 函数通常使用宽字符串（UTF-16）。将 URL 转换为宽字符串并添加一个 null 终止符
    let url_wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    let is_alive = unsafe {
        let result =
            InternetCheckConnectionW(PCWSTR(url_wide.as_ptr()), FLAG_ICC_FORCE_CONNECTION, 0);
        // 用 match 捕获异常
        match result {
            Ok(_) => true,
            Err(_) => false,
        }
    };

    // 方式2：使用 Windows API 函数 InternetGetConnectedState
    let is_alive = unsafe {
        let mut flags = INTERNET_CONNECTION::default();
        // 调用 InternetGetConnectedState 函数获取网络连接状态
        InternetGetConnectedState(&mut flags, Some(0)).is_err() ||
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
        // 用 match 捕获异常
        match result {
            Ok(_) => true,
            Err(_) => false,
        }
    };

    // 方式4：使用 Windows API 函数 GetConnectivity
    let is_alive = unsafe {
        let network_list_manager: INetworkListManager =
            CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let connectivity = network_list_manager
            .GetConnectivity()
            .map_err(|e| e.to_string())?;
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
        adapter_info = Some(std::mem::transmute(std::alloc::alloc(
            std::alloc::Layout::from_size_align_unchecked(
                out_buf_len as usize,
                std::mem::align_of::<IP_ADAPTER_INFO>(),
            ),
        )));
        // Second call to GetAdaptersInfo to get the actual data
        GetAdaptersInfo(adapter_info, &mut out_buf_len);
        // Access the adapter info
        let adapter_info = adapter_info.ok_or_else(|| std::io::Error::last_os_error())?;
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

/// 设置锁屏壁纸的函数
pub fn set_lock_screen_wallpaper(
    image_path: &str,
) -> anyhow::Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // 初始化COM
        let com_res = CoInitializeEx(None, COINIT_MULTITHREADED);
        if com_res.is_err() {
            // 初始化COM库失败
            Err(Box::<dyn Error>::from(com_res.message()))?;
        }
        // 初始化Windows运行时
        RoInitialize(RO_INIT_MULTITHREADED).map_err(|e| e.to_string())?;

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
            if !result.get()? {
                return Err("Failed to set lock screen image.".into());
            }
        })?;*/

        // https://learn.microsoft.com/zh-cn/uwp/api/windows.system.userprofile.lockscreen
        // SetImageFileAsync、SetImageStreamAsync
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(image_path))
            .map_err(|e| e.to_string())?;
        // 将 StorageFile 转换为 IStorageFile
        let file = file.cast::<IStorageFile>().map_err(|e| e.to_string())?;
        let result = LockScreen::SetImageFileAsync(&file);
        match result {
            Ok(result) => {
                println!("锁屏壁纸设置成功");
            }
            Err(error) => {
                eprintln!("设置锁屏壁纸失败: {:?}", error);
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

/// 设置壁纸的函数
pub fn set_wallpaper(image_path: &str) -> anyhow::Result<(), Box<dyn std::error::Error>> {
    // 通过系统调用设置壁纸
    println!("{:?}", get_wallpaper()?);
    /*println!("{:?}", wallpaper::get());
    // 从文件路径设置当前桌面的壁纸。
    wallpaper::set_from_path(&image_path)?;
    // 设置壁纸的样式，有填充、适应、拉伸、居中、裁剪等模式可选。
    wallpaper::set_mode(wallpaper::Mode::Crop)?;
    // 从URL设置当前桌面的壁纸。
    // wallpaper::set_from_url(&image_path)?;
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
        if result.is_err() {
            // 设置失败
            // 设置失败，检查错误码
            let error = GetLastError();
            if error == ERROR_ACCESS_DENIED {
                // 错误码表明权限不足
                Ok(false)
            } else {
                // 其他错误
                Err(windows::core::Error::from_hresult(result.into()))
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
        let desktop_wallpaper: IDesktopWallpaper = result?;

        // 获取桌面文件夹路径
        let result = SHGetDesktopFolder()?;

        // 设置锁屏壁纸
        // 构建完整路径
        let result = desktop_wallpaper.SetWallpaper(desktop_wallpaper.GetMonitorDevicePathAt(0)?, PCWSTR::from_raw(image_path.as_ptr() as _));
        if result.is_err() {
            return Err(From::from(result));
        }
        // 设置壁纸位置
        desktop_wallpaper.SetPosition(DWPOS_FILL)?;*/

        // 方式3 IActiveDesktop

        // 方法4 https://learn.microsoft.com/zh-cn/uwp/api/windows.system.userprofile.userprofilepersonalizationsettings.trysetwallpaperimageasync
    }
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取当前壁纸的函数
pub fn get_wallpaper() -> anyhow::Result<String, Box<dyn std::error::Error>> {
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

/// Windows Imaging Component (WIC) 将图像转码为 Windows 11 中 TranscodedWallpaper 文件（JPEG 格式）
// https://learn.microsoft.com/zh-cn/windows/win32/wic/-wic-about-windows-imaging-codec
pub fn wic_codec() -> windows::core::Result<()> {
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
        let property_bag: IPropertyBag2 = frame_encode.GetPropertyBag2()?;
        let mut var_quality = PROPVARIANT { vt: VT_R4, fltVal: 0.85f32, ..Default::default() };
        property_bag.Write(L"ImageQuality", &var_quality)?;

        // 设置帧大小
        let (width, height) = source_to_write.GetSize()?;
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
