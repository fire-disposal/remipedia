//! Rocket Fairings — 请求日志、CORS 等中间件
//!
//! 将原本位于 `main.rs` 中的 Fairing 定义移入库 crate，
//! 便于统一管理和测试。

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::request::Request;
use rocket::Response;
use std::time::Instant;

/// CORS Fairing — 为所有响应添加跨域头
pub struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "CORS",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(
        &self,
        request: &'r Request<'_>,
        response: &mut Response<'r>,
    ) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE, PATCH, OPTIONS, HEAD",
        ));

        if let Some(request_headers) =
            request.headers().get_one("Access-Control-Request-Headers")
        {
            response
                .set_header(Header::new("Access-Control-Allow-Headers", request_headers));
        } else {
            response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        }

        response.set_header(Header::new("Access-Control-Max-Age", "86400"));
        response.set_header(Header::new("Access-Control-Expose-Headers", "*"));
    }
}

/// 请求日志 Fairing — 自动记录每个请求的方法、路径、状态码和耗时。
pub struct RequestLogger;

#[rocket::async_trait]
impl Fairing for RequestLogger {
    fn info(&self) -> Info {
        Info {
            name: "RequestLogger",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut rocket::Data<'_>) {
        request.local_cache(|| Instant::now());
    }

    async fn on_response<'r>(
        &self,
        request: &'r Request<'_>,
        response: &mut Response<'r>,
    ) {
        let duration = request.local_cache(|| Instant::now()).elapsed();

        let method = request.method();
        let path = request.uri().path();
        let status = response.status();
        let user_agent = request
            .headers()
            .get_one("User-Agent")
            .unwrap_or("-");

        log::info!(
            "{} {} {} ({:?}) UA={}",
            method,
            path,
            status,
            duration,
            user_agent,
        );
    }
}
