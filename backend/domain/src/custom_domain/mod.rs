// Custom-domain host→realm mapping domain port
//
// This module defines the request-time query surface for the
// `custom_domain_mapping` table (design §4.3.2 / §5.1). The port is consumed
// by the host→realm middleware, the dynamic CORS predicate, the Caddy ask
// endpoint and the public resolve endpoint (BE-D04/D06/D07), and written by
// the publish/restore handlers (BE-D03).

mod entities;
pub mod ports;

pub use entities::MappingRow;
pub use ports::CustomDomainMappingRepository;
