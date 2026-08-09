use kittens_render::transfer::StartPermit;

fn clone_permit(permit: StartPermit<'_>) {
    let _copy = permit.clone();
}

fn main() {}
