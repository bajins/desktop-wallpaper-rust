// #!必须放在头部
// https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute
#![windows_subsystem = "windows"]

use std::{env, fs, io, mem};
use std::error::Error;
use std::ffi::c_void;
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::rc::Rc;
use reqwest;
use url::Url;
// use winapi::um::winuser::{SystemParametersInfoW, SPI_SETDESKWALLPAPER, SPIF_UPDATEINIFILE, SPIF_SENDCHANGE};
use windows::core::{BSTR, GUID, Interface, IntoParam, VARIANT};
use windows::Win32::UI::WindowsAndMessaging::{SPI_GETDESKWALLPAPER, SPI_SETDESKWALLPAPER, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoA, SystemParametersInfoW};
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, GetLastError, TRUE, VARIANT_BOOL, VARIANT_FALSE, VARIANT_TRUE};
use windows::Win32::System::TaskScheduler::{IAction, IActionCollection, IBootTrigger, IDailyTrigger, IEventTrigger, IExecAction, IIdleTrigger, ILogonTrigger, IMonthlyDOWTrigger, IMonthlyTrigger, INetworkSettings, IPrincipal, IRegistrationInfo, IRegistrationTrigger, IRepetitionPattern, ITaskDefinition, ITaskFolder, ITaskService, ITaskSettings, ITimeTrigger, ITrigger, ITriggerCollection, IWeeklyTrigger, TaskScheduler, TASK_ACTION_EXEC, TASK_LOGON_TYPE, TASK_RUNLEVEL_TYPE, TASK_TRIGGER_BOOT, TASK_TRIGGER_DAILY, TASK_TRIGGER_EVENT, TASK_TRIGGER_IDLE, TASK_TRIGGER_LOGON, TASK_TRIGGER_MONTHLY, TASK_TRIGGER_MONTHLYDOW, TASK_TRIGGER_REGISTRATION, TASK_TRIGGER_TIME, TASK_TRIGGER_WEEKLY, TASK_LOGON_INTERACTIVE_TOKEN, TASK_TRIGGER_TYPE2, TASK_TRIGGER_SESSION_STATE_CHANGE, TASK_CREATE_OR_UPDATE, ISessionStateChangeTrigger, TASK_SESSION_STATE_CHANGE_TYPE, TASK_SESSION_UNLOCK, TASK_TRIGGER_CUSTOM_TRIGGER_01, ITaskTrigger, TASK_RUNLEVEL_HIGHEST, TASK_INSTANCES_IGNORE_NEW, ITaskSettings2};
use windows::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, CoCreateInstance, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::System::Variant::{VariantClear, VariantInit};
use winreg::enums::*;
use winreg::RegKey;
use wallpaper;
use clap::{arg, command};
use rand::Rng;


// 下载必应每日一图
async fn get_bing_image_url() -> Result<(String, String), Box<dyn Error>> {
    // Bing 壁纸API的URL
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

//
async fn get_spotlight_image_url() -> Result<(String, String), Box<dyn Error>> {
    // Bing 壁纸API的URL
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
    // Bing 壁纸API的URL
    let api_url = "https://assets.msn.cn/resolver/api/resolve/v3/config/?expType=AppConfig&expInstance=default&apptype=edgeChromium&v=20240202.634";
    // 发起网络请求
    let res = reqwest::get(api_url).await?;
    let body = res.text().await?;
    println!("{:?}", body);
    let v: Value = serde_json::from_str(&body)?;
    let datas = v["configs"]["BackgroundImageWC/default"]["properties"]["cmsImage"]["data"]
        .as_array().unwrap();
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

// 下载壁纸图片
async fn download_image() -> Result<String, Box<dyn std::error::Error>> {
    // 获取图片的URL
    let image_url;
    let file_name;
    let mut rng = rand::thread_rng();
    let num = rng.gen_range(0..3);
    if num == 1 {
        (image_url, file_name) = get_spotlight_image_url().await?;
    } else if num == 2 {
        (image_url, file_name) = get_edge_chromium_image_url().await?;
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
        i_boot_trigger.SetEnabled(VARIANT_BOOL::from(true))?;
        // trigger8.SetStartBoundary(&BSTR::from("2007-01-01T08:00:00"))?;

        // 创建登录触发器
        let trigger9 = triggers.Create(TASK_TRIGGER_LOGON)?;
        let i_logon_trigger: ILogonTrigger = trigger9.cast::<ILogonTrigger>()?;
        i_logon_trigger.SetId(&BSTR::from("bing_wallpaper_logon_trigger"))?;
        i_logon_trigger.SetEnabled(VARIANT_BOOL::from(true))?;

        // 用于触发控制台连接或断开连接，远程连接或断开连接或工作站锁定或解锁通知的任务。
        let trigger11 = triggers.Create(TASK_TRIGGER_SESSION_STATE_CHANGE);
        let i_ssc_trigger: ISessionStateChangeTrigger = trigger11.unwrap()
            .cast::<ISessionStateChangeTrigger>()?;
        i_ssc_trigger.SetId(&BSTR::from("bing_wallpaper_ssc_trigger"))?;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image_path = download_image().await?;
    set_wallpaper(&image_path)?;
    // add_to_startup("", "")?;

    match fs::remove_file(image_path) {
        Err(e) => println!("壁纸文件删除错误: {}", e),
        Ok(_) => {}
    }

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