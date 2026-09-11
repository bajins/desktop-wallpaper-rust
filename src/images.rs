use rand::{seq::SliceRandom, RngExt};
use reqwest::{header, Client, RequestBuilder, StatusCode};
use scraper::{Html, Selector};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use std::{env, io};
use url::Url;

/// 通用浏览器 User-Agent：用于图片下载与部分 API 请求，避免被图片 CDN 当作爬虫拒绝而返回空响应
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36 Edg/123.0.0.0";

/// 单次 HTTP 请求（含读取响应体）的超时上限
///
/// 禁止无超时请求：网络异常时请求会无限挂起，进程长时间存活，
/// 叠加任务计划高频触发会形成进程堆积（进程风暴治理的一环）
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// 构建统一的 HTTP 客户端：带浏览器 UA（避免图片 CDN 拒绝裸请求返回空响应）与请求超时
///
/// 所有网络请求必须经由此函数获取客户端，
/// 禁止裸用 `reqwest::get` / `Client::new()`——二者默认无超时，会导致进程挂死堆积
fn http_client() -> Client {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        // 构建失败仅发生在 TLS 后端初始化异常等底层场景，直接 panic 暴露问题
        .expect("构建 HTTP 客户端失败")
}

/// 下载必应每日一图
/// https://blog.qzink.me/posts/%E5%BF%85%E5%BA%94%E5%A3%81%E7%BA%B8API%E5%8F%8A%E5%8E%86%E5%8F%B2%E5%AD%98%E6%A1%A3/
pub async fn get_bing_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    // https://services.bingapis.com/ge-apps/api/v2/bwc/hpimages?mkt=zh-cn&theme=bing&defaultBrowser=ME&dhpSetToBing=True&dseSetToBing=True
    // https://www.bing.com/hp/api/model?mkt=en-US
    let api_url = "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=1&mkt=en-US";
    // 发起网络请求（统一客户端：带 UA + 超时）
    let res = http_client().get(api_url).send().await?;
    let body = res.text().await?;
    println!("{:?}", body);
    let v: Value = serde_json::from_str(&body)?;
    let image_url = format!(
        "https://www.bing.com{}",
        v["images"][0]["url"]
            .as_str()
            .ok_or_else(|| std::io::Error::last_os_error())?
    );
    // 解析URL
    let parsed = Url::parse(&image_url)?;
    // 获取查询参数
    /*for (key, value) in parsed.query_pairs() {
        println!("{}: {}", key, value);
    }*/
    let id = parsed
        .query_pairs()
        .find(|(key, _)| key == "id")
        .ok_or_else(|| std::io::Error::last_os_error())?;
    println!("{:?}", id.1);
    let rf = parsed
        .query_pairs()
        .find(|(key, _)| key == "rf")
        .ok_or_else(|| std::io::Error::last_os_error())?;
    println!("{:?}", rf.1);

    Ok((image_url, rf.1.to_string()))
}

/// 获取Windows Spotlight壁纸
/// https://learn.microsoft.com/zh-cn/windows/configuration/windows-spotlight
///
/// https://api.qzink.me/spotlight
/// https://zhuanlan.zhihu.com/p/637298844
/// https://blog.qzink.me/tags/%E5%A3%81%E7%BA%B8/
/// https://github.com/ORelio/Spotlight-Downloader/blob/master/SpotlightAPI.md
pub async fn get_spotlight_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    // let api_url = "https://arc.msn.com/v3/Delivery/Placement?pid=209567&fmt=json&cdm=1&pl=zh-CN&lc=zh-CN&ctry=CN";
    let api_url = "https://fd.api.iris.microsoft.com/v4/api/selection?placement=88000820&bcnt=4&fmt=json&locale=zh-CN&country=CN";
    // 发起网络请求（统一客户端：带 UA + 超时）
    let res = http_client().get(api_url).send().await?;
    let body = res.text().await?;
    println!("{:?}", body);
    let v: Value = serde_json::from_str(&body)?;
    let item: Value = serde_json::from_str(
        v["batchrsp"]["items"][0]["item"]
            .as_str()
            .ok_or_else(|| std::io::Error::last_os_error())?,
    )?;
    let image_url = item["ad"]["landscapeImage"]["asset"]
        .as_str()
        .ok_or_else(|| std::io::Error::last_os_error())?;

    println!("{:?}", item["ad"]["description"]);

    Ok((image_url.to_string(), String::from("")))
}

