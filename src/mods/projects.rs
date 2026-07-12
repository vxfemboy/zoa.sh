//! Content for the `/projects` page — portfolio cards, rendered as responsive
//! ASCII boxes (reusing the `/about` section builder). Full voice retained.

use crate::mods::about::section;
use crate::mods::{BoxSizes, Site};

/// One bordered card per project, newest/most-notable first.
pub fn project_boxes(site: &Site) -> Vec<BoxSizes> {
    let projects: &[(&str, &str)] = &[
        (
            "FEMBOY CYBER NETWORKS",
            "an ISP that actually gives a damn. · live\n\n\
             BGP peering, fiber, upstream wrangling, customer ops. yes the name is real. yes the ASN is live. yes the routing tables are beautiful and i am very normal about them.\n\n\
             stack: BGP · fiber · linux · rust\n\
             → <a href=\"https://github.com/vxfemboy\" target=\"_blank\" rel=\"noopener\">github.com/vxfemboy</a>",
        ),
        (
            "ZOA.SH",
            "the site you're looking at right now. · rust + wasm\n\n\
             actix-web rendering responsive ascii boxes, a wasm cat that chases your cursor, a markdown blog with syntax highlighting, and a curl/ansi mode -- run `curl {domain}` and the whole site renders in your terminal. the pfp is my github avatar converted to truecolor half-block ansi, from source.\n\n\
             stack: rust · actix-web · wasm · syntect\n\
             → <a href=\"https://github.com/vxfemboy/zoa.sh\" target=\"_blank\" rel=\"noopener\">github.com/vxfemboy/zoa.sh</a>\n\
             → or just: curl {domain}",
        ),
        (
            "AI AUTOMATION CO.",
            "solo-built + sold an AI automation + marketing company. · acquired\n\n\
             reverse-engineered platform APIs, ran stable diffusion pipelines before \"generative AI\" was a buzzword anyone used, conversational agents handling inbound DMs, hands-free multi-platform posting + automated payouts. ran it all from my bed until someone made an offer on the whole thing.\n\n\
             stack: python · stable diffusion · LLMs · automation\n\
             → details under NDA. \"before it was cool.\"",
        ),
        (
            "INCENTER @ OCCAMSEC",
            "automated breach-and-attack simulation platform. · shipped\n\n\
             frontend + backend for InCenter, covering web, network, and cloud security. deployed and configured the infra on AWS with terraform + ansible.\n\n\
             stack: typescript · rust · aws · terraform · ansible",
        ),
    ];
    projects
        .iter()
        .map(|(title, body)| section(title, &site.apply(body)))
        .collect()
}
