pub mod device;
pub mod health;

use rocket::{options, routes, Route};

/// 处理所有 OPTIONS 预检请求
#[options("/<_..>")]
pub fn options_preflight() -> &'static str {
    ""
}

pub fn routes() -> Vec<Route> {
    let mut all_routes = Vec::new();
    all_routes.extend(device::routes());
    all_routes.extend(routes![options_preflight]);
    all_routes
}
