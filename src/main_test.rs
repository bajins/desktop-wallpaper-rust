#[cfg(test)]
mod tests {
    use crate::get_edge_chromium_image_url;
    use super::*;

    #[test]
    fn test_1() {
        println!("{:?}", get_edge_chromium_image_url());
    }
}