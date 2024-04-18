use std::env;
use std::fs;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::Command;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use winit::window::Icon;
use image::{ImageBuffer, Luma, DynamicImage, GenericImageView};
use ico::{IconDir};

fn main() {
    // 只在Windows平台上运行
    if cfg!(target_os = "windows") {
        // 将PNG图标转换为ICO图标
        let ico_path = "assets/icon1.ico";
        png_to_ico("assets/icon1_1.png", ico_path).expect("PNG转ICON失败");
        // 创建Windows资源文件
        let mut res = winres::WindowsResource::new();
        res.set_icon(ico_path);
        res.set_manifest_file("assets/manifest.xml");
        res.compile().unwrap();
        // 删除生成的ICO图标文件
        match fs::remove_file(ico_path) {
            Err(e) => println!("文件删除错误: {}", e),
            Ok(_) => {}
        }

        // 编译`.rc`文件生成`.res`文件
        /*Command::new("windres")
            .args(&["src/icon.rc", "-O coff", "-o", "icon.res"])
            .status()
            .unwrap();
        Command::new("cargo")
            .args(&["rustc", "--"
                , "-C", "link-arg=/ENTRY:mainCRTStartup"
                , "-C", "link-arg=icon.res"
            ])
            .status()
            .unwrap();
        // 指示 cargo 将`.res`文件链接到最终的可执行文件中
        println!("cargo:rustc-link-search=native=icon.res");
        println!("cargo:rustc-link-lib=dylib={}", "icon");
        println!("cargo:rustc-link-lib=kernel32"); // 需要链接kernel32.dll以使用资源
        */
        // embed_resource::compile("./icon.rc");
        /*embed_resource::compile_for("assets/installer.rc"
                                    , &["desktop-wallpaper-rust", "desktop-wallpaper-rust-installer"]
                                    , &["VERSION=\"0.1.0\""]);
        embed_resource::compile_for("assets/uninstaller.rc", &["unins001"], embed_resource::NONE);
        */
        // 请求管理员权限
        /*println!("cargo:rerun-if-changed=manifest.rc");
        embed_resource::compile("manifest.rc", embed_resource::NONE);*/

        // cargo bloat --release --crates --crates-limit 20

        // MSVC
        // cargo rustc --release -- -Clink-args="/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup"
        println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
        println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
        // println!("cargo:rustc-link-arg=/MACHINE:X64");
        // println!("cargo:rustc-link-arg=/NODEFAULTLIB:libucrt.lib");
        // GCC
        // cargo rustc --release -- -Clink-args="-Wl,--subsystem,windows"
        // println!("cargo:rustc-link-arg=-Wl,--subsystem,windows");
    }
}


/*fn set_icon() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("设置桌面壁纸")
        /*.with_decorations(true)
        .with_resizable(false)
        .with_transparent(false)
        .with_visible(true)
        .with_maximized(false)
        .with_min_inner_size(winit::dpi::LogicalSize::new(400.0, 400.0))
        .with_inner_size(winit::dpi::LogicalSize::new(400.0, 400.0))
        .with_position(winit::dpi::LogicalPosition::new(0.0, 0.0))
        .with_fullscreen(None)
        .with_ime(false)
        .with_always_on_top(false)*/
        // .with_window_icon(Some(include_bytes!("assets/icon1.png")?))
        .with_window_icon(Some(load_icon("assets/icon1.png")?))
        .build(&event_loop).unwrap();

    // event_loop.set_control_flow(ControlFlow::Wait);

    event_loop.run(move |event, elwt, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = elwt.exit(),
            Event::AboutToWait => {
                /*if let Some(icon) = load_icon("assets/icon1.png").ok() {
                    window.set_window_icon(Some(icon));
                }*/
                // window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                // window.set_visible(true);
            }
            _ => (),
        }
    });
}*/

/*fn load_icon(icon_path: &str) -> Result<Icon, Box<dyn Error>> {
    let icon_img = image::open(icon_path)?;
    let rgba = icon_img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(Icon::new(rgba.into_raw(), width, height))
}*/


/*fn load_icon_from_file(path: &str) -> Option<winit::window::Icon> {
    use std::fs::File;
    use std::io::Read;
    use winapi::um::winuser::LoadImageW;
    use winapi::um::winuser::LR_DEFAULTSIZE;
    use winapi::um::winuser::IMAGE_ICON;
    use winapi::um::winuser::LR_LOADFROMFILE;

    let mut file = File::open(path).ok()?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).ok()?;

    let hicon = unsafe {
        LoadImageW(
            std::ptr::null_mut(),
            data.as_ptr() as _,
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE | LR_DEFAULTSIZE,
        )
    };

    if hicon.is_null() {
        return None;
    }

    Some(winit::window::Icon::from_raw(hicon as _))
}*/

// 将PNG文件转换为ICO文件
fn png_to_ico(png_path: &str, output_path: &str) -> Result<(), Box<dyn Error>> {
    // PNG文件数据
    // let png_data = fs::read(png_path)?;
    // 读取PNG数据并解码为DynamicImage
    // let img: DynamicImage = image::load_from_memory(png_data.as_slice())?;
    // 将图像转换为灰度图像，因为ICO仅支持单通道图像
    // let gray_img: ImageBuffer<Luma<u8>, Vec<u8>> = img.grayscale().into_luma8();
    let input_path = Path::new(png_path);
    let output_path = Path::new(output_path);
    // 读取 PNG 图像
    let img = image::open(input_path)?;
    // 创建一个 ICO 容器
    let mut icon_dir = IconDir::new(ico::ResourceType::Icon);
    // 添加图像到 ICO 容器
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    // let mut icon_image = ImageBuffer::new(width, height);
    let ico_img =ico::IconImage::from_rgba_data(width, height, rgba.into_raw());
    icon_dir.add_entry(ico::IconDirEntry::encode(&ico_img).unwrap());
    // 将 ICO 保存到文件
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);
    icon_dir.write(&mut writer).unwrap();

    Ok(())
}