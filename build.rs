extern crate gcc;

fn main() {
    gcc::Config::new().file("src/stb_truetype.c").
        define("STB_TRUETYPE_IMPLEMENTATION", None).
        compile("libstb_truetype.a");

}
