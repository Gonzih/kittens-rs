use kittens_render::geometry::Region;
use kittens_render::transfer::FlightStarter;

fn start_without_target<F: FlightStarter>(starter: F, region: Region) {
    let _ = starter.start(region);
}

fn main() {}
