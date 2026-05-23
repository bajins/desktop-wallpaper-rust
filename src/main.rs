// #!必须放在头部
// https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute
// #![windows_subsystem = "windows"]

pub mod images;
pub mod task_scheduler;
pub mod ui;
pub mod utils;

use clap::{arg, command};

//#[async_std::main]
#[tokio::main]
async fn main() {
    // 用一个内部 async 块包裹所有可能出错的操作
    let result: Result<(), Box<dyn std::error::Error>> = async {
        let matches = command!()
            .arg(
                arg!(
                    -t --taskschd "设置Windows任务计划，可在taskschd.msc中查看"
                )
                .value_name("taskschd")
                .required(false),
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
            ui::confirm_and_run(
                "确定要设置任务吗？",
                "警告",
                "https://github.com/bajins/desktop-wallpaper-rust",
                task_scheduler::create_schedule,
            )?;
        }

        let image_path = images::download_image().await?;
        utils::set_wallpaper(&image_path)?;
        // task_scheduler::add_to_startup("", "")?;

        /*match fs::remove_file(image_path) {
            Err(e) => println!("壁纸文件删除错误: {}", e),
            Ok(_) => {}
        }*/

        Ok(())
    }
    .await;

    // 统一在退出前弹窗
    if let Err(e) = result {
        ui::show_error(&format!("程序异常退出: {}", e));
        // 这里可以选择：
        // std::process::exit(1);   // 非零退出码
        // 或者什么都不做，让 main 正常结束
    }
}

// 测试
#[tokio::test]
async fn test_get_url() {
    println!("{:?}", images::get_pixabay_image_url().await);
}
