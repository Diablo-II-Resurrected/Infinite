//! CASC 存档文件提取模块
//! 
//! 用于从 Diablo II: Resurrected 的 CASC 存档中提取游戏数据文件。

pub mod storage;

pub use storage::{CascStorage, CascError};
