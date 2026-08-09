use kittens_render::transfer::Settled;

fn clone_witness(witness: Settled<(), (), ()>) {
    let _copy: Settled<(), (), ()> = witness.clone();
}

fn main() {}
