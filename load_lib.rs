use std::path::Path;
use libloading::Library;

/**
 * 加载动态链库
 * @param lib_path 路径
 */
fn load_lib(lib_path: &str)->Result<Library,&str>{
    if Path::new(lib_path).exists(){
        unsafe{
            let library = Library::new(lib_path)?;
            Ok(library)
        }
        Err("load lib error") // 传入的路径不合法，无法通过该路径定位到一个文件
    }
    Err("lib not found") // 传入的路径不是合法的动态库路径，无法通过该路径加载所需要的库
}