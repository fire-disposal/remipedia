use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// OpenAPI 文档定义
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Remipedia IoT Health Platform API",
        version = "0.1.0",
        description = "IoT 健康数据平台 API 文档",
    ),
    paths(
        crate::api::routes::auth::login,
        crate::api::routes::auth::refresh_token,
        crate::api::routes::auth::me,
        crate::api::routes::device::register_device,
        crate::api::routes::device::get_device,
        crate::api::routes::device::update_device_status,
        crate::api::routes::device::delete_device,
        crate::api::routes::device::list_devices,
    ),
    components(
        schemas(
            crate::api::routes::auth::LoginRequest,
            crate::api::routes::auth::LoginResponse,
            crate::api::routes::auth::RefreshTokenRequest,
            crate::dto::request::RegisterDeviceRequest,
            crate::dto::request::UpdateDeviceRequest,
            crate::dto::request::DeviceQuery,
            crate::dto::response::DeviceResponse,
            crate::dto::response::DeviceListResponse,
            crate::dto::response::Pagination,
        ),
    ),
    tags(
        (name = "auth", description = "认证相关接口"),
        (name = "devices", description = "设备管理接口"),
    ),
)]
pub struct ApiDoc;

/// 创建 Swagger UI
pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui/<_..>").url("/api-docs/openapi.json", ApiDoc::openapi())
}
