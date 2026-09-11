// #!必须放在头部
// https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute
// #![windows_subsystem = "windows"]

pub mod images;
pub mod task_scheduler;
pub mod ui;
pub mod utils;

use std::time::Duration;

use clap::{arg, command};

/// 程序内运行时长自限：主流程整体超过该时长即自动终止进程
///
/// 与任务计划 SetExecutionTimeLimit(PT10M) 数值保持一致，构成不依赖任务计划的双保险：
/// 控制台程序收不到任务计划的软终止（WM_CLOSE），当任务计划的终止策略失效或程序被
/// 手动直接运行时，只有此处能可靠终止长时间挂起的进程（如无超时的网络请求）。
const EXECUTION_TIME_LIMIT: Duration = Duration::from_secs(600);

#[tokio::main]
async fn main() {
    // 先解析命令行参数：单实例锁检查需要区分任务计划静默模式与手动运行
    let matches = command!()
        .arg(
            arg!(
                -t --taskschd "设置Windows任务计划，可在taskschd.msc中查看"
            )
            .value_name("taskschd")
            .required(false),
        )
        // 任务计划静默模式：任务场景无人值守，出错不能阻塞在弹窗上（会挂死进程导致堆积），
        // 改为写标准错误并以非零退出码结束，退出码会记录在任务计划历史中便于排查
        .arg(
            arg!(
                -s --scheduled "由任务计划启动：静默执行，出错不弹窗，以非零退出码结束"
            )
            .value_name("scheduled")
            .required(false),
        )
        .get_matches();
    let is_scheduled = matches.get_flag("scheduled");

    // 程序内单实例锁：已有实例在运行则直接退出（响应进程堆积的程序内兜底，不依赖任务计划）
    if !utils::try_acquire_single_instance() {
        // 手动运行时弹提示，避免用户误以为程序"没反应"；
        // 任务计划场景静默退出（无人值守，弹窗同样会挂死进程）
        if !is_scheduled {
            ui::show_info("已有实例正在运行，本次启动已退出", "提示");
        }
        return;
    }

    // 用一个内部 async 块包裹所有可能出错的操作，并整体套上运行时长自限
    let result: Result<(), Box<dyn std::error::Error>> =
        tokio::time::timeout(EXECUTION_TIME_LIMIT, async {
            if matches.get_flag("taskschd") {
                ui::confirm_and_run(
                    "确定要设置任务吗？",
                    "警告",
                    "https://github.com/bajins/desktop-wallpaper-rust",
                    task_scheduler::create_schedule_guarded,
                )?;
            }

            let image_path = images::download_image().await?;
            utils::set_wallpaper(&image_path)?;
            // task_scheduler::add_to_startup("", "")?;

            Ok(())
        })
            .await.unwrap_or_else(|_| Err("程序运行超过 10 分钟上限，已自动终止".into()));

    // 统一在退出前处理错误
    if let Err(e) = result {
        // 任务计划场景：静默退出，严禁阻塞弹窗（无人点击会导致进程挂死堆积）
        if is_scheduled {
            eprintln!("程序异常退出: {e}");
            std::process::exit(1);
        }
        // 手动运行场景：弹窗提示后立即终止进程。
        // 不能让 main 正常返回：worker 线程可能仍卡在阻塞 FFI（如服务挂死时的
        // RegisterTaskDefinition），runtime 收尾会无限等待该线程，进程将永不退出；
        // process::exit 立即终止包括卡死线程在内的全部线程
        ui::show_error(&format!("程序异常退出: {e}"));
        std::process::exit(1);
    }
}

// 测试
#[tokio::test]
async fn test_get_url() {
    println!("{:?}", images::get_pixabay_image_url().await);
}
