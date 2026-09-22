use taichiwayfarer::data::get_slow_set_names;

#[expect(clippy::print_stdout)]
pub fn main() {
    let names = get_slow_set_names();
    for (i, (time, characters, pinyin, name)) in names.iter().enumerate() {
        println!("{time} {}. {characters} - {pinyin} - {name}", i + 1);
    }
}