/// 获取Edge Chromium壁纸
pub async fn get_edge_chromium_image_url() -> Result<(String, String), Box<dyn Error>> {
    //
    let api_url = "https://ntp.msn.com/edge/ntp?locale=zh-cn";
    // 发起网络请求（统一客户端：带 UA + 超时）
    let client = http_client();
    // 创建一个请求构建器
    let mut builder: RequestBuilder = client.get(api_url);
    // 设置请求头
    builder = builder.header(header::USER_AGENT, USER_AGENT);
    // 发送请求并获取响应
    let response = builder.send().await?;
    // 检查响应状态码
    if response.status() != StatusCode::OK {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "请求获取版本失败",
        )));
    }
    let body = response.text().await?;
    // println!("{:?}", body);
    // 解析HTML
    let document = Html::parse_document(&*body);
    let head_selector = Selector::parse("head")?;
    let head = document
        .select(&head_selector)
        .next()
        .ok_or_else(|| std::io::Error::last_os_error())?;
    let dcs = head
        .value()
        .attr("data-client-settings")
        .ok_or_else(|| std::io::Error::last_os_error())?;
    if dcs.is_empty() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "解析HTML并获取版本信息失败",
        )));
    }
    // 解析JSON
    let body_json: Value = serde_json::from_str(&dcs)?;
    println!("{:?}", body_json);
    let version = body_json["bundleInfo"]["v"]
        .as_str()
        .ok_or_else(|| std::io::Error::last_os_error())?;

    // 壁纸API的URL
    let api_url = "https://assets.msn.cn/resolver/api/resolve/v3/config/\
    ?expType=AppConfig&expInstance=default&apptype=edgeChromium&v="
        .to_owned()
        + version;
    // 发起网络请求（统一客户端：带 UA + 超时，复用本函数已构建的 client）
    let res = client.get(api_url).send().await?;
    let body = res.text().await?;
    let v: Value = serde_json::from_str(&body)?;
    let datas = v["configs"]["BackgroundImageWC/default"]["properties"]["cmsImage"]["data"]
        .as_array()
        .ok_or_else(|| std::io::Error::last_os_error())?;
    println!("{:?}", datas);
    // 随机获取一张图片
    let mut rng = rand::rng();
    let num = rng.random_range(0..datas.len());
    let data_map = datas[num]["image"]
        .as_object()
        .ok_or_else(|| std::io::Error::last_os_error())?;
    // 获取分辨率最大的图片
    let image = data_map
        .iter()
        .max_by_key(|(key, _value)| key.to_string()[1..].parse::<i64>().ok())
        .map(|(_key, _value)| _value);
    // 获取图片的URL
    let mut image_url = v["configs"]["StickyPeek/default"]["properties"]
        ["stickyPeekLightCoachmarkMainImageURL"]
        .as_str()
        .ok_or_else(|| std::io::Error::last_os_error())?;
    // 截取URL的路径
    match image_url.rfind("/") {
        Some(index) => image_url = &image_url[0..index + 1],
        None => println!("Substring not found"),
    }
    // 拼接图片的URL
    let image_url = format!(
        "{}{}",
        image_url,
        image
            .ok_or_else(|| std::io::Error::last_os_error())?
            .as_str()
            .ok_or_else(|| std::io::Error::last_os_error())?
    );

    println!("{:?}", data_map);
    println!("{:?}", image_url);

    Ok((image_url.to_string(), String::from("")))
}

/// 获取Pixabay壁纸
pub async fn get_pixabay_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://pixabay.com/api/?key=30271602-41319186b7198e7712c568e90&lang=zh&editors_choice=true";
    // 发起网络请求（统一客户端：带 UA + 超时）
    let res = http_client().get(api_url).send().await?;
    let body = res.text().await?;
    println!("{:?}", body);
    let v: Value = serde_json::from_str(&body)?;
    let img_infos = v["hits"]
        .as_array()
        .ok_or_else(|| std::io::Error::last_os_error())?;
    // 随机获取一张图片
    let mut rng = rand::rng();
    let num = rng.random_range(0..img_infos.len());
    let image_url = img_infos[num]["largeImageURL"]
        .as_str()
        .ok_or_else(|| std::io::Error::last_os_error())?;

    println!("{:?}", image_url);

    Ok((image_url.to_string(), String::from("")))
}

