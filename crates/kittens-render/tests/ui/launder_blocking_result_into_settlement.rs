use kittens_render::sweep::StripeSettlement;

fn launder(result: Result<(), ()>) -> StripeSettlement {
    result.into()
}

fn main() {}
