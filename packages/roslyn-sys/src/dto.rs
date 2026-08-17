//! Wire-format DTOs and conversion to domain types.
//!
//! The managed helper serializes to these shapes; the Rust analyzer
//! converts them into [`crate::domain`] values. Keeping the wire format
//! in its own module prevents drift between the helper and consumers.

use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::domain::{
    CallRelationship, MethodGraph, MethodId, MethodNode, RelationshipKind, SourceLocation,
};

/// Top-level document emitted by the managed helper.
#[derive(Debug, Deserialize)]
pub struct AnalysisGraph {
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub methods: Vec<MethodNodeDto>,
    #[serde(default)]
    pub edges: Vec<CallEdgeDto>,
}

#[derive(Debug, Deserialize)]
pub struct SourceLocationDto {
    pub file_path: String,
    #[serde(default)] pub start_line: u32,
    #[serde(default)] pub start_column: u32,
    #[serde(default)] pub end_line: u32,
    #[serde(default)] pub end_column: u32,
}

impl From<SourceLocationDto> for SourceLocation {
    fn from(v: SourceLocationDto) -> Self {
        Self {
            file_path: v.file_path,
            start_line: v.start_line,
            start_column: v.start_column,
            end_line: v.end_line,
            end_column: v.end_column,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MethodNodeDto {
    pub id: String,
    pub name: String,
    pub fully_qualified_name: String,
    pub display_name: String,
    pub containing_type: String,
    pub file_path: String,
    pub location: SourceLocationDto,
}

impl From<MethodNodeDto> for MethodNode {
    fn from(v: MethodNodeDto) -> Self {
        Self {
            id: MethodId::new(v.id),
            name: v.name,
            fully_qualified_name: v.fully_qualified_name,
            display_name: v.display_name,
            containing_type: v.containing_type,
            file_path: v.file_path,
            location: v.location.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CallEdgeDto {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub kind: String,
}

impl From<CallEdgeDto> for CallRelationship {
    fn from(v: CallEdgeDto) -> Self {
        let kind = match v.kind.as_str() {
            "calls" => RelationshipKind::Calls,
            _ => RelationshipKind::Calls,
        };
        Self {
            source: MethodId::new(v.source),
            target: MethodId::new(v.target),
            kind,
        }
    }
}

impl AnalysisGraph {
    /// Convert the DTO graph into a domain [`MethodGraph`]. Any
    /// `error` field becomes an `Err`.
    pub fn into_domain(self) -> Result<MethodGraph> {
        if let Some(err) = self.error {
            return Err(anyhow!("managed helper reported error: {}", err));
        }
        let mut graph = MethodGraph::new();
        for m in self.methods {
            graph.add_method(m.into());
        }
        for e in self.edges {
            graph.add_edge(e.into());
        }
        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Bridge;

    const FIXTURE: &str = r#"
    {
      "methods": [
        {
          "id": "csharp:OrderSystem.Services.OrderService.GetOrder(int)",
          "name": "GetOrder",
          "fully_qualified_name": "OrderSystem.Services.OrderService.GetOrder(int)",
          "display_name": "OrderService.GetOrder(int)",
          "containing_type": "OrderSystem.Services.OrderService",
          "file_path": "/repo/OrderService.cs",
          "location": {
            "file_path": "/repo/OrderService.cs",
            "start_line": 11,
            "start_column": 5,
            "end_line": 11,
            "end_column": 55
          }
        },
        {
          "id": "csharp:OrderSystem.Services.OrderRepository.FindById(int)",
          "name": "FindById",
          "fully_qualified_name": "OrderSystem.Services.OrderRepository.FindById(int)",
          "display_name": "OrderRepository.FindById(int)",
          "containing_type": "OrderSystem.Services.OrderRepository",
          "file_path": "/repo/OrderRepository.cs",
          "location": {
            "file_path": "/repo/OrderRepository.cs",
            "start_line": 16,
            "start_column": 5,
            "end_line": 16,
            "end_column": 70
          }
        }
      ],
      "edges": [
        {
          "source": "csharp:OrderSystem.Services.OrderService.GetOrder(int)",
          "target": "csharp:OrderSystem.Services.OrderRepository.FindById(int)",
          "kind": "calls"
        }
      ]
    }
    "#;

    #[test]
    fn parses_helper_json_into_domain_graph() {
        let bridge = Bridge::from_json(FIXTURE).expect("fixture bridge");
        let graph = bridge
            .parse_graph(FIXTURE)
            .expect("valid fixture")
            .into_domain()
            .expect("valid domain graph");

        assert_eq!(graph.methods.len(), 2);
        assert_eq!(graph.edges.len(), 1);

        let get_order = graph
            .methods
            .get(&MethodId::new("csharp:OrderSystem.Services.OrderService.GetOrder(int)"))
            .expect("method present");
        assert_eq!(get_order.display_name, "OrderService.GetOrder(int)");
        assert_eq!(get_order.location.start_line, 11);

        let callees = graph.callees_of(&MethodId::new(
            "csharp:OrderSystem.Services.OrderService.GetOrder(int)",
        ));
        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].name, "FindById");
    }

    #[test]
    fn surfaces_helper_error() {
        let doc = r#"{"error":"boom"}"#;
        let bridge = Bridge::from_json(doc).expect("fixture bridge");
        let err = bridge
            .parse_graph(doc)
            .expect("doc parses")
            .into_domain()
            .expect_err("error text becomes Err");
        assert!(err.to_string().contains("boom"));
    }
}