/// 获取金山词霸壁纸
pub async fn get_iciba_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://open.iciba.com/dsapi";
    // 发起网络请求（统一客户端：带 UA + 超时）
    let res = http_client().get(api_url).send().await?;
    let body = res.text().await?;
    println!("{:?}", body);
    let v: Value = serde_json::from_str(&body)?;
    let image_url = v["picture2"]
        .as_str()
        .ok_or_else(|| std::io::Error::last_os_error())?;

    println!("{:?}", image_url);

    Ok((image_url.to_string(), String::from("")))
}

/// 获取Alphacoders壁纸
pub async fn get_alphacoders_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://alphacoders.com/nature-4k-wallpapers";
    // 发起网络请求（统一客户端：带 UA + 超时）
    let client = http_client();
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
        header::HeaderValue::from_static(
            r#""Microsoft Edge";v="123", "Not:A-Brand";v="8", "Chromium";v="123""#,
        ),
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
        header::HeaderValue::from_static(USER_AGENT),
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
    let a_selector = Selector::parse("div.center a")?;
    let img_selector = Selector::parse("picture img")?;
    let img: Vec<_> = document.select(&img_selector).collect();
    println!("{:?}", img);
    let mut img_ids: Vec<String> = Vec::new();
    // 遍历符合父元素选择器条件的元素
    for a in document.select(&a_selector) {
        let image_url = a
            .value()
            .attr("href")
            .ok_or_else(|| std::io::Error::last_os_error())?;
        println!("{}", image_url);
        // 解析URL
        let parsed = Url::parse(&image_url)?;
        // 获取查询参数
        /*for (key, value) in parsed.query_pairs() {
            println!("{}: {}", key, value);
        }*/
        let id = parsed
            .query_pairs()
            .find(|(key, _)| key == "i")
            .ok_or_else(|| std::io::Error::last_os_error())?;
        println!("{:?}", id.1);
        img_ids.push(id.1.to_string());
    }
    if img_ids.is_empty() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "解析HTML并获取版本信息失败",
        )));
    }
    // 随机获取一张图片
    let mut rng = rand::rng();
    let num = rng.random_range(0..img_ids.len());
    let image_id = img_ids[num].as_str();

    let image_url = format!(
        "https://initiate.alphacoders.com/download/images6/{}/png",
        image_id
    );
    println!("{:?}", image_url);

    Ok((image_url.to_string(), String::from("")))
}

/// 获取NASA壁纸
/// https://github.com/nasa/api-docs
/// https://nasa.github.io/api-docs-stage
/// https://github.com/nasa/apod-api
/// https://wilsjame.github.io/how-to-nasa
pub async fn get_nasa_image_url() -> Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    // https://apod.nasa.gov/apod
    // let formatted_date = info.date.format("%Y-%m-%d").to_string();
    // &date={formatted_date}
    // U0PqJ5UprbVQExkXc7ZgsGVfIM7Z1O8Uiv7g2hOO
    // OcrpVpxqR9vJah1YaYVlgahUK7HURjAmWnciBsCz
    // YbbMNWeWX2BqxoPKXEiWWcKMgNlUHhHXgqWG5XBt
    // 4Qw6dtCAkjZa9G4zCnYDZeg0UjECBMcptxCHnJQt
    let api_url = "https://api.nasa.gov/planetary/apod?api_key=5B6oJsSCQyekXZvNOKpsUhRPl1e7FHqjIAyHpybk";
    // 创建一个允许所有证书的信任锚
    /*let mut trust_anchor = RootCertStore::empty();
    trust_anchor.add_server_trust_anchors(&[TrustAnchor::from_pem(include_bytes!("any_certificate.pem"))?]);
    // 创建一个TLS配置，忽略证书验证
    let tls_builder = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(trust_anchor);*/
    // 创建一个允许所有证书的reqwest客户端
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(REQUEST_TIMEOUT)
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
    let media_type = v["media_type"]
        .as_str()
        .ok_or_else(|| std::io::Error::last_os_error())?;
    if media_type != "image" {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("媒体类型不是图片: {media_type}"),
        )));
    }
    let image_url = v["hdurl"]
        .as_str()
        .ok_or_else(|| std::io::Error::last_os_error())?;

    println!("{:?}", image_url);

    Ok((image_url.to_string(), String::from("")))
}

