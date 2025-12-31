//! Random ASCII creature generator
//!
//! Procedurally generates cute ASCII art creatures that are different each page load.

use rand::Rng;
use serde::Serialize;

/// A randomly generated ASCII creature
#[derive(Serialize, Clone)]
pub struct Creature {
    pub art: String,
    pub name: String,
}

/// Pick a random element from a slice
fn pick<'a, T, R: Rng>(rng: &mut R, items: &'a [T]) -> &'a T {
    &items[rng.random_range(0..items.len())]
}

/// Generate a random creature
pub fn generate_creature() -> Creature {
    let mut rng = rand::rng();

    // Randomly pick creature type
    let creature_type = rng.random_range(0..8);

    match creature_type {
        0 => generate_cat(&mut rng),
        1 => generate_bunny(&mut rng),
        2 => generate_bear(&mut rng),
        3 => generate_blob(&mut rng),
        4 => generate_ghost(&mut rng),
        5 => generate_bat(&mut rng),
        6 => generate_fish(&mut rng),
        _ => generate_alien(&mut rng),
    }
}

fn generate_cat<R: Rng>(rng: &mut R) -> Creature {
    let ears = [
        ("/\\_/\\", 5),
        ("(\\__/)", 6),
        (" /\\ /\\", 6),
        ("(\\_/)", 5),
        (" ^..^", 5),
    ];
    let eyes = [
        "(o.o )", "(^.^ )", "(>.< )", "( owo )", "(-.o )", "(*w* )", "(o_o )", "( uwu )",
        "(=^.^=)", "( -.- )",
    ];
    let mouths = [" > ^ < ", "  (u)  ", "  w    ", " =\"=\"= ", "  ~    "];
    let bodies = [
        ("  /|\\  ", " (_|_) "),
        (" /|  |\\ ", "(_|  |_)"),
        ("  ||   ", " (__)  "),
        (" /(    ", "(__)\\ "),
        ("  |    ", " (==)  "),
    ];
    let tails = ["~", "~~", ")", "}>", ""];

    let (ear, ear_width) = pick(rng, &ears);
    let eye = pick(rng, &eyes);
    let mouth = pick(rng, &mouths);
    let (body1, body2) = pick(rng, &bodies);
    let tail = pick(rng, &tails);

    let pad = |s: &str, w: usize| {
        let len = s.chars().count();
        if len < w {
            format!("{}{}", " ".repeat((w - len) / 2), s)
        } else {
            s.to_string()
        }
    };

    let art = format!(
        "   {}\n   {}\n   {}\n   {}{}\n   {}",
        ear,
        pad(eye, *ear_width),
        pad(mouth, *ear_width),
        body1,
        tail,
        body2
    );

    let names = ["neko", "kitty", "meowser", "whiskers", "floof", "mochi"];
    let name = pick(rng, &names).to_string();

    Creature { art, name }
}

fn generate_bunny<R: Rng>(rng: &mut R) -> Creature {
    let ears = ["(\\(\\ ", "(\\_/)", " /| |\\", "(` ')", " |\\_/|"];
    let faces = [
        "(='.'=)", "(=^.^=)", "(=o.o=)", "(=>.<=)", "(='x'=)", "(=uwu=)",
    ];
    let bodies = [
        ("  (\")_(\")", ""),
        (" c(\")(\") ", ""),
        ("  (\")_(\") ", "~"),
        (" @(\")(\")@", ""),
    ];

    let ear = pick(rng, &ears);
    let face = pick(rng, &faces);
    let (body, tail) = pick(rng, &bodies);

    let art = format!("   {}\n   {}\n   {}{}", ear, face, body, tail);

    let names = ["bun", "hoppy", "fluffbutt", "cottontail", "thumper"];
    let name = pick(rng, &names).to_string();

    Creature { art, name }
}

fn generate_bear<R: Rng>(rng: &mut R) -> Creature {
    let faces = [
        ("ʕ•ᴥ•ʔ", ""),
        ("ʕ￫ᴥ￩ʔ", ""),
        ("ʕ·ᴥ·ʔ", ""),
        ("ʕ ᵔᴥᵔ ʔ", ""),
        ("ʕノ•ᴥ•ʔノ", " ︵"),
    ];
    let extras = ["\n  /|  |\\", "\n /(    )\\", "", "\n   || ||"];

    let (face, suffix) = pick(rng, &faces);
    let extra = pick(rng, &extras);

    let art = format!("   {}{}{}", face, suffix, extra);

    let names = ["beary", "kuma", "teddi", "honeypot", "pawbs"];
    let name = pick(rng, &names).to_string();

    Creature { art, name }
}

