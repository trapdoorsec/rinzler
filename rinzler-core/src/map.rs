// use crate::model::{NodeType, EdgeType};

// struct MapBuilder;
// impl MapBuilder {
//     pub fn process_discovered_url(
//         &mut self,
//         source_url: &str,
//         discovered_url: &str,
//         link_text: Option<String>,
//     ) -> Result<()> {
//         let source_domain = extract_domain(source_url);
//         let target_domain = extract_domain(discovered_url);
//
//         // Determine node type
//         let node_type = if self.is_root_domain(&target_domain) {
//             NodeType::Endpoint
//         } else if self.has_domain(&target_domain) {
//             NodeType::ExternalHost  // We've seen this domain before
//         } else {
//             // Brand new domain discovered
//             NodeType::ExternalHost
//         };
//
//         // Determine edge type
//         let edge_type = if source_domain == target_domain {
//             EdgeType::Navigation
//         } else {
//             EdgeType::Reference
//         };
//
//         // Insert node (or get existing)
//         let target_node_id = self.insert_or_get_node(discovered_url, node_type)?;
//         let source_node_id = self.get_node_id(source_url)?;
//
//         // Insert edge
//         self.insert_edge(source_node_id, target_node_id, edge_type, link_text)?;
//
//         Ok(())
//     }
//
//     fn insert_or_get_node(&self, p0: &str, p1: _) -> _ {
//         todo!()
//     }
//
//     fn get_node_id(&self, p0: &str) -> _ {
//         todo!()
//     }
//
//     fn has_domain(&self, p0: &_) -> bool {
//         todo!()
//     }
//
//     fn insert_edge(&self, p0: _, p1: _, p2: EdgeType, p3: Option<String>) -> _ {
//         todo!()
//     }
//
//     fn is_root_domain(&self, p0: &_) -> bool {
//         todo!()
//     }
// }
//
// fn extract_domain(p0: &str) -> _ {
//     todo!()
// }
