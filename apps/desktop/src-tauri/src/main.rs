//! Binary entry point. Keeps Windows happy (`#![windows_subsystem = "windows"]`)
//! and delegates to the library crate so logic is testable.

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

fn main() {
    code_graph_desktop_lib::run();
}
