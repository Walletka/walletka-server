use rand::Rng;

pub fn generate_random_name() -> String {
    let syllables = [
        "ko", "mi", "yu", "ta", "sa", "na", "shi", "ka", "to", "mo", "fu", "hi", "ma", "ku", "re",
        "no", "do", "chi", "ro", "me", "ri", "ra", "sen", "gan", "ga",
    ];
    let mut rng = rand::thread_rng();
    let num_syllables = rng.gen_range(2..=4); // Generate a name with 2 to 4 syllables
    let mut name = String::new();

    for _ in 0..num_syllables {
        let index = rng.gen_range(0..syllables.len());
        name.push_str(syllables[index]);
    }

    name
}
