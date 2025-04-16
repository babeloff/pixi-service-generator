
pub const SYSTEM_EXEC_NAME: &str = "systemd-pixi-system-generator";
pub const USER_EXEC_NAME: &str = "systemd-pixi-user-generator";
pub const SYSTEM_UNIT_FILE_TEMPLATE: &str = "src/resources/system.unit.service.tera";
pub const USER_UNIT_FILE_TEMPLATE: &str = "src/resources/user.unit.service.tera";

pub enum Privilege {
    SYSTEM,
    USER,
    UNSPEC,
}
