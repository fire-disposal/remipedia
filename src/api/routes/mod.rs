pub mod auth;
mod user;
mod patient;
mod device;
mod binding;
mod data;
pub mod health;

use rocket::Route;

pub fn routes() -> Vec<Route> {
    let mut routes = Vec::new();
    routes.extend(auth::routes());
    routes.extend(user::routes());
    routes.extend(patient::routes());
    routes.extend(device::routes());
    routes.extend(binding::routes());
    routes.extend(data::routes());
    // health routes mounted separately at root
    routes
}