/// 壁纸源数量（启用源数）。增删源时同步修改此处与 `try_one_source` 的 match 分支
const SOURCE_COUNT: usize = 5;

/// 下载壁纸图片
///
/// 随机打乱所有候选源顺序，依次尝试完整流程（获取URL→下载→保存），
/// 任一源成功即返回文件路径；全部失败则聚合各源错误信息后抛出，
/// 由顶层 `main` 统一弹窗，避免单个源故障导致整个程序退出。
///
/// 候选源各 1/5 = 20% 均等（打乱后第一个即随机选中），杜绝魔法数字 0..7 + 被注释分支落 else 导致 Bing 占 42.9% 的分布倾斜
/// 暂未启用：get_pixabay_image_url、get_alphacoders_image_url
/// 增删源：改 `SOURCE_COUNT` 与 `try_one_source` 的 match 分支
pub async fn download_image() -> Result<String, Box<dyn std::error::Error>> {
    let mut rng = rand::rng();
    // 随机打乱源索引顺序：第一个即随机选中的源，其后为失败时的 fallback 顺序
    let mut order: Vec<usize> = (0..SOURCE_COUNT).collect();
    order.shuffle(&mut rng);

    // 依次尝试各源，记录失败原因以便最终聚合
    let mut errors: Vec<String> = Vec::new();
    for &i in &order {
        match try_one_source(i).await {
            Ok(path) => return Ok(path),
            Err(e) => {
                eprintln!("源 {i} 失败，尝试下一个源: {e}");
                errors.push(format!("源{i}: {e}"));
            }
        }
    }
    // 所有源均失败，聚合错误信息抛出（顶层 main 会统一弹窗）
    Err(Box::<dyn std::error::Error>::from(format!(
        "所有壁纸源均不可用 | {}",
        errors.join(" ; ")
    )))
}

/// 尝试单个壁纸源的完整流程：获取图片 URL → 下载 → 保存到 `bing_wallpaper.<ext>`
///
/// 任一步失败返回带原因的 `String` 错误，供外层 `download_image` 换源重试时聚合。
///
/// `source` 壁纸源索引（`0..SOURCE_COUNT`），对应下方 match 分支
async fn try_one_source(source: usize) -> Result<String, String> {
    // 1. 获取图片的 URL 与文件名（文件名用于推断扩展名）
    let (image_url, file_name) = match source {
        0 => get_bing_image_url().await,
        1 => get_spotlight_image_url().await,
        2 => get_edge_chromium_image_url().await,
        3 => get_iciba_image_url().await,
        4 => get_nasa_image_url().await,
        _ => unreachable!(),
    }
    .map_err(|e| format!("获取URL失败: {e}"))?;

    // 2. 下载图片：统一客户端（带 UA + 超时），超时/失败会交由外层换源重试
    let client = http_client();
    let response = client
        .get(&image_url)
        .send()
        .await
        .map_err(|e| format!("下载失败: {e}"))?
        // 4xx/5xx 视为失败，交由外层换源重试，避免落盘错误页或空文件
        .error_for_status()
        .map_err(|e| format!("下载状态码异常: {e}"))?;
    let content = response
        .bytes()
        .await
        .map_err(|e| format!("读取图片字节失败: {e}"))?;
    // 空响应体：常见于被 CDN 拒绝或图片链接失效，落盘即 0kb，故判为失败触发换源
    if content.is_empty() {
        return Err(format!("图片内容为空: {image_url}"));
    }

    // 3. 保存到程序当前目录下的 bing_wallpaper.<ext>
    // 获取当前目录
    let current_dir = env::current_dir().map_err(|e| format!("获取当前目录失败: {e}"))?;
    // 获取文件的扩展名
    let ext = Path::new(&file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("jpg");
    // 构建文件的绝对路径
    let file_path = current_dir.join("bing_wallpaper.".to_owned() + ext);
    // 创建文件并写入
    let mut file = File::create(&file_path).map_err(|e| format!("创建文件失败: {e}"))?;
    file.write_all(&content)
        .map_err(|e| format!("写入文件失败: {e}"))?;

    Ok(file_path
        .to_str()
        .ok_or("文件路径转字符串失败")?
        .to_string())
}