fn generate_blob<R: Rng>(rng: &mut R) -> Creature {
    let blobs = [
        "  ___\n (o o)\n (  > )\n  ~~~",
        " .--.\n( o_o )\n |   |\n  ~~~",
        "  .-.\n ( ' )\n  \\_/",
        " ,--.\n( ^^ )\n `--'",
        "  __\n (oo)\n /||\\\n  ~~",
        " .-.\n(o.o)\n | | \n(_|_)",
        "  _\n (o)\n<| |>\n |_|",
    ];

    let blob = pick(rng, &blobs);
    let art = blob.to_string();

    let names = ["blobby", "gloop", "slimey", "squish", "jiggly", "pudge"];
    let name = pick(rng, &names).to_string();

    Creature { art, name }
}

fn generate_ghost<R: Rng>(rng: &mut R) -> Creature {
    let ghosts = [
        "   ___\n  /o o\\\n |  >  |\n  \\www/",
        "  .---.\n / o o \\\n|   v   |\n \\~~~~~/",
        "    _\n  /' '\\\n | o o |\n |  w  |\n  \\^~^/",
        "  ___\n /. .\\\n | ~ |\n  ~~~",
        "   _\n  ( )\n  /o\\\n |~~~|",
        "  .^.\n (o o)\n  |~|\n  ~~~",
    ];

    let ghost = pick(rng, &ghosts);
    let art = ghost.to_string();

    let names = ["boo", "spooky", "phantom", "wisp", "casper", "shade"];
    let name = pick(rng, &names).to_string();

    Creature { art, name }
}

fn generate_bat<R: Rng>(rng: &mut R) -> Creature {
    let bats = [
        "  /\\   /\\\n  \\ '-' /\n   \\ o /\n    '-'",
        " /\\__/\\\n( o  o )\n > ~~ <\n  ^^^^",
        "  _   _\n /     \\\n( ^   ^ )\n \\  w  /\n  \\v-v/",
        " /\\_/\\\n(=' '=)\n / > \\ \n^^   ^^",
        "  /V\\\n (o o)\n  \\_/\n /   \\",
    ];

    let bat = pick(rng, &bats);
    let art = bat.to_string();

    let names = ["batty", "fang", "night", "squeek", "echo", "luna"];
    let name = pick(rng, &names).to_string();

    Creature { art, name }
}

fn generate_fish<R: Rng>(rng: &mut R) -> Creature {
    let fish = [
        " ><(((('>",
        " ><(((°>",
        " <')))<",
        " <><",
        " ><>",
        " ~<«««'<",
        "><(((º>",
        " ><)))°>",
    ];

    let f = pick(rng, &fish);
    let art = f.to_string();

    let names = ["bubbles", "finn", "guppy", "nemo", "splash", "gill"];
    let name = pick(rng, &names).to_string();

    Creature { art, name }
}

fn generate_alien<R: Rng>(rng: &mut R) -> Creature {
    let aliens = [
        "   .-.\n  ( o o )\n  |  O  |\n  /`---'\\",
        "  .  .\n (o  o)\n  | ~ |\n /|---|\\\n   | |",
        "   ___\n  (o o)\n /-|-|-\\\n   |_|",
        "  @   @\n (o . o)\n  \\___/\n  /| |\\",
        "  {o,o}\n  |)__)\n  -\"-\"-",
        "  (\\_/)\n  (O.O)\n  (> <)",
    ];

    let alien = pick(rng, &aliens);
    let art = alien.to_string();

    let names = ["zorp", "xeno", "blip", "quark", "ziggy", "void"];
    let name = pick(rng, &names).to_string();

    Creature { art, name }
}

/// Generate multiple unique creatures
#[allow(dead_code)]
pub fn generate_creatures(count: usize) -> Vec<Creature> {
    (0..count).map(|_| generate_creature()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_creature() {
        let creature = generate_creature();
        assert!(!creature.art.is_empty());
        assert!(!creature.name.is_empty());
    }

    #[test]
    fn test_creatures_are_random() {
        let c1 = generate_creature();
        let c2 = generate_creature();
        let c3 = generate_creature();
        assert!(!c1.art.is_empty());
        assert!(!c2.art.is_empty());
        assert!(!c3.art.is_empty());
    }
}
