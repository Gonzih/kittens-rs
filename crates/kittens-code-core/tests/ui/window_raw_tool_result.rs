//! Gate G3/review-19 #14: a window tool result cannot be fabricated from
//! an uncapped raw string, regardless of that string's size.

use kittens_code_core::window::TailItem;
use kittens_code_protocol::ids::EffectId;

fn main() {
    let _oversized = TailItem::ToolResult {
        call: EffectId(1),
        text: "x".repeat(1_000_000),
    };
}
