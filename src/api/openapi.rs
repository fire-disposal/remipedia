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
        // Auth
        crate::api::routes::auth::login,
        crate::api::routes::auth::refresh_token,
        crate::api::routes::auth::change_password,
        crate::api::routes::auth::logout,
        // Users
        crate::api::routes::user::create_user,
        crate::api::routes::user::get_user,
        crate::api::routes::user::update_user,
        crate::api::routes::user::delete_user,
        crate::api::routes::user::list_users,
        // Patients
        crate::api::routes::patient::create_patient,
        crate::api::routes::patient::get_patient,
        crate::api::routes::patient::get_patient_detail,
        crate::api::routes::patient::update_patient,
        crate::api::routes::patient::delete_patient,
        crate::api::routes::patient::list_patients,
        // Devices
        crate::api::routes::device::register_device,
        crate::api::routes::device::get_device,
        crate::api::routes::device::update_device,
        crate::api::routes::device::delete_device,
        crate::api::routes::device::list_devices,
        // Bindings
        crate::api::routes::binding::list_bindings,
        crate::api::routes::binding::create_binding,
        crate::api::routes::binding::delete_binding,
        // Data
        crate::api::routes::data::report_data,
        crate::api::routes::data::query_data,
        // Admin - Roles & Permissions
        crate::api::routes::admin::list_roles,
        crate::api::routes::admin::get_role,
        crate::api::routes::admin::create_role,
        crate::api::routes::admin::update_role,
        crate::api::routes::admin::delete_role,
        crate::api::routes::admin::get_role_permissions,
        crate::api::routes::admin::assign_permission,
        crate::api::routes::admin::revoke_permission,
        crate::api::routes::admin::list_permissions,
        // Admin - Audit Logs
        crate::api::routes::admin::list_audit_logs,
        crate::api::routes::admin::get_audit_log,
        // Ingest
        crate::api::routes::ingest::mqtt_protocol_doc,
        crate::api::routes::ingest::query_raw_data,
    ),
    components(
        schemas(
            // Request DTOs
            crate::dto::request::LoginRequest,
            crate::dto::request::ChangePasswordRequest,
            crate::dto::request::RefreshTokenRequest,
            crate::dto::request::LogoutRequest,
            crate::dto::request::CreateUserRequest,
            crate::dto::request::UpdateUserRequest,
            crate::dto::request::UserQuery,
            crate::dto::request::CreatePatientRequest,
            crate::dto::request::UpdatePatientRequest,
            crate::dto::request::CreatePatientProfileRequest,
            crate::dto::request::PatientQuery,
            crate::dto::request::RegisterDeviceRequest,
            crate::dto::request::UpdateDeviceRequest,
            crate::dto::request::DeviceQuery,
            crate::dto::request::CreateBindingRequest,
            crate::dto::request::DataReportRequest,
            crate::dto::request::DataQuery,
            crate::dto::request::RawDataQuery,
            // Admin - Role & Permission Request DTOs
            crate::dto::response::CreateRoleRequest,
            crate::dto::response::UpdateRoleRequest,
            crate::dto::response::AssignPermissionRequest,
            crate::dto::response::AuditLogQueryParams,
            // Response DTOs
            crate::dto::response::LoginResponse,
            crate::dto::response::RefreshTokenResponse,
            crate::dto::response::UserInfo,
            crate::dto::response::UserResponse,
            crate::dto::response::UserListResponse,
            crate::dto::response::PatientResponse,
            crate::dto::response::PatientDetailResponse,
            crate::dto::response::PatientProfileResponse,
            crate::dto::response::PatientListResponse,
            crate::dto::response::DeviceResponse,
            crate::dto::response::DeviceListResponse,
            crate::dto::response::BindingInfo,
            crate::dto::response::BindingResponse,
            crate::dto::response::BindingListResponse,
            crate::dto::response::DataReportResponse,
            crate::dto::response::DataRecordResponse,
            crate::dto::response::DataQueryResponse,
            crate::dto::response::RawDataRecordResponse,
            crate::dto::response::RawDataQueryResponse,
            crate::dto::response::Pagination,
            // Admin - Role & Permission Response DTOs
            crate::dto::response::RoleResponse,
            crate::dto::response::RoleListResponse,
            crate::dto::response::PermissionResponse,
            crate::dto::response::PermissionListResponse,
            crate::dto::response::RolePermissionResponse,
            crate::dto::response::AuditLogResponse,
            crate::dto::response::AuditLogListResponse,
            crate::api::routes::ingest::MqttProtocolDoc,
            crate::api::routes::ingest::BrokerRecommendation,
        ),
    ),
    tags(
        (name = "auth", description = "认证相关接口"),
        (name = "users", description = "用户管理接口"),
        (name = "patients", description = "患者管理接口"),
        (name = "devices", description = "设备管理接口"),
        (name = "bindings", description = "绑定关系接口"),
        (name = "data", description = "数据接口"),
        (name = "admin", description = "系统管理接口（角色、权限、审计日志）"),
        (name = "ingest", description = "设备接入与协议文档"),
    ),
)]
pub struct ApiDoc;

/// 创建 Swagger UI
pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui/<_..>").url("/api-docs/openapi.json", ApiDoc::openapi())
}
