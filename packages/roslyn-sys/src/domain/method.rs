use serde::{Deserialize, Serialize};

use super::id::MethodId;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file_path: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

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
