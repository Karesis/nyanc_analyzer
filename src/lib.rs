pub mod db;
pub mod resolver;
pub mod ty;
#[cfg(test)]
mod tests;

pub use db::AnalyzerDb;
pub use resolver::{DefMap, Resolver};