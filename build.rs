extern crate gcc;

fn main() {
    gcc::Config::new().file("src/stb_truetype.h").
        define("STB_TRUETYPE_IMPLEMENTATION", None).
        compile("libstb_truetype.a");

    println!("cargo:rustc-link-search={}", env!("OUT_DIR"));

}
