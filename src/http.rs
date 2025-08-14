use anyhow::Result;
use log::{debug, warn};
use reqwest::{Client, header::{HeaderMap, HeaderName, HeaderValue}};
use std::time::Duration;

/// 构建HTTP客户端，包含自定义请求头
pub fn build_http_client(custom_headers: &[String]) -> Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "User-Agent", 
        HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
    );

    for header in custom_headers {
        if let Some((key, value)) = header.split_once(':') {
            let header_name = HeaderName::from_bytes(key.trim().as_bytes())?;
            let header_value = HeaderValue::from_str(value.trim())?;
            headers.insert(header_name, header_value);
        } else {
            warn!("Ignoring malformed header: {}", header);
        }
    }
    
    debug!("Using HTTP headers: {:?}", headers);

    let client = Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(30))
        .build()?;
    
    Ok(client)
}