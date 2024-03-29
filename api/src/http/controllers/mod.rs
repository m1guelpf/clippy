pub mod auth;
pub mod chatgpt;
pub mod project;
pub mod team;
pub mod user;
pub mod widget;

pub use auth as AuthController;
pub use chatgpt as ChatGPTController;
pub use project as ProjectController;
pub use team as TeamController;
pub use user as UserController;
pub use widget as WidgetController;
