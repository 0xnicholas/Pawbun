//! 内置工具集合。
//!
//! 当前提供文件读写、目录列表工具；网络工具在启用 `http` feature 时可用；
//! JSON 查询和 CSV 查询在启用对应 feature 时可用；
//! Vision、Embedding、CodeExecute 为接口占位，需外部集成。

pub mod code_execute;
pub mod directory_list;
pub mod file_read;
pub mod file_write;
pub mod vision;

#[cfg(feature = "csv")]
pub mod csv_query;
#[cfg(feature = "jsonpath")]
pub mod json_query;
#[cfg(feature = "http")]
pub mod web_fetch;
#[cfg(feature = "http")]
pub mod web_search;
#[cfg(feature = "http")]
pub(crate) mod url_utils;

pub(crate) mod path_utils;

pub use code_execute::CodeExecuteTool;
pub use directory_list::DirectoryListTool;
pub use file_read::FileReadTool;
pub use file_write::FileWriteTool;
pub use vision::VisionTool;

#[cfg(feature = "csv")]
pub use csv_query::CsvQueryTool;
#[cfg(feature = "jsonpath")]
pub use json_query::JsonQueryTool;
#[cfg(feature = "http")]
pub use web_fetch::WebFetchTool;
#[cfg(feature = "http")]
pub use web_search::WebSearchTool;
