//! Policy-sjekker for Pho2Album. Selve sjekkene ligger i `tests/`.
//!
//! «Lokalt først» (CLAUDE.md, prinsipp 1): analysekoden skal aldri kunne
//! snakke med nettverket. Testene i `tests/no_network.rs` feiler hvis en
//! `p2a-*`-pakke får en nettverksavhengighet eller bruker `std::net`.
