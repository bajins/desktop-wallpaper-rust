use rand::{Rng, RngExt};
use reqwest::{header, Client, RequestBuilder, StatusCode};
use scraper::{Html, Selector};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::{env, io};
use url::Url;

/// 下载必应每日一图
pub async fn get_bing_image_url() -> anyhow::Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=1&mkt=en-US";
    // 发起网络请求
    let res = reqwest::get(api_url).await?;
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
pub async fn get_spotlight_image_url() -> anyhow::Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://arc.msn.com/v3/Delivery/Placement?pid=209567&fmt=json&cdm=1&pl=zh-CN&lc=zh-CN&ctry=CN";
    // 发起网络请求
    let res = reqwest::get(api_url).await?;
    let body = res.text().await?;
    println!("{:?}", body);
    let v: Value = serde_json::from_str(&body)?;
    let item: Value = serde_json::from_str(
        v["batchrsp"]["items"][0]["item"]
            .as_str()
            .ok_or_else(|| std::io::Error::last_os_error())?,
    )?;
    let image_url = item["ad"]["image_fullscreen_001_landscape"]["u"]
        .as_str()
        .ok_or_else(|| std::io::Error::last_os_error())?;

    println!("{:?}", item["ad"]["hs1_title_text"]["tx"]);

    Ok((image_url.to_string(), String::from("")))
}

/// 获取Edge Chromium壁纸
pub async fn get_edge_chromium_image_url() -> anyhow::Result<(String, String), Box<dyn Error>> {
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
    // 发起网络请求
    let res = reqwest::get(api_url).await?;
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
        .map(|(key, _value)| _value);
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
pub async fn get_pixabay_image_url() -> anyhow::Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://pixabay.com/api/?key=30271602-41319186b7198e7712c568e90&lang=zh&editors_choice=true";
    // 发起网络请求
    let res = reqwest::get(api_url).await?;
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
pub async fn get_iciba_image_url() -> anyhow::Result<(String, String), Box<dyn Error>> {
    // 壁纸API的URL
    let api_url = "https://open.iciba.com/dsapi";
    // 发起网络请求
    let res = reqwest::get(api_url).await?;
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
pub async fn get_alphacoders_image_url() -> anyhow::Result<(String, String), Box<dyn Error>> {
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
pub async fn get_nasa_image_url() -> anyhow::Result<(String, String), Box<dyn Error>> {
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

/// 下载壁纸图片
pub async fn download_image() -> anyhow::Result<String, Box<dyn std::error::Error>> {
    // 获取图片的URL
    let image_url;
    let file_name;
    let mut rng = rand::rng();
    let num = rng.random_range(0..7);
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
    let ext = Path::new(&file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("jpg");
    // 构建文件的绝对路径
    let file_path = current_dir.join("bing_wallpaper.".to_owned() + ext);
    // 创建文件
    let mut file = File::create(&file_path)?;
    let content = response.bytes().await?;
    file.write_all(&content)?;

    Ok(file_path
        .to_str()
        .ok_or_else(|| std::io::Error::last_os_error())?
        .to_string())
}
