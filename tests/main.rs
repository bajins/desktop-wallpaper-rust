// 测试宏 https://juejin.cn/post/6864453433899008008#heading-4
macro_rules! aw {
  ($e:expr) => {
      tokio_test::block_on($e)
  };
}

// https://kaisery.github.io/trpl-zh-cn/ch11-03-test-organization.html
// https://segmentfault.com/q/1010000042727693
#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::*;
    use super::*;
    use tokio_test::block_on;

    #[test]
    fn test_1() {
        // println!("1{:?}", aw!(get_alphacoders_image_url()).unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_2() {
        // println!("2{:?}", get_alphacoders_image_url().await);
    }

    #[test]
    fn test_3() {
        // println!("2{:?}", block_on(get_alphacoders_image_url()));
    }

    /*#[actix_rt::test]
    async fn test_1_async() {
        println!("1{:?}", get_alphacoders_image_url());
    }*/
}

#[test]
#[ignore]
fn test_1() {
    println!("{:?}", 1);
}