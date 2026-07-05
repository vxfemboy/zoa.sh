//! Content for the `/uses` page (uses.tech style) — the tools/setup, rendered as
//! responsive ASCII boxes. Full voice retained.
//!
//! VERIFY/EDIT: editor (neovim), OS (void), and shell (zsh) are confirmed; the
//! TERMINAL / TOOLS / HARDWARE values are best-guess defaults — set them to your
//! real setup.

use crate::mods::about::section;
use crate::mods::BoxSizes;

/// One bordered card per category.
pub fn uses_boxes() -> Vec<BoxSizes> {
    let cards: &[(&str, &str)] = &[
        (
            "EDITOR",
            "neovim. a lua config i've rewritten more times than i'd like to admit -- treesitter, lsp, the usual. if it isn't modal i don't want it.",
        ),
        (
            "OS / KERNEL",
            "void linux. runit, not systemd. mainline kernels by choice, not accident -- built from source with hand-applied patches. ~18 years on linux across pretty much every device i own.",
        ),
        (
            "SHELL",
            "zsh -- a config that's mostly borrowed and partially understood. fzf wired into everything.",
        ),
        (
            "TERMINAL",
            "foot on wayland, tmux to keep the sessions alive.",
        ),
        (
            "TOOLS",
            "• ripgrep / fd / fzf for moving fast\n\
             • bat / eza because plain is boring\n\
             • git, obviously\n\
             • wireshark when packets misbehave, tcpdump when wireshark is overkill\n\
             • bgpq / bird when the routes do",
        ),
        (
            "HARDWARE",
            "whatever runs linux best. i've daily-driven more machines than i can count; the constant is they all boot to a tty first.",
        ),
        (
            "NETWORK / HOMELAB",
            "i run an ISP, so the \"homelab\" is just production. BGP, fiber, a rack of switches that's genuinely beautiful and i'm very normal about. the routing tables spark joy.",
        ),
    ];
    cards
        .iter()
        .map(|(title, body)| section(title, body))
        .collect()
}
