use kittens_render::transfer::StartPermit;

fn forge_permit() {
    let mut key = ();
    let _permit = StartPermit::new(&mut key);
}

fn main() {}
