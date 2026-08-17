//! Method node in the graph.

use serde::{Deserialize, Serialize};

use super::id::MethodId;

/// Location within a source file. Lines and columns are 1-based to match
/// common editor conventions; the frontend may convert as needed.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file_path: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// A method discovered by an analyzer.
///
/// `id` is the stable, language-independent identifier.
/// `fully_qualified_name` is the human-readable, language-native FQN
/// (e.g. `MyApp.Services.OrderService.GetOrder(int)`).
/// `display_name` is a short label suitable for compact UI
/// (e.g. `OrderService.GetOrder()`).
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MethodNode {
    pub id: MethodId,
    pub name: String,
    pub fully_qualified_name: String,
    pub display_name: String,
    pub containing_type: String,
    pub file_path: String,
    pub location: SourceLocation,
}
