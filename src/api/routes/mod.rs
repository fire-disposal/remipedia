pub mod auth;
pub mod binding;
pub mod data;
pub mod device;
pub mod health;
pub mod patient;
pub mod user;

use rocket::{Route, routes, options};

/// 处理所有 OPTIONS 预检请求
#[options("/<_..>")]
pub fn options_preflight() -> &'static str {
    ""
}

pub fn routes() -> Vec<Route> {
    let mut all_routes = Vec::new();
    all_routes.extend(auth::routes());
    all_routes.extend(user::routes());
    all_routes.extend(patient::routes());
    all_routes.extend(device::routes());
    all_routes.extend(binding::routes());
    all_routes.extend(data::routes());
    // 添加 OPTIONS 预检路由
    all_routes.extend(routes![options_preflight]);
    // health routes mounted separately at root
    all_routes
}
