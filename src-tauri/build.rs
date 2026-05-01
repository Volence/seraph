fn main() {
    tauri_build::build();

    cc::Build::new()
        .file("vendor/nuked-opn2/ym3438.c")
        .include("vendor/nuked-opn2")
        .opt_level(2)
        .compile("nuked_opn2");
}
