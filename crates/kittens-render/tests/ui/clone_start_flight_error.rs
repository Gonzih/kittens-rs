use kittens_render::transfer::StartFlightError;

fn clone_error(error: StartFlightError<(), ()>) {
    let _copy: StartFlightError<(), ()> = error.clone();
}

fn main() {}
