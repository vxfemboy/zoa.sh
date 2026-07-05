//! Content for the `/now` page (nownownow.com style) — what Zoa is focused on
//! right now, rendered as responsive ASCII boxes. Update the date + content when
//! things change.

use crate::mods::about::section;
use crate::mods::BoxSizes;

/// Bump this when you update the page (nownownow convention shows a date).
const UPDATED: &str = "june 2026";

pub fn now_boxes() -> Vec<BoxSizes> {
    let intro = format!(
        "what i'm actually doing right now. this is a /now page (see nownownow.com) -- it changes as i do.\n\nupdated: {UPDATED}"
    );

    let cards: Vec<(&str, String)> = vec![
        ("NOW", intro),
        (
            "BUILDING",
            "Femboy Cyber Networks -- the ISP. BGP peering, fiber, customer ops, upstream wrangling. the ASN is live and i think about routing in the shower.\n\nzoa.sh -- this site. rust, wasm, a curl/ansi mode, an auto-updating ansi pfp. always tinkering.".to_string(),
        ),
        (
            "RESEARCH",
            "applied AI on the side -- LLM fine-tuning, eval harnesses, training-observability infra. research is just breaking things on purpose and writing down what fell out.".to_string(),
        ),
        (
            "AVAILABLE",
            "open to interesting problems -- low-level, networking, kernels, AI infra, security. if it's weird and hard, even better.\n\n→ <a href=\"mailto:zoa@zoa.sh\">zoa@zoa.sh</a>".to_string(),
        ),
    ];
    cards
        .iter()
        .map(|(title, body)| section(title, body))
        .collect()
}
