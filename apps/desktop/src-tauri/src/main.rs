// Ikke åpne et ekstra konsollvindu på Windows i release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    fo2album_lib::run()
}
