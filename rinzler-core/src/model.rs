// #[derive(Debug, Clone)]
// pub enum NodeType {
//     RootHost,     // Initial seed URL domain
//     Endpoint,     // Same-domain URL
//     ExternalHost, // Different domain discovered
// }

// #[derive(Debug, Clone)]
// pub enum EdgeType {
//     Navigation, // Standard link
//     Reference,  // Cross-domain link
//     Redirect,   // HTTP 301/302
//     FormAction, // Form target
//     ApiCall,    // AJAX endpoint
//     Resource,   // CSS/JS/image
// }

// #[derive(Debug)]
// pub struct Node {
//     pub id: i64,
//     pub map_id: String,
//     pub url: String,
//     pub domain: String,
//     pub node_type: NodeType,
//     pub status: String,
//     pub depth: u32,
//     pub response_code: Option<u16>,
//     pub title: Option<String>,
//     // ... etc
// }
//
// #[derive(Debug)]
// pub struct Edge {
//     pub id: i64,
//     pub map_id: String,
//     pub source_node_id: i64,
//     pub target_node_id: i64,
//     pub edge_type: EdgeType,
//     pub link_text: Option<String>,
//     pub weight: f32,
// }
